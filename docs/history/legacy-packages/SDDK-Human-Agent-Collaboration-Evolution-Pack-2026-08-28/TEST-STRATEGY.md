# Estrategia de pruebas

## 1. Domain unit tests
- attention classification;
- risk policy;
- preference promotion;
- reframe validation;
- semantic fact normalization.

## 2. Contract tests
- schemas;
- source-of-truth;
- renderer cannot call lifecycle;
- instruction matrix references reales;
- backward schema compatibility.

## 3. Golden rendering
Para un mismo fact set:
- neutral;
- novice;
- expert;
- audit;
- wisecracking_robot.

Normalizar y verificar identidad de:
- status;
- phase;
- progress;
- blocker;
- risks;
- decision;
- next_action;
- human_action_required.

## 4. Property tests
`persona(render(facts))` preserva `extract_semantics(facts)`.

## 5. Resume fixtures
- clean active cycle;
- blocked phase;
- stale lease;
- missing cycle id;
- interrupted apply;
- released/not archived;
- closed cycle.

## 6. Behavioral parity facade
Los flows facade deben producir los mismos efectos/receipts que las secuencias low-level documentadas.

## 7. Performance
- renderer P50/P95;
- CurrentRunView build;
- profile load/write;
- telemetry overhead.

## 8. Failure injection
- corrupt artifact;
- ledger mismatch;
- missing vault;
- git divergence;
- invalid profile;
- renderer failure;
- persona template invalid.

Principio: fallo de presentation no puede fabricar success ni mutar lifecycle.

## 9. UAT
Ver `uat/UAT-PLAN.md`.
