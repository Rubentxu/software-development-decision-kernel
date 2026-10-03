# archive-manifest — cierre de publicacion de REL-2.5.5

Este fichero es el **indice de cierre** que pide `AGENTS.md` §8. La evidencia
completa vive en [`RECEIPT.md`](RECEIPT.md) del mismo directorio; aqui no se
repite ni se reinterpreta.

- Release: `v2.5.5`
- URL: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v2.5.5
- Publicada: `2026-10-03T22:56:09Z` · `draft=false` · `prerelease=false` · 9/9 assets
- Commit del tag: `886e47cc0e2f4cffcc59054f28e44667ab844f08`
- Pipeline: `bash scripts/release.sh` · recorrido `0`..`13` · **`EXIT=0`**
  (primera corrida completa en verde de este repo; log `/tmp/rel255d.log`)

## Commits publicados en el rango `v2.5.4..v2.5.5`

| Commit | Asunto |
|---|---|
| `0962a79d` | fix(release): el paso 10 volvia a decidir lo que el 8c ya habia decidido |
| `5e339d3e` | chore(release): bump version |
| `304c5d2a` | test(cli): el arreglo de 2026-09-19 quito uno de los dos corredores y dejo el otro |
| `89b470c3` | fix(cli): una linea en blanco de mas rompia el gate fmt del paso 1 |
| `839a3d88` | docs(changelog): el gate 2b encontro el fix de fmt que faltaba |
| `886e47cc` | docs(roadmap): session_74 — el gate que no se ejecuto |

## Estado local final, medido

| | |
|---|---|
| `binary` | `~/.local/bin/sddk` · `sddk 2.5.5` · sha256 `bb753428b56998f75310f77ed964456c5ef7f156673034706f43563434cbaebc` |
| `bundle` | `~/.local/share/sddk/framework/2.5.5/` · 397 ficheros (395 del manifiesto + `MANIFEST.sha256` + `BUNDLE.toml`) |
| `current` | symlink -> `framework/2.5.5` · unica version presente tras `--prune-only --keep 1` |
| `receipt` | este directorio: `RECEIPT.md` + `archive-manifest.md` |
| `doctor` | `binary.bundle_coherence: present` · `binary.build_identity: present` · `all_present: true` |

El sha256 del binario local coincide con el que el paso 8 declaro y con el que
el paso 10 recibio del CDN: **el artefacto publicado, el descargado y el que se
esta ejecutando son el mismo fichero**.

## Estado del ciclo en el ledger

`OPEN` / fase `design`, **sin lease viva** — `sddk cycle artifacts-dir` responde
`no active cycle found for project`, que es como se presenta una lease caducada
y no un ciclo inexistente.

Este ciclo **no se ha archivado**. Archivar es escritura sobre el ledger y la
decision es del operador. Lo que se ha hecho aqui es escritura en disco, que es
el camino que este proyecto usa para sus artefactos (`tests/`).

## Lo que sigue abierto, y no lo tapa este cierre

- **Autenticidad sin verificar.** Publicada con `SDDK_SKIP_SIGNING=1`; los dos
  instaladores exigen `SDDK_ALLOW_UNSIGNED=1`. El ancla de confianza sigue siendo
  el placeholder `@@SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY@@`. **No se fabricara
  una clave ni un ancla.**
- **Los tres artefactos de design que `CURRENT.md` declaraba
  (`exploration-report.md`, `specification.md`, `closeout.md`) nunca se
  escribieron.** No se reconstruyen ahora desde memoria: seria una narracion, no
  una evidencia. Ver `RECEIPT.md` §4.
- `SPEC-012` sin autoridad declarada (7 citas ambiguas de un ID, reportadas por 3h).
- `INC-DEBT-064`: los binarios instalados declaran `source: git`, luego
  `dev build-id --check` es N/A para ellos y **no tiene dientes** hasta que se
  publique una release construida de otra forma.
