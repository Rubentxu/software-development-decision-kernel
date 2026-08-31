#!/usr/bin/env python3
"""Deterministic contracts for the external SDDK agent-evaluation corpus."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parents[1]
DATASET = ROOT / "golden-dataset"
CASES = DATASET / "cases"


def load_runner():
    runner_dir = DATASET / "runner"
    sys.path.insert(0, str(runner_dir))
    spec = importlib.util.spec_from_file_location("run_golden", runner_dir / "run_golden.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class GoldenDatasetContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.case_dirs = sorted(path for path in CASES.iterdir() if path.is_dir())
        cls.cases = {
            path.name: yaml.safe_load((path / "expected-verdict.yaml").read_text())
            for path in cls.case_dirs
        }

    def test_case_contract_and_unique_ids(self) -> None:
        self.assertGreaterEqual(len(self.cases), 14)
        self.assertEqual(len(self.cases), len(set(self.cases)))
        for name, case in self.cases.items():
            with self.subTest(case=name):
                self.assertEqual(case["case"], name)
                self.assertEqual(case["schema_version"], "golden-case/v1")
                self.assertTrue(case["held_out"])
                self.assertGreaterEqual(case["trials"], 1)
                self.assertIn(case["target_phase"], {"verify", "debt"})
                self.assertIn(case["path"], {"B-direct", "A-min", "A-lite", "A-full"})
                self.assertTrue((CASES / name / "spec.md").is_file())
                self.assertTrue((CASES / name / "implementation").is_dir())
                labels = case["expected"].get("labels", [])
                label_ids = [(label["rule_id"], label["location"]) for label in labels]
                self.assertEqual(len(label_ids), len(set(label_ids)))

    def test_multistack_and_suite_coverage(self) -> None:
        languages = {case["language"] for case in self.cases.values()}
        self.assertTrue({"rust", "go", "python", "typescript"} <= languages)
        suites = {case["suite"] for case in self.cases.values()}
        self.assertTrue(
            {
                "negative-control",
                "verify-defects",
                "test-strength",
                "cli-contract",
                "adversarial-evidence",
                "routing-boundary",
                "finding-contract",
                "communication",
            }
            <= suites
        )

    def test_role_input_withholds_expected_labels(self) -> None:
        runner = load_runner()
        case_dir = self.case_dirs[0]
        case = self.cases[case_dir.name]
        value = runner.role_input(case_dir, case, "a" * 64, 1, "evaluator")
        serialized = json.dumps(value)
        self.assertNotIn("expected", value)
        self.assertNotIn("forbidden_rule_ids", serialized)
        for label in case["expected"].get("labels", []):
            self.assertNotIn(label["rule_id"], serialized)

    def test_runner_separates_roles_and_records_evidence(self) -> None:
        runner = (DATASET / "runner/run_golden.py").read_text()
        self.assertIn("evaluator and judge identities must differ", runner)
        self.assertIn("subprocess.Popen", runner)
        self.assertIn("TemporaryDirectory", runner)
        self.assertNotIn("**os.environ", runner)
        self.assertIn("output_digest", runner)
        self.assertIn("tool-trace.jsonl", runner)
        self.assertNotIn("PENDING", runner)
        grader = (DATASET / "runner/grade_results.py").read_text()
        for metric in ["precision", "recall", "f1", "false_block_rate", "critical_escape_rate", "pass_k"]:
            self.assertIn(f'"{metric}"', grader)

    def test_role_execution_and_grader_are_executable(self) -> None:
        runner = load_runner()
        case_dir = CASES / "01-clean-pass"
        case = self.cases[case_dir.name]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            case_temporary, isolated_case = runner.isolate_case(case_dir)
            bundle_temporary, isolated_bundle = runner.isolate_bundle()
            role_script = root / "role.py"
            secret_path = root / "host-secret.txt"
            secret_path.write_text("must-not-be-visible")
            role_script.write_text(
                "import json, os\n"
                f"assert not os.path.exists({str(case_dir / 'expected-verdict.yaml')!r})\n"
                f"assert not os.path.exists({str(secret_path)!r})\n"
                "assert os.path.isdir(os.path.join(os.environ['SDDK_EVAL_BUNDLE'], 'prompts'))\n"
                "json.dump({'identity': os.environ['SDDK_EVAL_IDENTITY'], "
                "'model': os.environ['SDDK_EVAL_MODEL'], 'provider': 'test', "
                "'invocation_id': 'test-1'}, open(os.environ['SDDK_EVAL_PROVENANCE'], 'w'))\n"
                "json.dump({'verdict': 'PASS', 'findings': []}, "
                "open(os.environ['SDDK_EVAL_OUTPUT'], 'w'))\n"
            )
            try:
                value = runner.role_input(
                    isolated_case,
                    case,
                    "a" * 64,
                    1,
                    "evaluator",
                    bundle_root=isolated_bundle,
                )
                output = runner.run_role(
                    f"{sys.executable} {role_script}",
                    "test-role",
                    "test-model",
                    value,
                    root / "role",
                )
            finally:
                case_temporary.cleanup()
                bundle_temporary.cleanup()
            self.assertEqual(output["verdict"], "PASS")
            execution = json.loads((root / "role/execution.json").read_text())
            self.assertEqual(execution["exit_code"], 0)

            run_dir = root / "run"
            trial_dir = run_dir / case_dir.name / "trial-001"
            for role in ["evaluator", "judge"]:
                role_dir = trial_dir / role
                role_dir.mkdir(parents=True)
                (role_dir / "output.json").write_text(
                    json.dumps({"verdict": "PASS", "findings": []})
                )
            summary = runner.grade_run(run_dir, {case_dir.name: case})
            self.assertEqual(summary["failed_trials"], 0)
            self.assertTrue(summary["pass_k"][case_dir.name]["all_passed"])

    def test_isolated_case_excludes_labels_and_protects_source(self) -> None:
        runner = load_runner()
        source = CASES / "01-clean-pass"
        temporary, isolated = runner.isolate_case(source)
        try:
            self.assertFalse((isolated / "expected-verdict.yaml").exists())
            self.assertEqual((isolated / "spec.md").read_text(), (source / "spec.md").read_text())
            isolated_file = isolated / "implementation/formatPrice.ts"
            self.assertEqual(isolated_file.stat().st_mode & 0o222, 0)
            self.assertNotEqual(isolated.parent, source.parent)
        finally:
            temporary.cleanup()

    def test_isolated_bundle_contains_only_evaluation_assets(self) -> None:
        runner = load_runner()
        temporary, isolated = runner.isolate_bundle()
        try:
            self.assertEqual(
                {path.name for path in isolated.iterdir()},
                {"agents", "skills", "prompts"},
            )
            self.assertFalse((isolated / "golden-dataset").exists())
            self.assertEqual(isolated.stat().st_mode & 0o222, 0)
        finally:
            temporary.cleanup()

    def test_role_environment_is_allowlisted(self) -> None:
        runner = load_runner()
        case_dir = CASES / "01-clean-pass"
        case = self.cases[case_dir.name]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            case_temporary, isolated_case = runner.isolate_case(case_dir)
            bundle_temporary, isolated_bundle = runner.isolate_bundle()
            role_script = root / "role.py"
            role_script.write_text(
                "import json, os\n"
                "assert 'SDDK_TEST_SECRET' not in os.environ\n"
                "json.dump({'identity': os.environ['SDDK_EVAL_IDENTITY'], "
                "'model': os.environ['SDDK_EVAL_MODEL'], 'provider': 'test', "
                "'invocation_id': 'test-2'}, open(os.environ['SDDK_EVAL_PROVENANCE'], 'w'))\n"
                "json.dump({'verdict': 'PASS', 'findings': []}, "
                "open(os.environ['SDDK_EVAL_OUTPUT'], 'w'))\n"
            )
            os.environ["SDDK_TEST_SECRET"] = "must-not-leak"
            try:
                value = runner.role_input(
                    isolated_case,
                    case,
                    "a" * 64,
                    1,
                    "evaluator",
                    bundle_root=isolated_bundle,
                )
                runner.run_role(
                    f"{sys.executable} {role_script}",
                    "test-role",
                    "test-model",
                    value,
                    root / "role",
                    network_policy="external-model",
                )
            finally:
                del os.environ["SDDK_TEST_SECRET"]
                case_temporary.cleanup()
                bundle_temporary.cleanup()

    def test_read_only_path_rejects_repository_ancestors(self) -> None:
        process = subprocess.run(
            [
                sys.executable,
                str(DATASET / "runner/run_golden.py"),
                "cases/01-clean-pass",
                "--evaluator-cmd",
                "unused",
                "--judge-cmd",
                "unused",
                "--evaluator-model",
                "model-a",
                "--judge-model",
                "model-b",
                "--read-only-path",
                "/",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(process.returncode, 0)
        self.assertIn("repository or its ancestors", process.stderr)

    def test_grader_rejects_incomplete_findings_and_zero_trials(self) -> None:
        runner = load_runner()
        invalid = {
            "verdict": "FAIL",
            "findings": [{"rule_id": "verify.incomplete"}],
        }
        with self.assertRaisesRegex(ValueError, "missing fields"):
            runner.validate_role_output(invalid)

        with tempfile.TemporaryDirectory() as temporary:
            run_dir = Path(temporary)
            with self.assertRaisesRegex(ValueError, "zero trials"):
                runner.grade_run(run_dir, {})

    def test_role_output_allows_one_rule_at_multiple_locations(self) -> None:
        runner = load_runner()

        def finding(finding_id: str, path: str) -> dict:
            return {
                "finding_id": finding_id,
                "rule_id": "verify.repeated-defect",
                "subject": {"base": None, "head": None, "diff_digest": None},
                "location": {"path": path, "start_line": 1, "end_line": 1},
                "classification": "blocking_defect",
                "severity": "high",
                "confidence": "high",
                "production_reachable": "yes",
                "evidence": [{"kind": "source", "observation": "defect", "output_digest": None}],
                "exemption": None,
                "owner_phase": "verify",
            }

        output = {
            "verdict": "FAIL",
            "findings": [finding("a" * 64, "implementation/a.py"), finding("b" * 64, "implementation/b.py")],
        }
        self.assertIs(runner.validate_role_output(output), output)
        from grade_results import grade_trial

        expected = {
            "target_phase": "verify",
            "expected": {
                "verify": "FAIL",
                "debt": "FAIL",
                "labels": [
                    {
                        "rule_id": "verify.repeated-defect",
                        "classification": "blocking_defect",
                        "severity": "high",
                        "location": "implementation/a.py",
                    },
                    {
                        "rule_id": "verify.repeated-defect",
                        "classification": "blocking_defect",
                        "severity": "high",
                        "location": "implementation/b.py",
                    },
                ],
            },
        }
        grade = grade_trial(expected, output, output)
        self.assertEqual(grade["tp"], 2)
        self.assertEqual(grade["fp"], 0)
        self.assertEqual(grade["fn"], 0)

    def test_run_manifest_validation_rejects_unbound_provenance(self) -> None:
        runner = load_runner()
        manifest = {
            "contract_version": "golden-run/v1",
            "bundle_hash": "a" * 64,
            "evaluation_hash": "b" * 64,
            "labels_snapshot_digest": "c" * 64,
            "created_at": "2026-08-24T12:00:00+00:00",
            "evaluator": {"identity": "evaluator", "model": "model-a"},
            "judge": {"identity": "judge", "model": "model-b"},
            "command_digests": {"evaluator": "d" * 64, "judge": "e" * 64},
            "execution_policy": {
                "network": "disabled",
                "passed_environment": [],
                "read_only_paths": [],
                "timeout_seconds": 60,
                "max_output_bytes": 1024,
            },
            "cases": ["01-clean-pass"],
        }
        self.assertIs(runner.validate_run_manifest(manifest), manifest)
        manifest["evaluator"] = {"identity": "evaluator", "model": ""}
        with self.assertRaisesRegex(ValueError, "empty evaluator"):
            runner.validate_run_manifest(manifest)

    def test_end_to_end_clean_trial(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            role_script = root / "role.py"
            role_script.write_text(
                "import json, os, pathlib\n"
                f"assert not list(pathlib.Path({str(root / 'results')!r}).glob('*/labels.snapshot.json'))\n"
                "json.dump({'identity': os.environ['SDDK_EVAL_IDENTITY'], "
                "'model': os.environ['SDDK_EVAL_MODEL'], 'provider': 'test', "
                "'invocation_id': os.environ['SDDK_EVAL_IDENTITY'] + '-1'}, "
                "open(os.environ['SDDK_EVAL_PROVENANCE'], 'w'))\n"
                "json.dump({'verdict': 'PASS', 'findings': []}, "
                "open(os.environ['SDDK_EVAL_OUTPUT'], 'w'))\n"
            )
            process = subprocess.run(
                [
                    sys.executable,
                    str(DATASET / "runner/run_golden.py"),
                    "cases/01-clean-pass",
                    "--trials",
                    "1",
                    "--evaluator-model",
                    "model-a",
                    "--judge-model",
                    "model-b",
                    "--evaluator-cmd",
                    f"{sys.executable} {role_script}",
                    "--judge-cmd",
                    f"{sys.executable} {role_script}",
                    "--results-dir",
                    str(root / "results"),
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(process.returncode, 0, process.stderr)
            summaries = list((root / "results").glob("*/summary.json"))
            self.assertEqual(len(summaries), 1)
            summary = json.loads(summaries[0].read_text())
            self.assertEqual(summary["trials"], 1)
            self.assertEqual(summary["failed_trials"], 0)
            self.assertEqual(len(list((root / "results").glob("*/labels.snapshot.json"))), 1)

    def test_role_output_limit_is_enforced_during_execution(self) -> None:
        runner = load_runner()
        case_dir = CASES / "01-clean-pass"
        case = self.cases[case_dir.name]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            case_temporary, isolated_case = runner.isolate_case(case_dir)
            bundle_temporary, isolated_bundle = runner.isolate_bundle()
            role_script = root / "oversized.py"
            role_script.write_text("import os\nos.write(1, b'x' * 8192)\n")
            try:
                value = runner.role_input(
                    isolated_case,
                    case,
                    "a" * 64,
                    1,
                    "evaluator",
                    bundle_root=isolated_bundle,
                )
                with self.assertRaisesRegex(RuntimeError, "file limit|exceeded"):
                    runner.run_role(
                        f"{sys.executable} {role_script}",
                        "limited-role",
                        "test-model",
                        value,
                        root / "role",
                        max_output_bytes=1024,
                    )
            finally:
                case_temporary.cleanup()
                bundle_temporary.cleanup()

    def test_verify_pipeline_and_finding_contract(self) -> None:
        verify = (ROOT / "prompts/sddk/phases/verify.md").read_text()
        for layer in range(7):
            self.assertIn(f"L{layer}", verify)
        schema = json.loads((ROOT / "prompts/sddk/contracts/verify-finding.schema.json").read_text())
        required = set(schema["required"])
        self.assertTrue(
            {
                "finding_id",
                "rule_id",
                "subject",
                "location",
                "classification",
                "severity",
                "confidence",
                "production_reachable",
                "evidence",
                "exemption",
                "owner_phase",
            }
            <= required
        )
        self.assertIn("debt-verify", schema["properties"]["owner_phase"]["enum"])

    def test_architecture_intent_flows_through_phase_contracts(self) -> None:
        propose = (ROOT / "prompts/sddk/phases/propose.md").read_text()
        design = (ROOT / "prompts/sddk/phases/design.md").read_text()
        verify = (ROOT / "prompts/sddk/phases/verify.md").read_text()
        model = (ROOT / "skills/sddk-c4-likec4/references/model-contract.md").read_text()
        self.assertIn("quality_intent", propose)
        self.assertIn("architecture_impact", propose)
        self.assertIn("skills/sddk-c4-likec4/SKILL.md", design)
        self.assertIn("planned_but_missing", verify)
        for field in [
            "cycle_id",
            "phase: propose | design | verify | archive",
            "accepted_evidence_coverage",
            "tool_versions",
            "diagnostics",
        ]:
            self.assertIn(field, model)

    def test_cli_mutants_and_budgets_are_covered(self) -> None:
        mutant = self.cases["26-cli-contract-mutants"]
        rules = {label["rule_id"] for label in mutant["expected"]["labels"]}
        self.assertEqual(len(rules), 8)
        self.assertIn("cli.owner.worker-lifecycle", rules)
        contract = (ROOT / "skills/_shared/cli-usage-contract.md").read_text()
        self.assertIn("## Call Budgets", contract)
        self.assertIn("| Verify | 1 | 0-1 if required | 1 | 2 | 1 | 1 |", contract)
        self.assertIn("workers and lenses always have a lifecycle budget of", contract)


if __name__ == "__main__":
    unittest.main()
