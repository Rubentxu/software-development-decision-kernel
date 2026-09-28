---
id: ADR-0143-RELEASE-SIGNATURE-TRUST-ROOT
status: proposed
supersedes_history: false
adopted_at: null
adoption_cycle: p-63676b11dc0ef88f/c4-release-cut
package_local_id: null
package_source: null
accepted_at: null
accepted_by_cycle: null
superseded_by: []
related_adrs:
  - ADR-0142-RELEASE-SCRIPT-SEMVER-CORRECTNESS
stale_after: null
---

# ADR-0143 — Trust root de la firma de releases (Fulcio keyless vía GitHub Actions OIDC)

- **Status:** proposed
- **Date:** 2026-09-28T08:02:00Z
- **Cycle:** session-23 (cierre técnico previo a la publicación de 2.0.11)
- **Refs:**
  - `docs/debt/INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY.md` (el incidente que esto cierra)
  - `crates/sddk-cli/src/cosign.rs` (fuente única del pinning)
  - `scripts/install.sh` (consumidor shell), `crates/sddk-cli/src/dev/update.rs` (consumidor Rust)
  - `.github/workflows/release.yml` (emisor)

## Nota previa: corrección de mi propio registro

Mi puntero de roadmap (session-22/23) decía que "el trust root de la firma
sigue sin definirse" y listaba el trabajo como bloqueado. **Eso era
incorrecto y esta ADR lo corrige.** El trust root ya está decidido e
implementado desde session-21: Fulcio keyless con issuer
`https://token.actions.githubusercontent.com` y subject anclado a
`release.yml` sobre un tag SemVer. La decisión existe; lo que no existía era
un documento que la gobierne, y eso es lo que esta ADR establece.

Lo que sí estaba realmente abierto era más estrecho, y es lo que la sección
"Lo que esta ADR deja abierto" delimita con precisión.

## Context

SDDK publica sus releases como assets de GitHub Releases y los instala desde
`https://github.com/<repo>/releases/download/<tag>/<asset>`. Antes de session-21,
el `.sha256` que validaba el bundle se descargaba del **mismo** origin path que
el payload: la cadena se auto-certificaba contra sí misma. Integridad sí,
autenticidad no. Eso es `INC-AUDIT-S14`.

La respuesta elegida fue firma keyless de cosign con Fulcio, y desde session-21
está implementada de extremo a extremo. Lo que se decide aquí no es *qué* firma,
sino **qué se acepta como prueba de que la firma es nuestra**, y qué pasa
cuando no se puede comprobar.

## Decision

### 1. La autoridad es el issuer, no una clave

No hay clave privada que conservar, custodiar ni rotar. Fulcio emite un
certificado efímero ligado a la identidad OIDC del workflow, y la prueba
verificable que sobrevive es:

| Elemento | Valor | Por qué |
|---|---|---|
| Issuer | `https://token.actions.githubusercontent.com` | Definido por GitHub, no secreto ni valor del proyecto |
| Subject | `Rubentxu/software-development-decision-kernel:.github/workflows/release\.yml@refs/tags/vX.Y.Z` | Regex, no literal (§3) |

Esta es la decisión con la consecuencia más importante, y conviene enunciarla
sin suavizar: **confiamos en que Fulcio no miente**. Fulcio es un servicio
público de terceros; no hay operación criptográfica nuestra que verificar. La
alternativa — una clave Ed25519 propia, guardada en un secreto de Actions — no
se ha descartado por capricho, sino porque invierte el problema: pasa de
"confiar en un tercero" a "confiar en que el secreto de Actions no se filtra y
en que la rotación se ejecuta". Ninguna de las dos es libre de riesgo; keyless
elige la que no exige custodia humana, a cambio de depender de Fulcio.

### 2. Ambos extremos se fijan, o ninguno

`cosign verify-blob` sin `--certificate-identity` **y** `--certificate-oidc-issuer`
acepta cualquier certificado válido de Sigstore. Eso es una firma real sobre
los bytes reales, de un certificado real de Fulcio, registrado en el log de
transparencia real — y solo prueba que *alguna cuenta de GitHub firmó esto*.

Dado que una cuenta de GitHub es aproximadamente tan fácil de obtener como un
runner de Actions, quien pueda publicar un release también puede acuñar una
firma válida para él. Fijar ambos extremos es lo que convierte el check en
"esto es nuestro artefacto":

- el **repo** — solo el workflow de este proyecto puede firmar;
- el **fichero de workflow** — `ci.yml` o cualquier otro con `id-token: write`
  no produce una firma de confianza;
- **el ref es un tag SemVer**, no una rama. Publicar un tag *es* el acto de
  release; una rama de feature no lo es.

Suministrar uno sin el otro no es "mitad de estricto": cosign rechaza la
llamada, y donde no la rechaza, la mitad sin fijar es la que elige el atacante.

### 3. El subject es una regex, no un literal — y esto ya costó un incidente

Tres candidatos, y solo uno sobrevive:

| Pin | Resultado |
|---|---|
| `@refs/heads/main` | No matchea nada. Toda instalación falla con un error que *parece* sabotaje. Fue el valor enviado en el primer corte. |
| `@refs/tags/v2.0.7` | Matchea ese release y rompe en silencio en 2.0.8 — verde hasta el día que no. Peor. |
| Regex sobre repo + workflow + tag SemVer | Correcto para todo release, sin editar nada |

La lección que se fija como norma: **un pin literal a un tag se rompe en el
siguiente release, y un pin a una rama nunca funcionó**. Un control de
seguridad que depende de que alguien recuerde editarlo es un control que
fallará.

### 4. Fail-closed por defecto, con opt-out explícito y ruidoso

Ambas rutas de consumo (`install.sh` y `dev update`) rechazan por defecto un
artefacto sin firma. Firma presente e inválida aborta. El opt-out es explícito
(`--allow-unsigned` / `SDDK_ALLOW_UNSIGNED*`) y **anuncia lo que hace**.

La asimetría es deliberada: *ausente* y *presente-pero-inválida* nunca se
confunden. Una firma que no verifica es un indicador de compromiso y jamás
debe degradarse a "sin firma, adelante". El caso "sin firma" existe porque
`v2.0.1` se publicó sin firmar, y no porque la ausencia de firma sea aceptable
en general.

### 5. Una firma detached sin `.pem` se rechaza

El CI publica `.sig` + `.pem`; `.bundle.json` es lo que emite cosign actual
con `--bundle`. Los consumidores caían a `verify-blob --signature` **sin**
`--certificate-chain`, donde `--certificate-identity` no tiene contra qué
casar. El control aparentaba ser estricto y era más débil: una firma sin
certificado se aceptaba.

Ahora una firma detached sin `.pem` se rechaza en vez de aceptarse. Un control
que aparenta verificar y no verifica es peor que no tener control: entrena al
operador a esperar un salto que no ocurre.

## Lo que esta ADR deja abierto — y por qué no se puede decidir aquí

### (a) La ventana de publicación firmada: un tag sin assets firmados

Este es el hueco real, y tiene una restricción que **no aparece hasta que se
mira qué emite cosign** y que condiciona las dos opciones.

`release-automation.yml` despacha `release.yml` sobre un tag. Si la rama
`sign` falla a mitad —cosign caído, red, cuota de OIDC agotada—, el tag
**queda publicado sin assets firmados**. Peor: los instaladores están
fail-closed, así que el release existe, es público, es el `latest`, y es
**ininstalable**.

**La restricción: la identidad keyless depende de dónde se firma, y son
mutuamente excluyentes.** El propio `cosign` lo dice en su ayuda, y lo
verifiqué contra el binario instalado (`cosign v3.1.3`, `strings` sobre
`/usr/bin/cosign`):

```
The OIDC issuer expected in a valid Fulcio certificate, e.g.
https://token.actions.githubusercontent.com or https://oauth2.sigstore.dev/auth.
```

| Dónde se firma | Issuer del certificado | Subject resultante |
|---|---|---|
| GitHub Actions | `https://token.actions.githubusercontent.com` | `Rubentxu/…:release.yml@refs/tags/vX.Y.Z` |
| Local, device flow | `https://oauth2.sigstore.dev/auth` | la **identidad de la persona** (email/usuario de Fulcio) |

El pin de `cosign.rs` exige el issuer de GitHub Actions **y** el subject del
workflow. Una firma hecha en local no satisface **ninguno** de los dos: no la
verificaría ni el propio `install.sh` del proyecto, porque debe.

Esto significa que la opción 1, tal como está escrita hoy, **no funciona en
local**: `release.sh` paso 8c firmaría con el issuer equivocado, publicaría, y
la instalación de ese release fallaría contra el pin. No es que falte
intervención humana (el device flow la pide), es que la **identidad sería
otra**, y el resultado sería un release firmado e inverificable por sus propios
consumidores.

Las dos opciones reales, entonces:

1. **Firmar en `release.sh` paso 8c, pero solo cuando se ejecuta dentro de
   GitHub Actions** (opción 1 original, con la condición añadida). Entonces el
   release solo se publica si la firma se obtuvo con la identidad correcta: un
   fallo deja un tag sin release, feo pero honesto. **Requiere** que
   `release.sh` se ejecute en Actions, no en local — lo que cambia cómo se
   publica, no solo cuándo.
2. **Mantener la rama `sign` del workflow** (que ya firma con la identidad
   correcta) y añadir un guard de publicación: si el tag tiene assets sin
   firmar, `latest` no debe apuntar allí. No requiere cambios de firma, pero
   deja un release roto visible en producción durante la ventana de fallo.

La opción 1 es la que prefiero, **con la condición explícita**, porque es la
única que hace imposible por construcción el "publicado pero ininstalable".
Pero ya no es un detalle de implementación que se pueda cerrar escribiendo
código: **cambia quién ejecuta el release**, y eso es política de proyecto.

> **Hueco de implementación — CERRADO en `08639ff` y el pre-check de
> session-25.**
> Cuando esta ADR se escribió, `release.sh` no comprobaba que el issuer del
> certificado que acababa de emitir coincidiera con `DEFAULT_CERT_ISSUER` antes
> de publicar. Ya está cerrado, por las dos vias que importan:
>
> 1. **Pre-check de contexto** (session-25): sin `GITHUB_ACTIONS=true`,
>    `release.sh` aborta antes de firmar, diciendo que la identidad del proyecto
>    no existe en ese host, y remitiendo a `release-automation.yml`. Esto evita
>    además el device flow interactivo, que en un run desatendido es un
>    cuelgue, no un fallo.
> 2. **Gate de issuer fail-closed** (`08639ff`): tras firmar, extrae el issuer
>    del certificado real (del bundle) y hace `die` si no es el esperado, si el
>    certificado falta, o si no se puede leer. Un certificado ilegible es un
>    fallo, nunca un pase.
>
> `SDDK_SKIP_SIGNING=1` sigue siendo la salida deliberada para publicar sin
> firma, asumiendo que los instaladores la rechazarán salvo que el operador lo
> acepte a propósito. `SDDK_ALLOW_LOCAL_SIGNING=1` existe solo para poder
> falsificar el pre-check sin un runner de Actions: **no** es una vía a un
> release bueno, porque el gate de issuer la rechaza igual.

### (a-bis) Lo que sigue abierto tras cerrar el hueco

El código ya no puede publicar un release firmado con la identidad
equivocada. Lo que queda no es ingeniería:

- **Quién ejecuta el release.** La opción 1 (firmar en el paso 8c) solo es
  válida desde GitHub Actions. Publicar desde local ya no es un error
  técnico: es una ruta que el propio script declara no disponible.
- **La elección entre las dos opciones** sigue siendo del operador, porque la
  opción 2 (rama `sign` + guard de publicación) deja un release roto visible
  durante la ventana de fallo, y aceptar eso es política, no código.

### (b) Cuándo puede un usuario aceptar un artefacto sin firma

Existe `--allow-unsigned` y existe el caso "cosign antiguo, certificado
expirado". El criterio actual es binario y no hay política escrita que diga
cuándo usarlo. Esto no se decide sin saber a quién se le pide confianza: un
mirror de air-gapped, un sistema embebido y un portátil de desarrollador
toleran riesgo distinto.

### (c) Rotación: qué pasa cuando el tercero cambia

Esta ADR introduce un trust root con **cero claves que rotar** — que es
precisamente su virtud — y como efecto colateral deja sin respuesta qué pasa
cuando Fulcio rota su raíz, cuando GitHub cambia el emisor del OIDC, o cuando
el repo se transfiere. Es un trade-off asumido: el recovery es "actualizar dos
constantes en `cosign.rs` y sus copias shell", que es exactamente lo que el
contract test ya verifica como sincronizado. La rotación no necesita
procedimiento porque **no hay material de clave que rotar**. Lo que sí necesita
es una señal de que el cambio ocurrió, y esa señal no está definida.

### (d) Verificación de certificados antiguos con el cosign de hoy

Una firma keyless tiene un TTL de certificado corto (horas), pero el `.sig` y
el `.pem` se publican junto al asset y se conservan. Un consumidor que verifique
meses después depende de que cosign valide contra el log de transparencia, no
contra la validez temporal del certificado. Si ese comportamiento cambia, todo
release firmado se rompe a la vez y de golpe. No hay test que lo fije.

No lo resuelvo aquí porque requiere verificarlo con `cosign verify-blob` contra
un release firmado real — y **no hay ningún release firmado que exista
todavía**. Falsificar la política que se acaba de decidir es un paso
obligatorio antes de considerarla verificada.

## Consequences

### Positivas

- `INC-AUDIT-S14` queda con una política de trust root escrita, no solo
  implementada. La opción (a) del INC podrá cerrarse en distribución en el
  momento en que exista un release firmado real.
- La regex del subject hace que publicar releases sucesivos sea una operación
  normal y no una edición que hay que recordar.
- El fail-closed con opt-out ruidoso convierte "no puedo verificar" en un
  estado explícito en vez de un silencio.

### Negativas, asumidas

- **Se confía en Fulcio.** Si el issuer queda comprometido, toda la cadena
  cae. Es la contrapartida de no custodiar claves.
- **El hueco (a) sigue abierto** hasta que el operador elija entre firmar
  antes de publicar o añadir un guard de publicación. Mientras tanto, un tag
  puede existir sin assets firmados y con `latest` apuntando a él.
- **"Firmado" no es "de fiar".** Un release firmado por el workflow correcto
  demuestra que salió del pipeline correcto. El contenido del release sigue
  sin revisión humana. El pinning limita *quién* firma, no *qué* contiene.

## Seguimiento requerido

1. Publicar el **primer release firmado** — paso obligatorio, y decisión del
   operador por ser irreversible sobre un repo público. Solo es viable
   firmado en GitHub Actions por lo de §(a).
2. Intentar verificar su `.pem` con el cosign de hoy. Si falla, §(d) es un bug
   real y la política de TTL necesita revisión antes de publicar más.
3. Cerrar `INC-AUDIT-S14` en distribución solo después de (1) y (2).
4. Decidir (a), (b), (c) y (d), con un ticket para cada uno.
5. Añadir a `release.sh` la comprobación de que el issuer emitido coincide con
   `DEFAULT_CERT_ISSUER` antes de publicar (declarado en §(a), no cerrado).

## Falsability

Esta ADR no declara que el sistema firmado funciona. Declara una política. Para
que pase a `accepted` y se lea como verificada hacen falta tres cosas:

- un release firmado real, publicado por el workflow, con `.sig` y `.pem`;
- `cosign verify-blob` contra ese release, con el pin de `cosign.rs`, en verde;
- la negación de la mutación: cambiar el pin a `@refs/heads/main` y comprobar
  que **la verificación falla**. Ya está cubierto por
  `cosign::tests::rejects_a_branch_ref` y por las 27 comprobaciones de
  `tests/test_install_asset_contract.sh`, falsificadas con 7 mutaciones.

Sin (1) y (2), esta ADR permanece `proposed` y `INC-AUDIT-S14` permanece
abierta en distribución. Se dice explícitamente para que nadie la lea como
verificada.
