# SPEC-CONF-009 — UAT Traceability and Conformance Receipt

## Intent

Make “100% compliant” an evidence-backed statement rather than a roadmap status label.

## Traceability schema

Every UAT and normative requirement MUST record:

```text
requirement_id
normative_source
implementation_owner
code_refs
test_refs
fixture/data
execution_command
result
artifact/version/commit
compatibility_notes
open_findings
```

## Test hierarchy

Evidence SHOULD be layered:

- T0 static architecture/forbidden-dependency checks;
- T1 pure domain/unit tests;
- T2 application/port contract tests;
- T3 storage/migration tests;
- T4 installed CLI contract tests;
- T5 end-to-end sandbox UAT;
- T6 migrated-repository/recovery/rebuild tests.

No single layer substitutes for the others where the normative requirement crosses boundaries.

## UAT execution

UAT-01..22 from the 09/09 package MUST each map to at least one executable scenario. The matrix in `../uat/UAT-09-09-CONFORMANCE-MASTER.md` is the authoritative closeout list.

## Final receipt

The closeout produces `09-09-CONFORMANCE-RECEIPT.md` from the template in `../reference/CONFORMANCE-RECEIPT-TEMPLATE.md`.

A valid receipt MUST contain:

- exact repository commit and installed SDDK version;
- baseline document hash/version;
- table of SPEC-001..018 status;
- table of UAT-01..22 results;
- all migration/legacy allowlists;
- architecture fitness output;
- projection rebuild/recovery evidence;
- clean-repo and migrated-repo evidence;
- unresolved findings section, which MUST be empty for 100% conformance.

## Prohibition

Do not label the baseline 100% conformant if any MUST requirement is `UNKNOWN`, `BLOCKED` or `FAIL`.
