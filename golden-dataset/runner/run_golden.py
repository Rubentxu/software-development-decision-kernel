#!/usr/bin/env python3
"""Run isolated SDDK prompt evaluations and grade them deterministically."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import resource
import secrets
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import yaml

from grade_results import (
    CLASSIFICATIONS,
    SEVERITIES,
    VERDICTS,
    grade_run,
    validate_role_output,
)


DATASET_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = DATASET_DIR.parent
CASE_REQUIRED = ("spec.md", "expected-verdict.yaml", "implementation")
SHA256 = re.compile(r"^[a-f0-9]{64}$")
SANDBOX_ROLE = Path("/work/role")
SANDBOX_IMPLEMENTATION = Path("/work/case/implementation")
SANDBOX_BUNDLE = Path("/work/bundle")
SYSTEM_PATHS = (Path("/usr"), Path("/bin"), Path("/lib"), Path("/lib64"))
SYSTEM_CONFIG_PATHS = (
    Path("/etc/alternatives"),
    Path("/etc/ca-certificates"),
    Path("/etc/ssl"),
    Path("/etc/hosts"),
    Path("/etc/resolv.conf"),
    Path("/etc/nsswitch.conf"),
    Path("/etc/gai.conf"),
    Path("/etc/ld.so.cache"),
)


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def digest_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def digest_tree(paths: list[Path], base: Path = REPO_ROOT) -> str:
    digest = hashlib.sha256()
    files: list[Path] = []
    for path in paths:
        if path.is_file():
            files.append(path)
        elif path.is_dir():
            files.extend(
                item
                for item in path.rglob("*")
                if item.is_file()
                and "__pycache__" not in item.parts
                and item.suffix != ".pyc"
            )
    for path in sorted(files):
        digest.update(str(path.relative_to(base)).encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def implementation_manifest(case_dir: Path) -> list[dict[str, Any]]:
    root = case_dir / "implementation"
    return [
        {
            "path": str(path.relative_to(case_dir)),
            "sha256": digest_file(path),
            "bytes": path.stat().st_size,
        }
        for path in sorted(root.rglob("*"))
        if path.is_file()
    ]


def load_case(case_dir: Path) -> dict[str, Any]:
    missing = [name for name in CASE_REQUIRED if not (case_dir / name).exists()]
    if missing:
        raise ValueError(f"{case_dir.name}: missing {', '.join(missing)}")
    expected = yaml.safe_load((case_dir / "expected-verdict.yaml").read_text())
    if not isinstance(expected, dict):
        raise ValueError(f"{case_dir.name}: expected-verdict.yaml must be an object")
    if expected.get("case") != case_dir.name:
        raise ValueError(f"{case_dir.name}: case field does not match directory")
    required = {
        "case",
        "schema_version",
        "suite",
        "target_phase",
        "language",
        "path",
        "held_out",
        "trials",
        "expected",
    }
    missing_fields = required - expected.keys()
    if missing_fields:
        raise ValueError(f"{case_dir.name}: missing fields {sorted(missing_fields)}")
    if expected["schema_version"] != "golden-case/v1" or expected["held_out"] is not True:
        raise ValueError(f"{case_dir.name}: invalid schema_version or held_out marker")
    if expected["target_phase"] not in {"verify", "debt"}:
        raise ValueError(f"{case_dir.name}: invalid target_phase")
    if expected["path"] not in {"B-direct", "A-min", "A-lite", "A-full"}:
        raise ValueError(f"{case_dir.name}: invalid path")
    if not isinstance(expected["trials"], int) or expected["trials"] < 1:
        raise ValueError(f"{case_dir.name}: trials must be at least 1")
    expected_result = expected["expected"]
    if not isinstance(expected_result, dict):
        raise ValueError(f"{case_dir.name}: expected must be an object")
    if expected_result.get("verify") not in VERDICTS or expected_result.get("debt") not in VERDICTS:
        raise ValueError(f"{case_dir.name}: invalid expected verdict")
    labels = expected_result.get("labels", [])
    if not isinstance(labels, list):
        raise ValueError(f"{case_dir.name}: labels must be an array")
    seen_labels: set[tuple[str, str]] = set()
    for label in labels:
        if not isinstance(label, dict) or set(label) != {
            "rule_id",
            "classification",
            "severity",
            "location",
        }:
            raise ValueError(f"{case_dir.name}: invalid label shape")
        label_key = (label["rule_id"], label["location"])
        if label_key in seen_labels:
            raise ValueError(f"{case_dir.name}: duplicate label rule_id/location")
        seen_labels.add(label_key)
        if label["classification"] not in CLASSIFICATIONS or label["severity"] not in SEVERITIES:
            raise ValueError(f"{case_dir.name}: invalid label classification or severity")
        if not isinstance(label["location"], str) or not label["location"]:
            raise ValueError(f"{case_dir.name}: invalid label location")
    return expected


def validate_run_manifest(manifest: Any) -> dict[str, Any]:
    if not isinstance(manifest, dict):
        raise ValueError("run manifest must be an object")
    required = {
        "contract_version",
        "bundle_hash",
        "evaluation_hash",
        "labels_snapshot_digest",
        "created_at",
        "evaluator",
        "judge",
        "command_digests",
        "execution_policy",
        "cases",
    }
    if set(manifest) != required:
        raise ValueError("run manifest fields do not match evaluation.schema.json")
    if manifest["contract_version"] != "golden-run/v1":
        raise ValueError("run manifest has an invalid contract_version")
    for field in ("bundle_hash", "evaluation_hash", "labels_snapshot_digest"):
        if not isinstance(manifest[field], str) or not SHA256.fullmatch(manifest[field]):
            raise ValueError(f"run manifest has an invalid {field}")
    try:
        datetime.fromisoformat(manifest["created_at"])
    except (TypeError, ValueError) as error:
        raise ValueError("run manifest has an invalid created_at") from error
    for role_name in ("evaluator", "judge"):
        role = manifest[role_name]
        if not isinstance(role, dict) or set(role) != {"identity", "model"}:
            raise ValueError(f"run manifest has an invalid {role_name}")
        if not all(isinstance(value, str) and value for value in role.values()):
            raise ValueError(f"run manifest has an empty {role_name} field")
    command_digests = manifest["command_digests"]
    if not isinstance(command_digests, dict) or set(command_digests) != {"evaluator", "judge"}:
        raise ValueError("run manifest has invalid command_digests")
    if not all(isinstance(value, str) and SHA256.fullmatch(value) for value in command_digests.values()):
        raise ValueError("run manifest has a malformed command digest")
    policy = manifest["execution_policy"]
    if not isinstance(policy, dict) or set(policy) != {
        "network",
        "passed_environment",
        "read_only_paths",
        "timeout_seconds",
        "max_output_bytes",
    }:
        raise ValueError("run manifest has an invalid execution_policy")
    if policy["network"] not in {"disabled", "external-model"}:
        raise ValueError("run manifest has an invalid network policy")
    if not isinstance(policy["passed_environment"], list) or len(policy["passed_environment"]) != len(set(policy["passed_environment"])):
        raise ValueError("run manifest has invalid passed_environment")
    if not all(isinstance(value, str) and value for value in policy["passed_environment"]):
        raise ValueError("run manifest has invalid passed_environment")
    if not isinstance(policy["read_only_paths"], list) or len(policy["read_only_paths"]) != len(set(policy["read_only_paths"])):
        raise ValueError("run manifest has invalid read_only_paths")
    if not all(isinstance(value, str) and Path(value).is_absolute() for value in policy["read_only_paths"]):
        raise ValueError("run manifest read_only_paths must be absolute")
    for field in ("timeout_seconds", "max_output_bytes"):
        if not isinstance(policy[field], int) or policy[field] < 1:
            raise ValueError(f"run manifest has an invalid {field}")
    cases = manifest["cases"]
    if not isinstance(cases, list) or not cases or len(cases) != len(set(cases)):
        raise ValueError("run manifest cases must be a non-empty unique array")
    if not all(isinstance(case, str) and case for case in cases):
        raise ValueError("run manifest has an invalid case ID")
    return manifest


def role_input(
    case_dir: Path,
    case_meta: dict[str, Any],
    bundle_hash: str,
    trial: int,
    role: str,
    prior_output: dict[str, Any] | None = None,
    bundle_root: Path | None = None,
) -> dict[str, Any]:
    value: dict[str, Any] = {
        "contract_version": "golden-role-input/v1",
        "role": role,
        "case_id": case_meta["case"],
        "suite": case_meta["suite"],
        "target_phase": case_meta["target_phase"],
        "language": case_meta["language"],
        "path": case_meta["path"],
        "trial": trial,
        "bundle_hash": bundle_hash,
        "bundle_root": str((bundle_root or REPO_ROOT).resolve()),
        "task": (case_dir / "spec.md").read_text(),
        "implementation_root": str((case_dir / "implementation").resolve()),
        "location_base": "case; use the implementation/... paths from implementation_manifest",
        "implementation_manifest": implementation_manifest(case_dir),
        "required_output": {
            "verdict": "PASS | PASS_WITH_WARNINGS | FAIL | INCONCLUSIVE",
            "findings": [
                {
                    "finding_id": "SHA-256 of canonical rule_id + subject + location",
                    "rule_id": "stable.rule-id",
                    "classification": "blocking_defect | warning | suggestion | false_positive | insufficient_evidence",
                    "severity": "critical | high | medium | low",
                    "subject": {"base": None, "head": None, "diff_digest": None},
                    "location": {"path": "relative path", "start_line": 1, "end_line": 1},
                    "confidence": "high | medium | low",
                    "production_reachable": "yes | no | unknown",
                    "evidence": [
                        {
                            "kind": "source | command | test | trace | artifact",
                            "observation": "observable fact",
                            "output_digest": None,
                        }
                    ],
                    "exemption": None,
                    "owner_phase": "apply | verify | debt-verify | replan | human",
                }
            ],
        },
    }
    if prior_output is not None:
        value["candidate_output"] = prior_output
    return value


def run_role(
    command_template: str,
    identity: str,
    model: str,
    input_value: dict[str, Any],
    role_dir: Path,
    pass_env: tuple[str, ...] = (),
    timeout_seconds: int = 900,
    max_output_bytes: int = 1_048_576,
    network_policy: str = "disabled",
    read_only_paths: tuple[Path, ...] = (),
) -> dict[str, Any]:
    role_dir.mkdir(parents=True, exist_ok=False)
    input_path = role_dir / "input.json"
    output_path = role_dir / "output.json"
    trace_path = role_dir / "tool-trace.jsonl"
    provenance_path = role_dir / "provenance.json"
    sandbox_input = json.loads(json.dumps(input_value))
    sandbox_input["implementation_root"] = str(SANDBOX_IMPLEMENTATION)
    sandbox_input["bundle_root"] = str(SANDBOX_BUNDLE)
    input_path.write_text(json.dumps(sandbox_input, indent=2, sort_keys=True) + "\n")

    substitutions = {
        "input": str(SANDBOX_ROLE / "input.json"),
        "output": str(SANDBOX_ROLE / "output.json"),
        "trace": str(SANDBOX_ROLE / "tool-trace.jsonl"),
        "provenance": str(SANDBOX_ROLE / "provenance.json"),
        "case_dir": str(SANDBOX_IMPLEMENTATION),
        "bundle": str(SANDBOX_BUNDLE),
    }
    rendered = command_template
    for name, value in substitutions.items():
        rendered = rendered.replace("{" + name + "}", shlex.quote(value))
    argv = shlex.split(rendered)
    if network_policy not in {"disabled", "external-model"}:
        raise RuntimeError(f"unsupported network policy: {network_policy}")
    bwrap = shutil.which("bwrap")
    if bwrap is None:
        raise RuntimeError("golden role isolation requires bwrap")
    sandbox_argv = [
        bwrap,
        "--die-with-parent",
        "--new-session",
        "--tmpfs", "/tmp",
        "--tmpfs", "/home",
        "--dir", "/work",
        "--dir", "/work/case",
        "--dir", "/etc",
        "--bind", str(role_dir), str(SANDBOX_ROLE),
        "--ro-bind", str(input_path), str(SANDBOX_ROLE / "input.json"),
        "--ro-bind", input_value["implementation_root"], str(SANDBOX_IMPLEMENTATION),
        "--ro-bind", input_value["bundle_root"], str(SANDBOX_BUNDLE),
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--chdir",
        str(SANDBOX_IMPLEMENTATION),
    ]
    for path in (*SYSTEM_PATHS, *SYSTEM_CONFIG_PATHS):
        if path.exists():
            sandbox_argv.extend(("--ro-bind", str(path), str(path)))

    def add_private_bind(path: Path) -> None:
        resolved = path.resolve()
        protected_roots = (REPO_ROOT, role_dir.resolve())
        if any(
            resolved.is_relative_to(protected) or protected.is_relative_to(resolved)
            for protected in protected_roots
        ):
            raise RuntimeError(f"refusing to expose protected evaluation path: {resolved}")
        existing_roots = {Path("/tmp"), Path("/home"), Path("/work"), Path("/etc"), *SYSTEM_PATHS}
        parents = [
            parent
            for parent in resolved.parents
            if parent != Path("/") and parent not in existing_roots
        ]
        for parent in reversed(parents):
            sandbox_argv.extend(("--dir", str(parent)))
        sandbox_argv.extend(("--ro-bind", str(resolved), str(resolved)))

    resolved_executable = shutil.which(argv[0])
    if resolved_executable is None:
        raise RuntimeError(f"role executable not found: {argv[0]}")
    argv[0] = resolved_executable
    explicit_binds: set[Path] = set()
    for argument in argv:
        candidate = Path(argument)
        if candidate.is_absolute() and candidate.is_file() and not any(
            candidate.is_relative_to(system_path) for system_path in SYSTEM_PATHS
        ):
            explicit_binds.add(candidate)
    explicit_binds.update(path.resolve() for path in read_only_paths)
    for path in sorted(explicit_binds):
        if not path.exists():
            raise RuntimeError(f"read-only role path does not exist: {path}")
        add_private_bind(path)
    if network_policy == "disabled":
        sandbox_argv.append("--unshare-net")
    argv = [*sandbox_argv, "--", *argv]
    started = datetime.now(timezone.utc).isoformat()
    environment = {
        name: os.environ[name]
        for name in ("PATH", "LANG", "LC_ALL", "LC_CTYPE", "TZ", *pass_env)
        if name in os.environ
    }
    environment.update(
        {
            "PWD": str(SANDBOX_IMPLEMENTATION),
            "SDDK_EVAL_INPUT": str(SANDBOX_ROLE / "input.json"),
            "SDDK_EVAL_OUTPUT": str(SANDBOX_ROLE / "output.json"),
            "SDDK_EVAL_TRACE": str(SANDBOX_ROLE / "tool-trace.jsonl"),
            "SDDK_EVAL_PROVENANCE": str(SANDBOX_ROLE / "provenance.json"),
            "SDDK_EVAL_IDENTITY": identity,
            "SDDK_EVAL_MODEL": model,
            "SDDK_EVAL_BUNDLE": str(SANDBOX_BUNDLE),
        }
    )
    stdout_path = role_dir / "stdout.txt"
    stderr_path = role_dir / "stderr.txt"
    timed_out = False
    with stdout_path.open("wb") as stdout_file, stderr_path.open("wb") as stderr_file:
        process = subprocess.Popen(
            argv,
            cwd=input_value["implementation_root"],
            env=environment,
            stdout=stdout_file,
            stderr=stderr_file,
            start_new_session=True,
            preexec_fn=lambda: (
                resource.setrlimit(
                    resource.RLIMIT_FSIZE,
                    (max_output_bytes + 1, max_output_bytes + 1),
                ),
                resource.setrlimit(resource.RLIMIT_CORE, (0, 0)),
            ),
        )
        try:
            process.wait(timeout=timeout_seconds)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()

    bounded_paths = (stdout_path, stderr_path, output_path, trace_path, provenance_path)
    oversized = [
        path.name
        for path in bounded_paths
        if path.exists() and path.stat().st_size > max_output_bytes
    ]
    stdout_bytes = stdout_path.read_bytes() if "stdout.txt" not in oversized else b""
    stderr_bytes = stderr_path.read_bytes() if "stderr.txt" not in oversized else b""
    if not output_path.exists() and stdout_bytes.strip():
        output_path.write_bytes(stdout_bytes)

    try:
        if "provenance.json" in oversized:
            raise ValueError("provenance exceeds output limit")
        provenance = json.loads(provenance_path.read_text())
    except (OSError, ValueError) as error:
        provenance = None
        provenance_error = str(error)
    else:
        provenance_error = None
    execution = {
        "identity": identity,
        "model": model,
        "argv": argv,
        "started_at": started,
        "finished_at": datetime.now(timezone.utc).isoformat(),
        "exit_code": process.returncode,
        "timed_out": timed_out,
        "timeout_seconds": timeout_seconds,
        "network_policy": network_policy,
        "input_digest": digest_file(input_path),
        "stdout_digest": digest_file(stdout_path),
        "stderr_digest": digest_file(stderr_path),
        "trace_digest": digest_file(trace_path) if trace_path.exists() else None,
        "provenance_digest": digest_file(provenance_path) if provenance_path.exists() else None,
    }
    (role_dir / "execution.json").write_text(
        json.dumps(execution, indent=2, sort_keys=True) + "\n"
    )
    if oversized:
        raise RuntimeError(
            f"{identity} exceeded the {max_output_bytes}-byte limit in {sorted(oversized)}"
        )
    if timed_out:
        raise RuntimeError(f"{identity} timed out after {timeout_seconds} seconds; see {role_dir}")
    if process.returncode == -signal.SIGXFSZ:
        raise RuntimeError(f"{identity} exceeded the {max_output_bytes}-byte file limit")
    if process.returncode != 0:
        raise RuntimeError(f"{identity} exited {process.returncode}; see {role_dir}")
    if provenance_error is not None:
        raise RuntimeError(f"{identity} did not produce valid provenance JSON: {provenance_error}")
    if not isinstance(provenance, dict):
        raise RuntimeError(f"{identity} provenance must be a JSON object")
    required_provenance = {"identity", "model", "provider", "invocation_id"}
    if set(provenance) != required_provenance:
        raise RuntimeError(f"{identity} provenance fields must be {sorted(required_provenance)}")
    if provenance["identity"] != identity or provenance["model"] != model:
        raise RuntimeError(f"{identity} provenance does not match requested identity/model")
    if not all(isinstance(provenance[key], str) and provenance[key] for key in required_provenance):
        raise RuntimeError(f"{identity} provenance values must be non-empty strings")
    try:
        output = json.loads(output_path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"{identity} did not produce valid JSON: {error}") from error
    try:
        return validate_role_output(output, f"{identity} output")
    except ValueError as error:
        raise RuntimeError(str(error)) from error


def isolate_case(case_dir: Path) -> tuple[tempfile.TemporaryDirectory[str], Path]:
    temporary = tempfile.TemporaryDirectory(prefix=f"sddk-golden-{case_dir.name}-")
    isolated = Path(temporary.name) / "case"
    isolated.mkdir()
    shutil.copy2(case_dir / "spec.md", isolated / "spec.md")
    shutil.copytree(case_dir / "implementation", isolated / "implementation")
    for path in (isolated / "implementation").rglob("*"):
        path.chmod(0o555 if path.is_dir() else 0o444)
    (isolated / "implementation").chmod(0o555)
    return temporary, isolated


def isolate_bundle() -> tuple[tempfile.TemporaryDirectory[str], Path]:
    temporary = tempfile.TemporaryDirectory(prefix="sddk-golden-bundle-")
    isolated = Path(temporary.name) / "bundle"
    isolated.mkdir()
    for name in ("agents", "skills", "prompts"):
        shutil.copytree(REPO_ROOT / name, isolated / name)
    for path in isolated.rglob("*"):
        path.chmod(0o555 if path.is_dir() else 0o444)
    isolated.chmod(0o555)
    return temporary, isolated


def select_cases(values: list[str]) -> list[Path]:
    cases_root = (DATASET_DIR / "cases").resolve()
    if not values:
        return sorted(path for path in (DATASET_DIR / "cases").iterdir() if path.is_dir())
    selected = []
    for value in values:
        path = Path(value)
        if not path.is_absolute():
            path = DATASET_DIR / value
        path = path.resolve()
        if not path.is_relative_to(cases_root) or not path.is_dir():
            raise ValueError(f"case must be a directory below {cases_root}: {value}")
        selected.append(path)
    if len(selected) != len(set(selected)):
        raise ValueError("duplicate cases are not allowed")
    return selected


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cases", nargs="*", help="Case paths relative to golden-dataset/")
    parser.add_argument("--validate-only", action="store_true")
    parser.add_argument("--trials", type=int, default=None)
    parser.add_argument("--evaluator-cmd", default=os.getenv("SDDK_EVALUATOR_CMD"))
    parser.add_argument("--judge-cmd", default=os.getenv("SDDK_JUDGE_CMD"))
    parser.add_argument("--evaluator-id", default="evaluator-under-test")
    parser.add_argument("--judge-id", default="adversarial-judge")
    parser.add_argument("--evaluator-model", default=os.getenv("SDDK_EVALUATOR_MODEL", "unknown"))
    parser.add_argument("--judge-model", default=os.getenv("SDDK_JUDGE_MODEL", "unknown"))
    parser.add_argument("--results-dir", type=Path, default=DATASET_DIR / "results")
    parser.add_argument("--pass-env", action="append", default=[], metavar="NAME")
    parser.add_argument(
        "--read-only-path",
        action="append",
        default=[],
        type=Path,
        metavar="PATH",
        help="explicit host path exposed read-only to both roles; repository paths are rejected",
    )
    parser.add_argument("--timeout-seconds", type=int, default=900)
    parser.add_argument("--max-output-bytes", type=int, default=1_048_576)
    parser.add_argument(
        "--network-policy",
        choices=("disabled", "external-model"),
        default="disabled",
        help="disabled uses bwrap network isolation; external-model explicitly permits network egress",
    )
    args = parser.parse_args()

    cases = select_cases(args.cases)
    metadata = {case.name: load_case(case) for case in cases}
    print(f"Validated {len(cases)} golden cases")
    if args.validate_only:
        return 0

    if not args.evaluator_cmd or not args.judge_cmd:
        parser.error("--evaluator-cmd and --judge-cmd are required unless --validate-only is used")
    if args.evaluator_id == args.judge_id:
        parser.error("evaluator and judge identities must differ")
    if not args.evaluator_model or not args.judge_model:
        parser.error("--evaluator-model and --judge-model are required")
    if "unknown" in {args.evaluator_model, args.judge_model}:
        parser.error("model identifiers must be explicit, not 'unknown'")
    if args.trials is not None and args.trials < 1:
        parser.error("--trials must be at least 1")
    if args.timeout_seconds < 1:
        parser.error("--timeout-seconds must be at least 1")
    if args.max_output_bytes < 1:
        parser.error("--max-output-bytes must be at least 1")
    read_only_paths = tuple(path.resolve() for path in args.read_only_path)
    if len(read_only_paths) != len(set(read_only_paths)):
        parser.error("--read-only-path values must be unique")
    if any(
        path.is_relative_to(REPO_ROOT) or REPO_ROOT.is_relative_to(path)
        for path in read_only_paths
    ):
        parser.error("--read-only-path cannot expose the framework repository or its ancestors")

    bundle_hash = digest_tree([REPO_ROOT / "agents", REPO_ROOT / "skills", REPO_ROOT / "prompts"])
    evaluation_hash = digest_tree([DATASET_DIR / "runner", DATASET_DIR / "schemas", DATASET_DIR / "cases"])
    labels_snapshot = {case.name: metadata[case.name] for case in cases}
    labels_bytes = json.dumps(labels_snapshot, indent=2, sort_keys=True).encode() + b"\n"
    run_stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
    run_dir = args.results_dir / f"{run_stamp}-{bundle_hash[:12]}-{secrets.token_hex(3)}"
    run_dir.mkdir(parents=True, exist_ok=False)
    run_manifest = {
        "contract_version": "golden-run/v1",
        "bundle_hash": bundle_hash,
        "evaluation_hash": evaluation_hash,
        "labels_snapshot_digest": digest_bytes(labels_bytes),
        "created_at": datetime.now(timezone.utc).isoformat(),
        "evaluator": {"identity": args.evaluator_id, "model": args.evaluator_model},
        "judge": {"identity": args.judge_id, "model": args.judge_model},
        "command_digests": {
            "evaluator": digest_bytes(args.evaluator_cmd.encode()),
            "judge": digest_bytes(args.judge_cmd.encode()),
        },
        "execution_policy": {
            "network": args.network_policy,
            "passed_environment": sorted(set(args.pass_env)),
            "read_only_paths": sorted(str(path) for path in read_only_paths),
            "timeout_seconds": args.timeout_seconds,
            "max_output_bytes": args.max_output_bytes,
        },
        "cases": [case.name for case in cases],
    }
    validate_run_manifest(run_manifest)
    (run_dir / "run.json").write_text(json.dumps(run_manifest, indent=2) + "\n")
    bundle_temporary, isolated_bundle_root = isolate_bundle()
    isolated_bundle_hash = digest_tree(
        [
            isolated_bundle_root / "agents",
            isolated_bundle_root / "skills",
            isolated_bundle_root / "prompts",
        ],
        isolated_bundle_root,
    )
    if isolated_bundle_hash != bundle_hash:
        bundle_temporary.cleanup()
        raise RuntimeError("isolated bundle hash does not match the run manifest")

    try:
        for case_dir in cases:
            case_meta = metadata[case_dir.name]
            trials = args.trials or int(case_meta.get("trials", 1))
            for trial in range(1, trials + 1):
                trial_dir = run_dir / case_dir.name / f"trial-{trial:03d}"
                trial_dir.mkdir(parents=True, exist_ok=False)
                temporary, isolated_case_dir = isolate_case(case_dir)
                isolated_roles = Path(temporary.name) / "roles"
                try:
                    evaluator_input = role_input(
                        isolated_case_dir,
                        case_meta,
                        bundle_hash,
                        trial,
                        "evaluator",
                        bundle_root=isolated_bundle_root,
                    )
                    evaluator_output = run_role(
                        args.evaluator_cmd,
                        args.evaluator_id,
                        args.evaluator_model,
                        evaluator_input,
                        isolated_roles / "evaluator",
                        tuple(args.pass_env),
                        args.timeout_seconds,
                        args.max_output_bytes,
                        args.network_policy,
                        read_only_paths,
                    )
                    judge_input = role_input(
                        isolated_case_dir,
                        case_meta,
                        bundle_hash,
                        trial,
                        "judge",
                        evaluator_output,
                        isolated_bundle_root,
                    )
                    run_role(
                        args.judge_cmd,
                        args.judge_id,
                        args.judge_model,
                        judge_input,
                        isolated_roles / "judge",
                        tuple(args.pass_env),
                        args.timeout_seconds,
                        args.max_output_bytes,
                        args.network_policy,
                        read_only_paths,
                    )
                finally:
                    if isolated_roles.exists():
                        for role_dir in isolated_roles.iterdir():
                            shutil.copytree(role_dir, trial_dir / role_dir.name)
                    temporary.cleanup()
    finally:
        bundle_temporary.cleanup()

    # Keep held-out labels unavailable for the full evaluator/judge execution.
    (run_dir / "labels.snapshot.json").write_bytes(labels_bytes)
    summary = grade_run(run_dir, labels_snapshot)
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0 if summary["failed_trials"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
