# SDDK E2E Validation Report

**Date:** 2026-08-06T20:09Z
**Version under test:** v1.3.0 (latest)
**Stack:** podman + act (local CI) + mmdc + chrome headless + musl static build

## Summary

| Suite | Verdict |
|-------|---------|
| n1-install | PASS |
| n2-render | PASS |
| ml-rust | PASS |
| ml-python | PASS |
| ml-go | PASS |
| ml-node | PASS |
| ml-c | PASS |
| n3-editor (checklist entorno real) | PASS |

## Evidence

- N1 reports: `~/.sddk-e2e/{a,b,c,d}/report.json` (install variants: no-cosign, cosign keyless, editor-none, pinned)
- N2 artifacts: `~/.sddk-e2e/render/diagrams/workflow-states.svg` + `screenshots/vault-inspector.png`, `screenshots/closing-report.png`
- ML reports: `~/.sddk-validate/fixture-{rust,python,go,node,c}/report.json` (adopt + cycle open on fixture projects, musl static binary)
- N3 checklist: `e2e-plan.md` §N3 (dev link → agents/skills/prompts; doctor all_present; orchestrator + sddk-verify prompts resueltos; skills sddk-* disponibles)

## Key Fixes This Round

- **musl static build**: binary now runs on ANY Linux base (was GLIBC 2.39 vs bookworm 2.36)
- **build image fixed to rust:alpine** (language-agnostic; golang/node/gcc images have no cargo)
- **baseline pass pattern per language** (cargo vs unittest vs go test vs node:test vs gcc)
- **pinned variant resolves 'latest'** to the real released version
