---
id: INC-DEBT-022-INSTALLER-ASSET-NAME-404
title: "install.sh pedía un asset que el release no publica: 404 y abort antes de enlazar nada"
status: closed
severity: critical
priority: P1
created: 2026-09-27
discovered_by: session-16 audit (OBSERVED, ejecución real del e2e)
closed_by: session-16
cluster_id: CL-SUPPLY-CHAIN
fingerprint: "installer_requests_unpublished_bare_asset"
---

## Qué era

`scripts/install.sh` (ruta legacy split-asset) descargaba el binario
como `$ASSET`, donde `ASSET="$(detect_asset)"` produce
`sddk-linux-x86_64-musl`:

```bash
download "$(release_url "$ASSET")" "$TMP_DIR/sddk"
```

Ese nombre **nunca fue un asset publicado**. El release publica el
binario pelado como `sddk` (el `basename` del binario compilado) y el
tarball unificado como `sddk-<tag>-sddk-linux-x86_64-musl.tar.gz`.

`$ASSET` es sólo un *componente* del nombre del tarball unificado; se
confundió como nombre de asset suelto.

## Por qué no lo detectó nadie

La ruta legacy es la que se toma cuando **no se puede resolver
`latest` a una versión concreta**, es decir cuando `gh` no está
disponible. Es exactamente el caso del contenedor de test
(`debian:12-slim` sin `gh`) y el de la mayoría de usuarios que no
tienen la CLI de GitHub instalada.

No había ningún test que atara los nombres que el instalador pide con
los que el release publica. El ciclo-46 rediseñó el contrato de assets
y el instalador se quedó atrás sin que nada lo señalara.

## Evidencia (OBSERVED)

```text
$ curl -o /dev/null -w '%{http_code}' \
    .../releases/latest/download/sddk-linux-x86_64-musl
404
$ curl -o /dev/null -w '%{http_code}' \
    .../releases/latest/download/sddk
200
```

Y en contenedor, el log del install:

```text
downloading: .../releases/latest/download/sddk-linux-x86_64-musl
curl: (22) The requested URL returned error: 404
```

`download` aborta con `exit 1` antes de extraer el bundle, antes de
enlazar agents/skills/prompts, antes de `sddk dev doctor`. El usuario
se queda sin nada instalado y con un 404 sin contexto.

## Corrección (session-16)

```diff
-    download "$(release_url "$ASSET")" "$TMP_DIR/sddk"
-    download "$(release_url "$ASSET.sha256")" "$TMP_DIR/sddk.sha256"
+    download "$(release_url "sddk")" "$TMP_DIR/sddk"
+    download "$(release_url "sddk.sha256")" "$TMP_DIR/sddk.sha256"
```

Se añade además `tests/test_install_asset_contract.sh`, que ata
estáticamente los nombres que pide `install.sh` con los que publica
`release.sh`, para que este desfase no pueda volver a introducirse
silenciosamente.

## Verificación

```text
$ bash tests/test_install_asset_contract.sh
install asset contract: all checks passed   (9/9; el 10º check, el musl, sigue RED por INC-DEBT-021)

$ bash scripts/e2e-install.sh --version v2.0.1
VARIANT a: PASS
VARIANT b: PASS   (⚠️ known gap INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY)
VARIANT c: PASS
VARIANT d: PASS
N1: ALL VARIANTS PASS          exit 0
```

Install real end-to-end en contenedor, confirmado por separado:

```text
sddk 2.0.1
bundle: /root/.local/share/sddk/framework/2.0.1
binary.bundle_coherence: present
all_present: true
```

## Nota sobre el exit code del e2e

Durante la auditoría se sospechó que `e2e-install.sh` reportaba `FAIL`
pero salía con 0, lo que enmascararía todo. **Se comprobó y era
falso**: el script hace `return "$failures"` en `run_variant` y
`exit 1` cuando `TOTAL_FAILURES != 0` (líneas 239, 265). El "exit 0"
observado era un artefacto de cómo se invocó el comando en la sonda
(`| tail` con `PIPESTATUS` mal leído), no un defecto del script.

Queda anotado porque la hipótesis era plausible y conviene dejar
registrado que se falsó, para no volver a "descubrirla" como si fuera
nueva.
