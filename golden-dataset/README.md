# Golden Dataset - SDDK Agent Evaluation

Corpus held-out para medir verify, debt-verify, evidencia, routing, contratos CLI
y comunicacion. Los labels solo los lee el grader determinista; nunca se pasan al
evaluator ni al juez adversarial.

## Propósito

El harness ejecuta trials aislados con dos identidades distintas:

1. `evaluator`: sistema bajo prueba.
2. `judge`: critica adversarialmente el output sin ver labels.
3. `grade_results.py`: compara despues contra `expected-verdict.yaml`.

## Estructura

```
golden-dataset/
├── cases/
│   ├── 01-clean-pass/              <- cambio limpio
│   │   ├── spec.md                 ← spec del cambio
│   │   ├── implementation/         ← código (2-5 archivos)
│   │   ├── expected-verdict.yaml   ← verdict + findings esperados
│   │   └── known-issues.md         ← falsos positivos conocidos (qué NO debería encontrar)
│   ├── 06-god-class-fail/          ← god-class obvio → FAIL esperado
│   ├── 11-circular-import-fail/    ← circular import → FAIL esperado
│   ├── 16-subtle-feature-envy-pw/  ← deuda sutil → PW esperado
│   └── 21-adversarial-hidden-mutation/ ← parece limpio, tiene issue → FAIL esperado
├── runner/
│   ├── run-golden.sh               <- wrapper estable
│   ├── run_golden.py               <- aislamiento y ejecucion externa
│   └── grade_results.py            <- grader con labels held-out
├── schemas/evaluation.schema.json
└── results/                        <- generado e ignorado por Git
```

## Buckets

| Suite | Casos | Mide |
|---|---|---|---|
| Limpio/debt | 01, 06, 11, 16, 21 | Falsos bloqueos y deuda obvia/sutil/adversarial |
| Verify multi-stack | 22-25 | Placeholders, hardcodes, exenciones y fuerza de tests |
| Contratos | 26-29 | CLI, evidencia, roles y forma de findings |
| Comunicación | 30 | Impacto, evidencia ausente y recuperación accionable |

## Cómo ejecutar

Instala la dependencia versionada del harness y valida el corpus sin ejecutar
modelos:

```bash
python3 -m pip install --requirement golden-dataset/requirements.txt
./golden-dataset/runner/run-golden.sh --validate-only
python3 tests/test_golden_dataset_contract.py
```

Para trials reales, ambos comandos reciben las rutas por placeholders o por
`SDDK_EVAL_INPUT`, `SDDK_EVAL_OUTPUT`, `SDDK_EVAL_TRACE`, `SDDK_EVAL_BUNDLE` y
`SDDK_EVAL_PROVENANCE`. Cada adapter debe escribir provenance JSON con
`identity`, `model`, `provider` e `invocation_id`, coincidentes con la ejecución:

```bash
./golden-dataset/runner/run-golden.sh \
  --evaluator-id verify-under-test \
  --judge-id adversarial-judge \
  --evaluator-model MODEL_A \
  --judge-model MODEL_B \
  --network-policy external-model \
  --pass-env PROVIDER_API_KEY \
  --read-only-path /opt/model-adapters \
  --evaluator-cmd 'eval-agent --input {input} --output {output} --trace {trace} --provenance {provenance}' \
  --judge-cmd 'judge-agent --input {input} --output {output} --trace {trace} --provenance {provenance}'
```

Por defecto `--network-policy disabled` usa `bwrap` para retirar red. El modo
`external-model` permite egress de red explícitamente y debe ejecutarse detrás
del proxy/allowlist del proveedor cuando exista. Solo se heredan variables
indicadas con `--pass-env`, además del entorno mínimo del proceso.
El sandbox no monta `/` ni el home del host. Expone únicamente binarios y
librerías del sistema, caso, bundle, directorio del rol y los paths declarados
explícitamente con `--read-only-path`; cualquier path dentro del checkout del
framework se rechaza. Los adapters con venv/config propio deben declarar solo
su directorio mínimo.

Cada run usa un snapshot temporal del bundle y cada trial otro del caso, ambos
fuera del repositorio y de solo lectura. `bwrap` oculta el checkout original, por
lo que evaluator/judge consumen `SDDK_EVAL_BUNDLE` sin acceso al host ni a
labels. Las labels se materializan solo después de terminar ambos roles. El harness
registra hashes de bundle/evaluación y
labels, política de ejecución, procedencia, argv, timestamps, exit code, digests
de stdout/stderr/trace, inputs sin labels, outputs y grade por trial.

## Cómo añadir un caso

1. Crear `cases/NN-descripción/`
2. Escribir `spec.md` (qué se supone que hace el cambio)
3. Escribir `implementation/` en el stack que corresponda.
4. Escribir `expected-verdict.yaml` con `golden-case/v1`, suite, `target_phase`,
   lenguaje, path, trials y labels por `rule_id`, classification, severity y
   location estables.
5. Escribir `known-issues.md` (falsos positivos a vigilar)

## Métricas que produce

- **Precision** = TP / (TP + FP)
- **Recall** = TP / (TP + FN)
- **F1** = 2 × (precision × recall) / (precision + recall)
- Falsos bloqueos, escapes criticos, desacuerdo evaluator/judge y `pass^k`.

Objetivo: precision > 0.8, recall > 0.7. Por debajo de eso, los clusters necesitan ajuste.

El corpus incluye controles negativos y casos Rust, Go, Python, TypeScript,
mutantes CLI, integridad de evidencia, limites de rol, findings y comunicacion.
La ejecucion multi-trial es deliberadamente externa; CI solo valida contratos.
