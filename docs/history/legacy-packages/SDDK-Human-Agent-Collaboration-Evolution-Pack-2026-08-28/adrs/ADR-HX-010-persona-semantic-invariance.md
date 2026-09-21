# ADR-HX-010 — Persona Semantic Invariance

**Status:** proposed  
**Date:** 2026-08-28

## Context
Una personalidad sarcástica puede accidentalmente ocultar o deformar información.

## Decision
Persona es un transform post-semántico; facts/verdict/risk/required_action son invariantes. Safety tone filter tiene precedencia.

## Consequences
Permite personalidad fuerte con garantías.

## Alternatives considered
Persona dentro de phase prompts o reasoning rechazado.
