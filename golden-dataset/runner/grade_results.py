#!/usr/bin/env python3
"""Deterministically grade isolated golden evaluation outputs."""

from __future__ import annotations

import argparse
from collections import Counter
import json
import re
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
FINDING_SCHEMA = json.loads(
    (REPO_ROOT / "prompts/sddk/contracts/verify-finding.schema.json").read_text()
)
FINDING_REQUIRED = set(FINDING_SCHEMA["required"])
FINDING_PROPERTIES = set(FINDING_SCHEMA["properties"])
VERDICTS = {"PASS", "PASS_WITH_WARNINGS", "FAIL", "INCONCLUSIVE"}
CLASSIFICATIONS = set(FINDING_SCHEMA["properties"]["classification"]["enum"])
SEVERITIES = set(FINDING_SCHEMA["properties"]["severity"]["enum"])
CONFIDENCES = set(FINDING_SCHEMA["properties"]["confidence"]["enum"])
PRODUCTION_REACHABILITY = set(
    FINDING_SCHEMA["properties"]["production_reachable"]["enum"]
)
OWNER_PHASES = set(FINDING_SCHEMA["properties"]["owner_phase"]["enum"])
EVIDENCE_KINDS = set(
    FINDING_SCHEMA["properties"]["evidence"]["items"]["properties"]["kind"]["enum"]
)
RULE_ID = re.compile(r"^[a-z0-9]+(?:[.-][a-z0-9]+)*$")
SHA256 = re.compile(r"^[a-f0-9]{64}$")
GIT_SHA = re.compile(r"^[a-f0-9]{40,64}$")


def safe_ratio(numerator: int, denominator: int) -> float:
    return numerator / denominator if denominator else 0.0


def require_keys(value: dict[str, Any], required: set[str], allowed: set[str], context: str) -> None:
    missing = required - value.keys()
    unexpected = value.keys() - allowed
    if missing:
        raise ValueError(f"{context}: missing fields {sorted(missing)}")
    if unexpected:
        raise ValueError(f"{context}: unexpected fields {sorted(unexpected)}")


def validate_role_output(output: Any, context: str = "role output") -> dict[str, Any]:
    if not isinstance(output, dict):
        raise ValueError(f"{context}: must be a JSON object")
    require_keys(output, {"verdict", "findings"}, {"verdict", "findings"}, context)
    if output["verdict"] not in VERDICTS:
        raise ValueError(f"{context}: invalid verdict {output['verdict']!r}")
    if not isinstance(output["findings"], list):
        raise ValueError(f"{context}: findings must be an array")

    finding_ids: set[str] = set()
    for index, finding in enumerate(output["findings"]):
        finding_context = f"{context}.findings[{index}]"
        if not isinstance(finding, dict):
            raise ValueError(f"{finding_context}: must be an object")
        require_keys(finding, FINDING_REQUIRED, FINDING_PROPERTIES, finding_context)
        if not isinstance(finding["finding_id"], str) or not SHA256.fullmatch(finding["finding_id"]):
            raise ValueError(f"{finding_context}: finding_id must be a SHA-256 fingerprint")
        if finding["finding_id"] in finding_ids:
            raise ValueError(f"{finding_context}: duplicate finding_id")
        finding_ids.add(finding["finding_id"])
        if not isinstance(finding["rule_id"], str) or not RULE_ID.fullmatch(finding["rule_id"]):
            raise ValueError(f"{finding_context}: invalid rule_id")
        if finding["classification"] not in CLASSIFICATIONS:
            raise ValueError(f"{finding_context}: invalid classification")
        if finding["severity"] not in SEVERITIES:
            raise ValueError(f"{finding_context}: invalid severity")
        if finding["confidence"] not in CONFIDENCES:
            raise ValueError(f"{finding_context}: invalid confidence")
        if finding["production_reachable"] not in PRODUCTION_REACHABILITY:
            raise ValueError(f"{finding_context}: invalid production_reachable")
        if finding["owner_phase"] not in OWNER_PHASES:
            raise ValueError(f"{finding_context}: invalid owner_phase")

        subject = finding["subject"]
        if not isinstance(subject, dict):
            raise ValueError(f"{finding_context}.subject: must be an object")
        require_keys(subject, {"base", "head", "diff_digest"}, {"base", "head", "diff_digest"}, f"{finding_context}.subject")
        for field in ("base", "head"):
            if subject[field] is not None and (
                not isinstance(subject[field], str) or not GIT_SHA.fullmatch(subject[field])
            ):
                raise ValueError(f"{finding_context}.subject.{field}: must be a Git SHA or null")
        if subject["diff_digest"] is not None and (
            not isinstance(subject["diff_digest"], str) or not SHA256.fullmatch(subject["diff_digest"])
        ):
            raise ValueError(f"{finding_context}.subject.diff_digest: must be SHA-256 or null")

        location = finding["location"]
        if not isinstance(location, dict):
            raise ValueError(f"{finding_context}.location: must be an object")
        require_keys(
            location,
            {"path", "start_line", "end_line"},
            {"path", "start_line", "end_line", "symbol"},
            f"{finding_context}.location",
        )
        if not isinstance(location["path"], str) or not location["path"]:
            raise ValueError(f"{finding_context}.location.path: must be non-empty")
        if not isinstance(location["start_line"], int) or location["start_line"] < 1:
            raise ValueError(f"{finding_context}.location.start_line: must be >= 1")
        if not isinstance(location["end_line"], int) or location["end_line"] < location["start_line"]:
            raise ValueError(f"{finding_context}.location.end_line: must be >= start_line")
        if "symbol" in location and location["symbol"] is not None and not isinstance(location["symbol"], str):
            raise ValueError(f"{finding_context}.location.symbol: must be a string or null")

        evidence = finding["evidence"]
        if not isinstance(evidence, list) or not evidence:
            raise ValueError(f"{finding_context}.evidence: must be a non-empty array")
        for evidence_index, item in enumerate(evidence):
            evidence_context = f"{finding_context}.evidence[{evidence_index}]"
            if not isinstance(item, dict):
                raise ValueError(f"{evidence_context}: must be an object")
            require_keys(
                item,
                {"kind", "observation", "output_digest"},
                {"kind", "observation", "command", "exit_code", "output_digest"},
                evidence_context,
            )
            if item["kind"] not in EVIDENCE_KINDS:
                raise ValueError(f"{evidence_context}: invalid kind")
            if not isinstance(item["observation"], str) or not item["observation"]:
                raise ValueError(f"{evidence_context}: observation must be non-empty")
            if item["output_digest"] is not None and (
                not isinstance(item["output_digest"], str)
                or not SHA256.fullmatch(item["output_digest"])
            ):
                raise ValueError(f"{evidence_context}: output_digest must be SHA-256 or null")
            if "command" in item and item["command"] is not None and not isinstance(item["command"], str):
                raise ValueError(f"{evidence_context}: command must be a string or null")
            if "exit_code" in item and item["exit_code"] is not None and not isinstance(item["exit_code"], int):
                raise ValueError(f"{evidence_context}: exit_code must be an integer or null")

        exemption = finding["exemption"]
        if exemption is not None:
            if not isinstance(exemption, dict):
                raise ValueError(f"{finding_context}.exemption: must be an object or null")
            require_keys(
                exemption,
                {"authority", "reason", "expires_at"},
                {"authority", "reason", "expires_at"},
                f"{finding_context}.exemption",
            )
            if not isinstance(exemption["authority"], str) or not exemption["authority"]:
                raise ValueError(f"{finding_context}.exemption.authority: must be non-empty")
            if not isinstance(exemption["reason"], str) or not exemption["reason"]:
                raise ValueError(f"{finding_context}.exemption.reason: must be non-empty")
            if exemption["expires_at"] is not None and not isinstance(exemption["expires_at"], str):
                raise ValueError(f"{finding_context}.exemption.expires_at: must be a string or null")
    return output


def semantic_findings(output: dict[str, Any]) -> Counter[tuple[str, str, str, str]]:
    return Counter(
        (
            finding["rule_id"],
            finding["classification"],
            finding["severity"],
            finding["location"]["path"],
        )
        for finding in output["findings"]
    )


def grade_trial(expected: dict[str, Any], actual: dict[str, Any], judge: dict[str, Any]) -> dict[str, Any]:
    actual = validate_role_output(actual, "evaluator output")
    judge = validate_role_output(judge, "judge output")
    labels = expected["expected"].get("labels", [])
    forbidden_rules = set(expected["expected"].get("forbidden_rule_ids", []))
    expected_findings = Counter(
        (label["rule_id"], label["classification"], label["severity"], label["location"])
        for label in labels
    )
    actual_findings = semantic_findings(actual)
    matching_findings = expected_findings & actual_findings
    missing_findings = expected_findings - matching_findings
    unexpected_findings = actual_findings - matching_findings
    forbidden_matches = Counter(
        {
            signature: count
            for signature, count in matching_findings.items()
            if signature[0] in forbidden_rules
        }
    )
    tp = sum(matching_findings.values())
    fn = sum(missing_findings.values())
    fp = sum(unexpected_findings.values()) + sum(forbidden_matches.values())
    tn = 1 if not expected_findings and not actual_findings else 0
    target_phase = expected["target_phase"]
    expected_verdict = expected["expected"][target_phase]
    actual_verdict = actual.get("verdict")
    critical_expected = any(label["severity"] == "critical" for label in labels)
    critical_escape = critical_expected and (
        actual_verdict != "FAIL"
        or any(
            signature[2] == "critical" and count > 0
            for signature, count in missing_findings.items()
        )
    )
    clean_expected = not expected_findings and expected_verdict == "PASS"
    false_block = clean_expected and actual_verdict in {"FAIL", "INCONCLUSIVE"}
    return {
        "target_phase": target_phase,
        "expected_verdict": expected_verdict,
        "actual_verdict": actual_verdict,
        "verdict_match": actual_verdict == expected_verdict,
        "tp": tp,
        "fp": fp,
        "fn": fn,
        "tn": tn,
        "missing_rule_ids": sorted(
            signature[0]
            for signature, count in missing_findings.items()
            for _ in range(count)
        ),
        "unexpected_rule_ids": sorted(
            [
                signature[0]
                for signature, count in unexpected_findings.items()
                for _ in range(count)
            ]
            + [
                signature[0]
                for signature, count in forbidden_matches.items()
                for _ in range(count)
            ]
        ),
        "mismatched_labels": sorted(
            {
                signature[0]
                for signature in missing_findings
                if any(actual_signature[0] == signature[0] for actual_signature in actual_findings)
            }
        ),
        "clean_expected": clean_expected,
        "false_block": false_block,
        "critical_expected": critical_expected,
        "critical_escape": critical_escape,
        "evaluator_judge_disagreement": (
            actual_verdict != judge.get("verdict")
            or semantic_findings(actual) != semantic_findings(judge)
        ),
        "passed": actual_verdict == expected_verdict and fn == 0 and fp == 0,
    }


def grade_run(run_dir: Path, labels: dict[str, Any]) -> dict[str, Any]:
    totals = {"tp": 0, "fp": 0, "fn": 0, "tn": 0}
    trial_count = 0
    failed_trials = 0
    disagreements = 0
    clean_trials = 0
    false_blocks = 0
    critical_trials = 0
    critical_escapes = 0
    case_passes: dict[str, list[bool]] = {}

    for case_dir in sorted(path for path in run_dir.iterdir() if path.is_dir()):
        if case_dir.name not in labels:
            raise ValueError(f"missing label snapshot for {case_dir.name}")
        expected = labels[case_dir.name]
        case_passes[case_dir.name] = []
        for trial_dir in sorted(path for path in case_dir.iterdir() if path.is_dir()):
            evaluator = json.loads((trial_dir / "evaluator/output.json").read_text())
            judge = json.loads((trial_dir / "judge/output.json").read_text())
            grade = grade_trial(expected, evaluator, judge)
            (trial_dir / "grade.json").write_text(json.dumps(grade, indent=2, sort_keys=True) + "\n")
            trial_count += 1
            failed_trials += int(not grade["passed"])
            disagreements += int(grade["evaluator_judge_disagreement"])
            clean_trials += int(grade["clean_expected"])
            false_blocks += int(grade["false_block"])
            critical_trials += int(grade["critical_expected"])
            critical_escapes += int(grade["critical_escape"])
            case_passes[case_dir.name].append(grade["passed"])
            for key in totals:
                totals[key] += grade[key]

    if trial_count == 0:
        raise ValueError("run contains zero trials")
    empty_cases = sorted(case for case, values in case_passes.items() if not values)
    if empty_cases:
        raise ValueError(f"cases contain zero trials: {empty_cases}")

    precision = safe_ratio(totals["tp"], totals["tp"] + totals["fp"])
    recall = safe_ratio(totals["tp"], totals["tp"] + totals["fn"])
    summary = {
        "contract_version": "golden-summary/v1",
        **totals,
        "precision": precision,
        "recall": recall,
        "f1": safe_ratio(2 * precision * recall, precision + recall),
        "trials": trial_count,
        "failed_trials": failed_trials,
        "false_block_rate": safe_ratio(false_blocks, clean_trials),
        "critical_escape_rate": safe_ratio(critical_escapes, critical_trials),
        "evaluator_judge_disagreement_rate": safe_ratio(disagreements, trial_count),
        "pass_k": {
            case: {"k": len(values), "all_passed": all(values), "passes": sum(values)}
            for case, values in case_passes.items()
        },
    }
    (run_dir / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    return summary


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("--labels", type=Path, default=None)
    args = parser.parse_args()
    labels_path = args.labels or args.run_dir / "labels.snapshot.json"
    labels = json.loads(labels_path.read_text())
    print(json.dumps(grade_run(args.run_dir, labels), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
