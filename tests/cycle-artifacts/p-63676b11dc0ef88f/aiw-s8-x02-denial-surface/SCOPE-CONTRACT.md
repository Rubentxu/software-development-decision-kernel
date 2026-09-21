# SCOPE-CONTRACT — AIW-S8 X02 — Denial surface (raw args/secretos)

> **Slice id:** `p-63676b11dc0ef88f/aiw-s8-x02-denial-surface`
> **Parent:** `aiw-s8-cli-host-evaluation/SCOPE-CONTRACT.md` §S8-STOP-1
> **Status:** ✅ IMPLEMENTED
> **HEAD base:** 7fca3d9

## §1 Goal

Close X02 (SEC): `host accede a raw/denied args/secretos. Debe ser
imposible por API estable o denegarse; 0 leak.`

The STOP in §S8-STOP-1 was lifted by operator decision: this slice
implements the denial layer at parse/evaluation time for host action
requests crossing the gateway boundary.

## §2 In scope

- `crates/sddk-gateway/src/denial_surface.rs` (NEW): `DenialPolicy`,
  `DenialReason`, `DenialVerdict`, `HostActionRequest::evaluate`.
- Export via `crates/sddk-gateway/src/lib.rs` (`pub mod denial_surface`).
- Integration tests `crates/sddk-gateway/tests/aiw_s8_x02_denial_surface.rs`.

## §3 Zero-leak contract

A `DenialReason` carries only structural facts: the matched secret
**prefix** (never the argument value), payload **sizes**, or the bare
fact of malformed bytes (`RawBytesArg`, `NonUtf8Arg`, `EmptyActionId`).
Pinned by `denial_zero_leak_no_payload_in_reason` (unit) and
`integration_denial_zero_leak_no_payload_in_reason` +
`integration_with_host_action_request` (rendered-verdict leak scan).

## §4 Out of scope

- Wiring `evaluate()` into `sddk-engine`'s `AgentHost` dispatch path
  (host-side adoption is a follow-up; this slice establishes the surface).
- Payload content inspection beyond size cap (no malware scanning).
