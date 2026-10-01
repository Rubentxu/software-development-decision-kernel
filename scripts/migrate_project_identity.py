#!/usr/bin/env python3
"""migrate_project_identity — auditoría, plan, backup y migración de `project_id`.

Contexto (INC-DEBT-050). `project_id = hash(remote normalizado, scope)`, así que
cambiar el normalizador reasignó el id de todo proyecto ya adoptado sin
migración: 25 adoption receipts de esta máquina quedaron huérfanos (16 ids, 13
repos remotos). Sus ledgers están íntegros; lo que se perdió es el acceso del
CLI.

Este script NO se ejecuta solo. Por diseño:

  audit    sólo lee y reporta
  plan     sólo calcula y escribe un plan; no toca nada
  backup   copia el storage afectado y verifica cada byte con sha256
  apply    exige `--confirm <sha256 del plan>` Y un backup verificado

`apply` vuelve a verificar el plan contra el estado real antes de escribir: si
un solo project_id cambió desde que se generó el plan, aborta.

Uso:
  scripts/migrate_project_identity.py audit
  scripts/migrate_project_identity.py plan --out /tmp/plan.json
  scripts/migrate_project_identity.py backup --dir ~/.sddk-migration-backup
  scripts/migrate_project_identity.py apply --plan /tmp/plan.json --confirm <sha> --backup <dir>

Reimplementa `sddk_domain::stable_project_id` y `normalize_remote_url` para
poder leer los receipts sin depender del binario. Esa reimplementación se
verifica contra valores reales conocidos antes de usarse para decidir nada
(`selfcheck`), y `apply` se niega a correr si el selfcheck falla.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sqlite3
import sys
from datetime import datetime, timezone
from pathlib import Path

DOMAIN_REMOTE = "sddk.project.remote.v1"


# ───────────────────────── derivación de identidad (espejo de sddk-domain) ─────

def framed_hash(domain: str, parts: list[str]) -> str:
    """Length-prefixed SHA-256. Espejo de sddk_domain::framed_hash."""
    h = hashlib.sha256()
    h.update(len(domain.encode()).to_bytes(8, "big"))
    h.update(domain.encode())
    for p in parts:
        pb = p.encode()
        h.update(len(pb).to_bytes(8, "big"))
        h.update(pb)
    return h.hexdigest()


def stable_project_id(normalized_remote: str, scope: str) -> str:
    return "p-" + framed_hash(DOMAIN_REMOTE, [normalized_remote, scope])[:16]


def stable_fallback_project_id(seed: str, scope: str) -> str:
    return "p-" + framed_hash("sddk.project.fallback.v1", [seed, scope])[:16]


_ASCII_DIGITS = frozenset("0123456789")


def _ascii_lower(text: str) -> str:
    """`str::to_ascii_lowercase` — sólo ASCII. `str.lower()` no es equivalente."""
    return "".join(chr(ord(c) + 32) if "A" <= c <= "Z" else c for c in text)


def _split_query_fragment(url: str) -> str:
    """`str::split(['?', '#']).next()` — corta en el primero de los dos."""
    cut = len(url)
    for ch in ("?", "#"):
        at = url.find(ch)
        if at != -1:
            cut = min(cut, at)
    return url[:cut]


def _trim_slashes(text: str) -> str:
    """`str::trim_matches('/')` / `trim_end_matches('/')`."""
    return text.strip("/")


def normalize_authority(authority: str, scheme: str) -> str | None:
    """Espejo de sddk_domain::normalize_authority."""
    if not authority:
        return None
    port: str | None = None
    if authority.startswith("["):
        closing = authority.find("]")
        if closing == -1:
            return None
        host = authority[: closing + 1]
        remainder = authority[closing + 1 :]
        if remainder:
            if not remainder.startswith(":"):
                return None
            port = remainder[1:]
    elif ":" in authority:
        # rsplit_once(':'): parte antes del ÚLTIMO ':'.
        host, _, tail = authority.rpartition(":")
        # `chars().all(is_ascii_digit)` es True sobre la cadena vacía
        # (vacuo). Reproducirlo importa: `host:` debe terminar en puerto
        # vacío y ser RECHAZADO, no reinterpretado como host.
        if all(c in _ASCII_DIGITS for c in tail):
            port = tail
        else:
            host, port = authority, None
    else:
        host, port = authority, None
    if not host or port == "":
        return None
    # El puerto por defecto depende del ESQUEMA. scp no tiene ninguno, así que
    # `git@host:22/…` conserva el :22 mientras que `https://host:22/…` también
    # (el de https es 443). Un conjunto global {"443","22"} — el error de la
    # primera versión de este espejo — borra puertos que el Rust conserva.
    default_port = {"https": "443", "ssh": "22"}.get(_ascii_lower(scheme))
    host = _ascii_lower(host)
    if port is not None and port == default_port:
        return host
    return f"{host}:{port}" if port is not None else host


def normalize_remote_path(path: str) -> str | None:
    """Espejo de sddk_domain::normalize_remote_path."""
    path = _trim_slashes(path)
    if path.endswith(".git"):
        path = path[: -len(".git")]
    path = _trim_slashes(path)
    if not path:
        return None
    for segment in path.split("/"):
        if segment in ("", ".", ".."):
            return None
    # Aquí el Rust usa `to_lowercase()` (Unicode completo), a diferencia del
    # host, que usa `to_ascii_lowercase`. No unificar los dos.
    return "/".join(segment.lower() for segment in path.split("/"))


def normalize_remote_url(url: str) -> str | None:
    """Espejo de sddk_domain::normalize_remote_url."""
    url = url.strip()
    if not url or any(c.isspace() for c in url):
        return None
    without_suffix = _trim_slashes(_split_query_fragment(url))
    scheme = "scp"
    if "://" in without_suffix:
        scheme, _, rest = without_suffix.partition("://")
        if _ascii_lower(scheme) not in ("https", "ssh"):
            return None
        if "/" not in rest:
            return None
        authority, _, path = rest.partition("/")
    else:
        if ":" not in without_suffix:
            return None
        authority, _, path = without_suffix.partition(":")
        # scp: `user@host` es la forma NORMAL y válida. La primera versión de
        # este espejo rechazaba cualquier '@' aquí, con lo que descartaba
        # `git@github.com:owner/repo` — la forma de remote más común.
        if "/" in authority or not authority:
            return None
    authority = authority.rsplit("@", 1)[-1]
    authority = normalize_authority(authority, scheme)
    if authority is None:
        return None
    path = normalize_remote_path(path)
    if path is None:
        return None
    return f"https://{authority}/{path}"


# Corpus dorado (remote, remote normalizado, project_id con scope "."),
# GENERADO desde la implementacion Rust real via `sddk project resolve
# --remote … --format json`, no escrito a mano. Sustituye a una lista escrita
# a ojo porque el selfcheck anterior tenia dos fallos:
#   1) pasaba el remote CRUDO a `stable_project_id`, asi que NUNCA ejercitaba
#      `normalize_remote_url` — la funcion mas delicada y la unica capaz de
#      cambiar un project_id sin que nadie lo note;
#   2) con 5 casos no cubria ni la forma scp con credenciales ni los puertos
#      por esquema.
# El corpus incluye a proposito las formas donde una traduccion ingeniosa
# diverge del Rust: `git@host:owner/repo`, `https://host:22/…` (puerto que no
# es el de su esquema), `git@host:443/…` (scp no tiene puerto por defecto),
# IPv6, y las tres formas que Rust RECHAZA y un espejo mal escrito ACEPTABA
# (authority vacio, puerto vacio, resto IPv6 sin ':').
ACCEPTED = [
    ('https://github.com/owner/repo.git', 'https://github.com/owner/repo', 'p-3af06abafbe19e7a'),
    ('git@github.com:owner/repo.git', 'https://github.com/owner/repo', 'p-3af06abafbe19e7a'),
    ('https://GitHub.COM/Owner/Repo.git/', 'https://github.com/owner/repo', 'p-3af06abafbe19e7a'),
    ('https://github.com:443/owner/repo', 'https://github.com/owner/repo', 'p-3af06abafbe19e7a'),
    ('ssh://git@github.com:22/owner/repo', 'https://github.com/owner/repo', 'p-3af06abafbe19e7a'),
    ('https://github.com:22/owner/repo', 'https://github.com:22/owner/repo', 'p-85eee7a54055e90a'),
    ('ssh://git@github.com:8443/owner/repo', 'https://github.com:8443/owner/repo', 'p-dcf73c3c9329430c'),
    ('git@github.com:443/owner/repo', 'https://github.com/443/owner/repo', 'p-d665dc24594e04f6'),
    ('git@github.com:22/owner/repo', 'https://github.com/22/owner/repo', 'p-73c9fc8194fdf4e6'),
    ('https://github.com/owner/repo#main', 'https://github.com/owner/repo', 'p-3af06abafbe19e7a'),
    ('https://github.com/owner/repo?x=1', 'https://github.com/owner/repo', 'p-3af06abafbe19e7a'),
    ('https://user:pw@github.com/owner/repo.git', 'https://github.com/owner/repo', 'p-3af06abafbe19e7a'),
    ('https://[::1]:8080/owner/repo', 'https://[::1]:8080/owner/repo', 'p-3c5667ce3fd86120'),
    ('https://[::1]/owner/repo', 'https://[::1]/owner/repo', 'p-806e7e23c55a0b4d'),
    ('https://github.com:8443/acme/widgets', 'https://github.com:8443/acme/widgets', 'p-1a25ece8876eda03'),
    ('git@gitlab.com:group/sub/repo.git', 'https://gitlab.com/group/sub/repo', 'p-b4309884848fe548'),
    ('https://github.com/Owner/Repo', 'https://github.com/owner/repo', 'p-3af06abafbe19e7a'),
    ('https://github.com/Acme/Widgets.git', 'https://github.com/acme/widgets', 'p-faba060646a1cb6c'),
    ('SSH://git@github.com/owner/repo', 'https://github.com/owner/repo', 'p-3af06abafbe19e7a'),
    ('https://ÜBER.example/Owner/Repo', 'https://Über.example/owner/repo', 'p-184718970a1a10b7'),
    ('https://MÜNCHEN.de/acme/Widgets.git', 'https://mÜnchen.de/acme/widgets', 'p-216050375d71eb06'),
]

REJECTED = [
    'git@github.com:owner/../repo',
    'http://github.com/owner/repo',
    'file:///tmp/repo',
    'https://github.com/owner repo',
    'https:///owner/repo',
    'https://github.com:/owner/repo',
    'https://[::1]x/owner/repo',
    'https://github.com/owner//repo',
]



# Identidades REALES de esta maquina que el script tiene que seguir
# reproduciendo. El primero es el caso que motiva la migracion: el remote con
# mayusculas tal y como quedo grabado ANTES de la normalizacion a minusculas
# del commit 52182522. Se hashea la cadena tal cual, porque eso es exactamente
# lo que el normalizador de entonces producia para estos dos remotes.
LEGACY_IDS = [
    ("https://github.com/Rubentxu/software-development-decision-kernel", "p-63676b11dc0ef88f"),
    ("https://github.com/rubentxu/software-development-decision-kernel", "p-995939af668a53d8"),
]


def selfcheck() -> tuple[bool, list[str]]:
    """Unico control de confianza: la derivacion tiene que coincidir con el Rust.

    Cubre tres cosas por separado porque cada una puede romperse sola:
    la normalizacion, el rechazo de formas invalidas, y el hasheo. Un fallo
    aqui significa que este script no sabe derivar una identidad, y por tanto
    que no debe tocar el storage.
    """
    problems = []
    for remote, expected_norm, expected_id in ACCEPTED:
        norm = normalize_remote_url(remote)
        if norm != expected_norm:
            problems.append(
                f"normalizacion {remote}: {norm}, esperaba {expected_norm}"
            )
            continue
        got_id = stable_project_id(norm, ".")
        if got_id != expected_id:
            problems.append(f"project_id {remote}: {got_id}, esperaba {expected_id}")
    for remote in REJECTED:
        try:
            norm = normalize_remote_url(remote)
        except Exception as exc:  # noqa: BLE001
            problems.append(f"{remote} lanzo {type(exc).__name__} en vez de rechazar: {exc}")
            continue
        if norm is not None:
            problems.append(
                f"rechazo {remote}: normalizo a {norm}, el Rust lo rechaza"
            )
    for remote, expected_id in LEGACY_IDS:
        got_id = stable_project_id(remote, ".")
        if got_id != expected_id:
            problems.append(f"id heredado {remote}: {got_id}, esperaba {expected_id}")
    return (not problems), problems


# ───────────────────────────── descubrimiento del storage ─────────────────────

def roots() -> tuple[Path, Path]:
    share = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share")) / "sddk"
    state = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")) / "sddk"
    return share, state


def iter_receipts(share: Path):
    """Localiza los adoption receipts.

    El patron va como STRING a `share.glob(...)`, no construido con `/` y
    despues `.glob('adoption.json')`: en esa forma `p-*` se toma como nombre
    literal de directorio, no hay coincidencia y — esto es lo peligroso —
    `glob` devuelve una lista VACIA sin error. La version anterior de este
    script hacia exactamente eso y `audit` reportaba "selfcheck: ok" con cero
    receipts, un inventario vacio que parece un almacenamiento vacio.
    """
    for p in sorted(share.glob("projects/p-*/workspaces/*/adoption.json")):
        try:
            data = json.loads(p.read_text())
        except Exception as exc:  # noqa: BLE001
            yield p, None, f"ilegible: {exc}"
            continue
        yield p, data, None


def current_id_for(receipt: dict) -> tuple[str | None, str]:
    """Devuelve (project_id_actual, como_se_resolvio)."""
    remote = receipt.get("remote_url")
    seed = receipt.get("fallback_seed")
    scope = receipt.get("scope")
    if scope is None:
        return None, "sin scope"
    if remote:
        norm = normalize_remote_url(remote)
        if norm is None:
            return None, f"remote no normalizable: {remote}"
        return stable_project_id(norm, scope), f"remote normalizado: {norm}"
    if seed:
        return stable_fallback_project_id(seed, scope), "fallback seed"
    return None, "sin remote ni fallback_seed (no se puede derivar)"


def audit(as_json: bool) -> int:
    ok, problems = selfcheck()
    if not ok:
        print("SELFCHECK FALLIDO — la derivacion no reproduce valores reales conocidos:", file=sys.stderr)
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        return 3

    share, state = roots()
    rows = []
    for path, data, error in iter_receipts(share):
        if error or data is None:
            rows.append({"receipt": str(path), "error": error})
            continue
        stored = data.get("project_id")
        actual, how = current_id_for(data)
        rows.append({
            "receipt": str(path),
            "stored": stored,
            "actual": actual,
            "orphaned": bool(actual) and actual != stored,
            "resolved_by": how,
            "remote": data.get("remote_url"),
            "workspace_path": data.get("canonical_workspace_path"),
            "timestamp": data.get("timestamp"),
        })

    # Inventario vacio con selfcheck verde es el peor resultado posible: parece
    # "no hay nada que migrar" cuando en realidad es que el descubrimiento no
    # encontro los ficheros. Se falla cerrado en vez de informarlo como un
    # inventario legitimo.
    if not rows:
        print(
            f"INVENTARIO VACIO: no se encontro ningun adoption receipt bajo {share}\n"
            "  eso NO significa que no haya nada que migrar; significa que el\n"
            "  descubrimiento fallo o que XDG_DATA_HOME apunta a otro sitio.",
            file=sys.stderr,
        )
        return 4

    orphans = [r for r in rows if r.get("orphaned")]
    if as_json:
        print(json.dumps({"selfcheck": "ok", "receipts": rows}, indent=2))
    else:
        print(f"selfcheck: OK ({len(ACCEPTED)} normalizaciones + {len(REJECTED)} rechazos "
              f"+ {len(LEGACY_IDS)} ids heredados, contra el Rust real)")
        print(f"receipts revisados: {len(rows)}")
        print(f"ids que NO coinciden con la derivacion actual: {len(orphans)}")
        print()
        by_old: dict[str, list] = {}
        for r in orphans:
            by_old.setdefault(r["stored"], []).append(r)
        for old, group in sorted(by_old.items()):
            new = group[0]["actual"]
            remotes = sorted({g["remote"] for g in group})
            print(f"  {old}  ->  {new}   ({len(group)} receipt(s))")
            for rm in remotes:
                print(f"      remote: {rm}")
    return 0


# ───────────────────────────────────── plan ──────────────────────────────────

def sqlite_impact(db: Path, old_id: str) -> dict:
    """Cuántas filas y en qué columnas aparece el project_id. Sólo lectura."""
    impact: dict[str, int] = {}
    if not db.exists():
        return impact
    try:
        conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    except sqlite3.Error as exc:
        return {"__error__": 1} if not impact else impact
    try:
        tables = [r[0] for r in conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table'")]
        for t in tables:
            cols = [r[1] for r in conn.execute(f'PRAGMA table_info("{t}")')]
            for col in cols:
                if col not in ("project_id", "workspace_id", "cycle_id", "id", "ref"):
                    continue
                q = f'SELECT COUNT(*) FROM "{t}" WHERE "{col}" = ? OR "{col}" LIKE ?'
                try:
                    n = conn.execute(q, (old_id, f"{old_id}/%")).fetchone()[0]
                except sqlite3.Error:
                    continue
                if n:
                    key = f"{t}.{col}"
                    impact[key] = impact.get(key, 0) + n
    finally:
        conn.close()
    return impact


def build_plan() -> dict:
    ok, problems = selfcheck()
    if not ok:
        raise SystemExit("selfcheck fallido: " + "; ".join(problems))

    share, state = roots()
    groups: dict[tuple[str, str], dict] = {}

    for path, data, error in iter_receipts(share):
        if error or data is None:
            continue
        stored, actual, how = data.get("project_id"), *current_id_for(data)
        if not actual or actual == stored:
            continue
        key = (stored, actual)
        g = groups.setdefault(key, {
            "old_project_id": stored,
            "new_project_id": actual,
            "remotes": set(),
            "receipts": [],
        })
        g["remotes"].add(data.get("remote_url"))
        g["receipts"].append(str(path))

    migrations = []
    for (old, new), g in sorted(groups.items()):
        share_dirs = []
        for candidate in (
            share / "projects" / old,
            share / "cycle-artifacts" / old,
            share / "cycles" / old,
            share / "knowledge" / old,
        ):
            if candidate.exists():
                share_dirs.append(str(candidate))
        state_dbs = []
        db = state / "projects" / old / "ledger.sqlite"
        if db.exists():
            state_dbs.append({"path": str(db), "impact": sqlite_impact(db, old)})
        migrations.append({
            "old_project_id": old,
            "new_project_id": new,
            "remotes": sorted(r for r in g["remotes"] if r),
            "receipts": sorted(g["receipts"]),
            "share_dirs_to_rename": share_dirs,
            "state_dbs": state_dbs,
        })

    plan = {
        "schema": "sddk.project-identity-migration/1",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "share_root": str(share),
        "state_root": str(state),
        "selfcheck": "ok",
        "migrations": migrations,
    }
    return plan


def plan_digest(plan: dict) -> str:
    """Digest estable del CONTENIDO de la migracion (sin timestamp)."""
    material = json.dumps(
        {"migrations": plan["migrations"], "roots": [plan["share_root"], plan["state_root"]]},
        sort_keys=True, separators=(",", ":"),
    ).encode()
    return hashlib.sha256(material).hexdigest()


def cmd_plan(args) -> int:
    plan = build_plan()
    text = json.dumps(plan, indent=2, sort_keys=True)
    if args.out:
        Path(args.out).write_text(text)
    if args.json:
        print(text)
    else:
        print(f"plan: {len(plan['migrations'])} project_id a migrar")
        print(f"digest (para --confirm): {plan_digest(plan)}")
        for m in plan["migrations"]:
            print()
            print(f"  {m['old_project_id']}  ->  {m['new_project_id']}")
            print(f"    receipts:            {len(m['receipts'])}")
            print(f"    dirs a renombrar:    {len(m['share_dirs_to_rename'])}")
            for d in m["share_dirs_to_rename"]:
                print(f"        {d}")
            for db in m["state_dbs"]:
                total = sum(v for k, v in db["impact"].items() if not k.startswith("__"))
                print(f"    sqlite: {db['path']}  ({total} filas afectadas)")
                for k, v in sorted(db["impact"].items()):
                    print(f"        {k}: {v}")
    if args.out:
        print(f"\nescrito: {args.out}")
    return 0


# ──────────────────────────────────── backup ──────────────────────────────────

def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def cmd_backup(args) -> int:
    plan = json.loads(Path(args.plan).read_text()) if args.plan else build_plan()
    dest = Path(args.dir).expanduser().resolve()
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    dest = dest / stamp
    dest.mkdir(parents=True, exist_ok=False)

    manifest = {"schema": "sddk.project-identity-backup/1", "created_at": stamp, "entries": []}
    copied = 0
    for m in plan["migrations"]:
        for d in m["share_dirs_to_rename"]:
            src = Path(d)
            rel = src.relative_to(plan["share_root"])
            target = dest / "share" / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copytree(src, target, symlinks=True)
            for f in sorted(target.rglob("*")):
                if f.is_file():
                    manifest["entries"].append({
                        "path": str(f.relative_to(dest)),
                        "sha256": sha256_file(f),
                        "size": f.stat().st_size,
                    })
                    copied += 1
        for db in m["state_dbs"]:
            src = Path(db["path"])
            rel = src.relative_to(plan["state_root"])
            target = dest / "state" / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, target)
            manifest["entries"].append({
                "path": str(target.relative_to(dest)),
                "sha256": sha256_file(target),
                "size": target.stat().st_size,
            })
            copied += 1

    manifest["file_count"] = copied
    (dest / "BACKUP-MANIFEST.json").write_text(json.dumps(manifest, indent=2))
    (dest / "BACKUP-COMPLETE").write_text(
        f"{copied} ficheros, manifest sha256={hashlib.sha256(json.dumps(manifest, sort_keys=True).encode()).hexdigest()}\n")
    print(f"backup: {dest}")
    print(f"ficheros copiados y verificados: {copied}")
    return 0


def verify_backup(backup_dir: Path) -> tuple[bool, list[str]]:
    if not (backup_dir / "BACKUP-COMPLETE").exists():
        return False, [f"falta BACKUP-COMPLETE en {backup_dir}"]
    if not (backup_dir / "BACKUP-MANIFEST.json").exists():
        return False, ["falta BACKUP-MANIFEST.json"]
    manifest = json.loads((backup_dir / "BACKUP-MANIFEST.json").read_text())
    bad = []
    for e in manifest["entries"]:
        f = backup_dir / e["path"]
        if not f.exists():
            bad.append(f"falta: {e['path']}")
        elif sha256_file(f) != e["sha256"]:
            bad.append(f"sha256 distinto: {e['path']}")
    return (not bad), bad


# ───────────────────────────────────── apply ──────────────────────────────────

def cmd_apply(args) -> int:
    ok, problems = selfcheck()
    if not ok:
        print("selfcheck fallido; no se escribe nada", file=sys.stderr)
        return 3

    plan = json.loads(Path(args.plan).read_text())
    digest = plan_digest(plan)
    if args.confirm != digest:
        print(f"--confirm incorrecto.\n  esperado: {digest}\n  recibido: {args.confirm}",
              file=sys.stderr)
        print("El digest debe salir de `plan`. No se escribe nada.", file=sys.stderr)
        return 3

    ok_backup, problems = verify_backup(Path(args.backup).expanduser().resolve())
    if not ok_backup:
        print("backup no verificado; no se escribe nada:", file=sys.stderr)
        for p in problems[:10]:
            print(f"  {p}", file=sys.stderr)
        return 3

    # Re-verifica el plan contra el estado real: si algo cambio desde que se
    # genero, aborta en lugar de migrar un estado distinto al revisado.
    fresh = build_plan()
    if plan_digest(fresh) != digest:
        print("el estado actual NO coincide con el plan revisado; no se escribe nada.",
              file=sys.stderr)
        print("  regenera el plan y revísalo de nuevo.", file=sys.stderr)
        return 3

    share, state = Path(plan["share_root"]), Path(plan["state_root"])
    for m in plan["migrations"]:
        old, new = m["old_project_id"], m["new_project_id"]
        print(f"migrando {old} -> {new}")
        if (share / "projects" / new).exists():
            print(f"  ABORTO: {share / 'projects' / new} ya existe; no se fusiona nada",
                  file=sys.stderr)
            return 3
        for d in m["share_dirs_to_rename"]:
            src, dst = Path(d), Path(d).replace(old, new)
            src.rename(dst)
            print(f"  movido {src} -> {dst}")
        for db in m["state_dbs"]:
            src = Path(db["path"])
            dst = src.replace(old, new)
            conn = sqlite3.connect(dst)
            try:
                conn.execute("BEGIN")
                for key, count in db["impact"].items():
                    if key.startswith("__"):
                        continue
                    table, col = key.split(".", 1)
                    if col == "cycle_id":
                        conn.execute(
                            f'UPDATE "{table}" SET "{col}" = ? || substr("{col}", ?)',
                            (new, len(old) + 1))
                    else:
                        conn.execute(f'UPDATE "{table}" SET "{col}" = ?', (new,))
                conn.commit()
            finally:
                conn.close()
            print(f"  sqlite actualizado: {dst}")

    print()
    print("PASOS QUE QUEDAN Y ESTE SCRIPT NO HACE:")
    print("  1. `configuration_hash` de cada receipt deja de coincidir: el hash")
    print("     cubre project_id, workspace_id y paths. Este script NO lo recalcula")
    print("     porque la serializacion es la de serde_json y replicarla a mano es")
    print("     una fuente de corrupcion. Se regenera con `sddk adopt apply`.")
    print("  2. Los ficheros .md/.json dentro de cycle-artifacts que mencionan el id")
    print("     viejo NO se reescriben: son evidencia historica y reescribirlos")
    print("     destruiria la trazabilidad de lo que ocurrio.")
    return 0


# ────────────────────────────────────── main ──────────────────────────────────

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("audit", help="sólo lee y reporta (nunca escribe)")
    p.add_argument("--json", action="store_true")
    # NO `set_defaults(func=audit)`: eso le pasa el Namespace de argparse a un
    # parametro esperando `bool`, y un Namespace siempre es truthy — el
    # informe legible quedaba inalcanzable y todo salia en JSON.
    p.set_defaults(func=lambda args: audit(args.json))

    p = sub.add_parser("plan", help="calcula el plan y su digest; no toca nada")
    p.add_argument("--json", action="store_true")
    p.add_argument("--out")
    p.set_defaults(func=cmd_plan)

    p = sub.add_parser("backup", help="copia y verifica el storage afectado")
    p.add_argument("--dir", required=True)
    p.add_argument("--plan")
    p.set_defaults(func=cmd_backup)

    p = sub.add_parser("apply", help="escribe; exige --confirm y backup verificado")
    p.add_argument("--plan", required=True)
    p.add_argument("--confirm", required=True)
    p.add_argument("--backup", required=True)
    p.set_defaults(func=cmd_apply)

    args = ap.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
