# ADR-HX-001 — Canonical State Authority

**Status:** proposed  
**Date:** 2026-08-28

## Context
Prompts actuales contienen lenguaje histórico inconsistente sobre CLI/ledger, vault y git.

## Decision
Runtime = CLI/ledger; artifacts = XDG/CAS; code = Git; durable knowledge = vault; chat = context only. CurrentRunView es una proyección.

## Consequences
Elimina ambigüedad y hace Resume verificable. Obliga a corregir status-query y contract tests.

## Alternatives considered
Vault-primary fue rechazado porque un vault puede estar stale. Chat-primary fue rechazado por no ser durable.
