# Tipos de claim por dominio

Catálogo de `claim_type` específicos por dominio. Se usan para clasificar preguntas en la agenda y para asignar floors de evidencia.

## Tecnología / Software

| claim_type | Descripción | Floor |
|------------|-------------|-------|
| `api-existence` | ¿Existe la API/función/método X? | L1 (docs oficiales, código fuente) |
| `version` | ¿Cuál es la versión actual / cuándo se introdujo? | L1 (release notes, repo) |
| `behavior` | ¿Cómo se comporta X en condición Y? | L1 (código fuente) o L2 (docs) |
| `performance` | ¿Cuál es la latencia/throughput? | L1-exp (benchmark reproducible) |
| `security` | ¿Es vulnerable a X? | L1 (CVE) o L2 (security advisory) |
| `best-practice` | ¿Cuál es la forma recomendada? | L2 (docs oficiales) o L3 (paper secundario) |
| `architectural-pattern` | ¿Cómo se estructura X en producción? | L2 (white paper) o L3 (case study peer-reviewed) |
| `dependency` | ¿Qué versiones de Y requiere X? | L1 (Cargo.toml, package.json, go.mod) |

## IA / Machine Learning

| claim_type | Floor |
|------------|-------|
| `architecture` | L1 (paper del modelo) |
| `training-data` | L1 (paper + dataset card) |
| `benchmark` | L2 (papers que reporten el benchmark reproduciblemente) |
| `limitation` | L1 (paper original) o L2 (peer-reviewed re-test) |
| `safety` | L1 o L2 (papers, guidelines NIST/ISO) |
| `ethical-concern` | L2 (peer-reviewed) + L5 (discusión pública) |
| `cost` | L1 (pricing oficial) |
| `capability` | L1 (paper) o L2 (peer-reviewed) |

## Systems Thinking (Meadows, Forrester, Senge, Kim)

| claim_type | Floor |
|------------|-------|
| `concept-meadows` | L1 (Meadows original) |
| `archetype-structure` | L1 (Senge/Kim) |
| `leverage-rank` | L1 (Meadows original) |
| `feedback-behavior` | L1 (Forrester/Meadows) |
| `world3-model` | L1 (Limits to Growth 1972, World3) |
| `historical-case` | L2 (paper revisado) |
| `policy-resistance` | L1 (Meadows cap. 5) |
| `paradigm-claim` | L1 (Meadows leverage points) |

## Ciencia (biología / química / física)

| claim_type | Floor |
|------------|-------|
| `experimental-result` | L1 (paper peer-reviewed) |
| `theory` | L1 (paper original + replicación) |
| `mechanism` | L1 (paper peer-reviewed) |
| `replication-status` | L1 (resultados replicados) o L1 + L4 (crisis de replicación) |
| `consensus-position` | L1 (review paper, society statement) |

## Medicina

| claim_type | Floor |
|------------|-------|
| `clinical-trial` | L1 (ClinicalTrials.gov + paper) |
| `guideline` | L1 (society guideline peer-reviewed) |
| `contraindication` | L1 (paper + guideline) |
| `mechanism` | L1 (paper peer-reviewed) |
| `side-effect` | L1 (FAERS, paper) |

## Economía / Política

| claim_type | Floor |
|------------|-------|
| `dataset` | L2 (instituto oficial: BLS, INE, Banco Mundial) |
| `policy-impact` | L2 (peer-reviewed) o L1 (paper original) |
| `historical-event` | L1 (archivo) o L2 (historiografía peer-reviewed) |
| `opinion-secondary` | L5 (nunca blocker) |
| `forecast` | L2 (institución con track record) + disclaimer |

## Historia

| claim_type | Floor |
|------------|-------|
| `event-date` | L1 (archivo, documento de época) |
| `primary-source-quote` | L1 (texto original) |
| `interpretation` | L2 (peer-reviewed) o L3 (tesis) |
| `revisionism` | L2 + debate explícito |
| `counterfactual` | L5 + disclaimer explícito (no verificable) |

## Filosofía / Ética

| claim_type | Floor |
|------------|-------|
| `argument` | L1 (texto del filósofo) |
| `school-of-thought` | L2 (Stanford Encyclopedia) |
| `ethical-stance` | L5 (discusión) + L1 cuando aplique |

---

## Anti-patrones

- ❌ Usar `opinion-secondary` (L5) como soporte único para `api-existence` (debe ser L1).
- ❌ Marcar todo como `behavior` cuando la mayoría son `best-practice` (clasifica mal los floors).
- ❌ No distinguir entre `theory` (paper) y `consensus-position` (review).
- ❌ Para datos cuantitativos, mezclar `dataset` (datos) con `opinion-secondary` (interpretación).
