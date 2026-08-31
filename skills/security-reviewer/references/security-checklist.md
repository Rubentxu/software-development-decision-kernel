# Checklist de seguridad (exhaustiva)

## Secretos
- [ ] Ningún API key/token/password real (usar `$API_KEY`, `<YOUR_TOKEN>`).
- [ ] No hay `.env` commiteado con valores reales.
- [ ] Los ejemplos de JWT/firmas usan claves de ejemplo claras.

## Permisos y privilegios
- [ ] `chmod` mínimo necesario (evitar `777`).
- [ ] Sin `sudo` salvo justificación explícita.
- [ ] RBAC de Kubernetes con scope mínimo (no `cluster-admin` en ejemplos).
- [ ] Usuarios de BD con permisos mínimos.

## Contenedores
- [ ] Sin `--privileged` salvo explicación.
- [ ] Sin `hostPath`/`hostNetwork` innecesarios.
- [ ] `runAsNonRoot: true` cuando sea posible.
- [ ] readOnlyRootFilesystem cuando aplique.

## Comandos destructivos
- [ ] `rm -rf` con path absoluto explicado y acotado.
- [ ] `dd`, `mkfs`, `fdisk` con aviso `WARNING:` previo.
- [ ] Comandos SQL (`DROP`, `TRUNCATE`, `DELETE` sin WHERE) con aviso.

## Configuración
- [ ] TLS/HTTPS activado en ejemplos de red.
- [ ] CORS acotado (no `*` salvo justificación).
- [ ] Sin modo debug en ejemplos "de producción".
- [ ] Rate-limit / auth en endpoints de ejemplo.

## Supply chain
- [ ] Evitar `curl https://... | bash` sin verificación.
- [ ] Pip/cargo/npm con versiones fijadas o hashes.
- [ ] Imágenes de contenedor con tagdigest o tag concreto (no `:latest` en prod).

## Regla de severidad
- `critical`: secreto real, comando destructivo sin aviso, RCE posible.
- `high`: permisos excesivos, TLS off, CORS `*`.
- `med`: `:latest`, debug on, sin readOnlyRootFilesystem.
