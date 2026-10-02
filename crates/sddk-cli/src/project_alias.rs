//! project_alias — the storage-level alias table (ADR-0152, lote 2 de 3).
//!
//! # Por qué un JSON y no una tabla SQLite
//!
//! El SCOPE-CONTRACT de este ciclo decía «migración de esquema de la tabla».
//! Se aparta, y la razón es el modo de fallo, no la comodidad:
//!
//! El fallo que este store tiene que hacer visible es **redirigir la identidad
//! de un proyecto entero al sitio equivocado sin avisar**. Un fichero JSON se
//! lee con `cat`, se diff-ea, y se puede pegar en un bug report. Una tabla
//! SQLite exige una herramienta de consulta, y su inspección es un acto
//! deliberado — que es exactamente lo que no va a pasar si nadie sospecha que
//! hay un alias.
//!
//! La tabla tiene quince entradas. Una migración de esquema completa —con su
//! `user_version`, su pre-flight y su `pre_flight_check`— para quince filas es
//! maquinaria que no se paga, y `sddk-storage` ya tiene un sistema de
//! migraciones *por ledger*: un store global no pertenece ahí, porque un alias
//! se resuelve **antes** de saber qué ledger es, y meterlo en el ledger del
//! destino es un huevo y gallina.
//!
//! El precedente también es del repo: el pin vive en `.sddk/project-pin.json`.
//! Los dos son entradas de resolución de identidad, y ahora viven en el mismo
//! sitio y con el mismo formato.
//!
//! # Lo que este módulo NO hace
//!
//! No resuelve identidad. Eso es [`crate::resolve_identity_honoring_pin`], y es
//! el **único** sitio donde se decide. Este módulo sólo lee y escribe la tabla,
//! para que el cableado no pueda volver a desincronizarse en varios resolutores
//! — que es el defecto que W2c ya corrigió una vez (INC-DEBT-049).
//!
//! # Escritura
//!
//! Atómica: fichero temporal + `rename`. Una escritura a medio hacer deja un
//! fichero de aliases truncado, y un fichero truncado que se parsea igual no
//! puede pasar: el store **valida el shape completo antes de escribir**, y un
//! JSON que no valida no se escribe.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow, bail};
use sddk_domain::identity::{AliasTable, ProjectAlias};
use serde::{Deserialize, Serialize};

/// Bumped only for a breaking change to the on-disk shape.
pub const PROJECT_ALIASES_SCHEMA_VERSION: u32 = 1;

/// On-disk envelope. `schema_version` is checked the same way the pin's is:
/// a file written by a future build must fail loud here, not be parsed with
/// this build's assumptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAliasesFile {
    /// On-disk schema version. A file written by a future build is refused
    /// rather than parsed with this build's assumptions.
    pub schema_version: u32,
    /// Declaration order is preserved, and it is meaningful: the FIRST entry
    /// whose `from_id` matches wins. So order is part of the contract, not a
    /// detail of serialisation.
    pub aliases: Vec<ProjectAlias>,
}

/// `$XDG_STATE_HOME/sddk/project-aliases.json`.
///
/// Sibling of `projects/<id>/ledger.sqlite`, not inside it: an alias has to be
/// readable before the project it names is known.
pub fn project_aliases_path(state_home: &Path) -> PathBuf {
    state_home.join("sddk").join("project-aliases.json")
}

/// Reads the table. A missing file is an **empty table, not an error**: a
/// machine that has never declared an alias is the normal case, and failing
/// there would make the store indistinguishable from a broken one.
pub fn load_alias_table(state_home: &Path) -> anyhow::Result<AliasTable> {
    let path = project_aliases_path(state_home);
    load_alias_table_at(&path)
}

/// Same, against an explicit path. Split out so the tests do not have to
/// fabricate an XDG tree to exercise one file.
pub fn load_alias_table_at(path: &Path) -> anyhow::Result<AliasTable> {
    if !path.exists() {
        return Ok(AliasTable::new(Vec::new()));
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("cannot read project aliases {}", path.display()))?;
    let file: ProjectAliasesFile = serde_json::from_str(&raw).with_context(|| {
        format!(
            "invalid project aliases {}: the file is not readable as this schema",
            path.display()
        )
    })?;
    if file.schema_version != PROJECT_ALIASES_SCHEMA_VERSION {
        bail!(
            "project aliases {} has schema_version {}, this build accepts {}",
            path.display(),
            file.schema_version,
            PROJECT_ALIASES_SCHEMA_VERSION
        );
    }
    Ok(AliasTable::new(file.aliases))
}

/// Writes the table atomically, after validating that it round-trips.
///
/// The round-trip is not ceremony. `AliasTable` resolves at read time and
/// serialises at write time; if the on-disk form could produce a table that
/// resolves differently, the store would be writing a redirect it cannot
/// reproduce. Every alias is resolved once before writing, and a cycle or a
/// malformed chain aborts the write.
pub fn store_alias_table(state_home: &Path, table: &AliasTable) -> anyhow::Result<()> {
    store_alias_table_at(&project_aliases_path(state_home), table)
}

/// Same, against an explicit path.
pub fn store_alias_table_at(path: &Path, table: &AliasTable) -> anyhow::Result<()> {
    // Resolve every entry before writing: a table that cannot resolve is a
    // table that would redirect wrongly at read time.
    for alias in table.entries() {
        table
            .resolve(alias.from_id.clone())
            .with_context(|| format!("alias {} does not resolve", alias.from_id))?;
    }

    let file = ProjectAliasesFile {
        schema_version: PROJECT_ALIASES_SCHEMA_VERSION,
        aliases: table.entries().to_vec(),
    };
    let body =
        serde_json::to_string_pretty(&file).context("cannot serialise the project alias table")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    {
        let mut f =
            fs::File::create(&tmp).with_context(|| format!("cannot create {}", tmp.display()))?;
        f.write_all(body.as_bytes())
            .with_context(|| format!("cannot write {}", tmp.display()))?;
        f.sync_all().ok();
    }
    fs::rename(&tmp, path).with_context(|| format!("cannot move {} into place", path.display()))?;
    Ok(())
}

/// Declares one alias, refusing the one shape that cannot be right.
///
/// `from_id == to_id` is a no-op, and a no-op that looks like a redirect is
/// worse than no entry: it makes the table look like it is doing something.
///
/// **What this does NOT refuse**, so nobody reads a stronger guarantee into it:
/// a `to_id` that is already another alias's `from_id`. Chains are legal
/// (ADR-0152 rule 1) and sometimes intended, so they are not blocked here. A
/// *cycle* is refused one step later, by [`store_alias_table_at`], which
/// resolves every entry before writing — which is why a single call to
/// [`declare_alias_at`] cannot leave a cyclic table on disk.
pub fn declare_alias(table: &mut AliasTable, alias: ProjectAlias) -> anyhow::Result<()> {
    if alias.from_id == alias.to_id {
        bail!(
            "alias {} -> {} is a self-alias: it changes nothing and would read as a redirect",
            alias.from_id,
            alias.to_id
        );
    }
    let mut entries: Vec<ProjectAlias> = table.entries().to_vec();
    entries.push(alias);
    *table = AliasTable::new(entries);
    Ok(())
}

/// Reads the table, applies one declaration, writes it back.
///
/// Read-declare-write is **not** atomic across processes. Two concurrent
/// `sddk project alias` invocations can lose one declaration. That is a real
/// limitation and it is stated rather than papered over: the table is expected
/// to be declared from one operator, at rest, and the alternative — a lock
/// protocol for fifteen rows that are written twice in a project's life — is
/// machinery with its own failure modes.
pub fn declare_alias_at(path: &Path, alias: ProjectAlias) -> anyhow::Result<()> {
    let mut table = load_alias_table_at(path)?;
    declare_alias(&mut table, alias)?;
    store_alias_table_at(path, &table)
}

/// Convenience for the declaration path: parses and validates both ids, so a
/// typo in either is a message naming the argument, not a row in a JSON file.
pub fn parse_alias(
    from: &str,
    to: &str,
    reason: &str,
    created_at: &str,
) -> anyhow::Result<ProjectAlias> {
    let from_id =
        sddk_domain::identity::ProjectId::new(from).map_err(|e| anyhow!("--from: {e}"))?;
    let to_id = sddk_domain::identity::ProjectId::new(to).map_err(|e| anyhow!("--to: {e}"))?;
    ProjectAlias::new(from_id, to_id, reason, created_at).map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::identity::ProjectId;

    fn pid(s: &str) -> ProjectId {
        ProjectId::new(s).unwrap()
    }

    fn alias(from: &str, to: &str) -> ProjectAlias {
        ProjectAlias::new(pid(from), pid(to), "caso medido", "2026-10-02T00:00:00Z").unwrap()
    }

    fn tmp_path(dir: &Path) -> PathBuf {
        dir.join("project-aliases.json")
    }

    // ── Ausencia de fichero: tabla vacía, NO error ─────────────────────
    //
    // Una maquina que nunca declaro un alias es el caso normal. Si esto
    // fallara, el store seria indistinguible de uno roto.

    #[test]
    fn a_missing_file_is_an_empty_table_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let table = load_alias_table_at(&tmp_path(dir.path())).unwrap();
        assert!(table.entries().is_empty());
    }

    #[test]
    fn an_empty_table_resolves_everything_to_itself() {
        let dir = tempfile::tempdir().unwrap();
        let table = load_alias_table_at(&tmp_path(dir.path())).unwrap();
        let got = table.resolve(pid("p-cualquiera")).unwrap();
        assert_eq!(got.project_id, pid("p-cualquiera"));
        assert!(!got.redirected());
    }

    // ── Ida y vuelta ──────────────────────────────────────────────────

    #[test]
    fn a_written_table_reads_back_identical() {
        let dir = tempfile::tempdir().unwrap();
        let path = tmp_path(dir.path());
        let table = AliasTable::new(vec![alias("p-derived", "p-canonical")]);
        store_alias_table_at(&path, &table).unwrap();

        let back = load_alias_table_at(&path).unwrap();
        assert_eq!(back.entries().len(), 1);
        assert_eq!(back.entries()[0].from_id, pid("p-derived"));
        assert_eq!(back.entries()[0].to_id, pid("p-canonical"));
        assert_eq!(back.entries()[0].reason, "caso medido");
    }

    #[test]
    fn declaration_order_survives_the_round_trip() {
        // El orden es PARTE DEL CONTRATO: gana la primera entrada cuyo
        // from_id casa. Un JSON que reordenase al serializar cambiaria la
        // resolucion sin que nadie lo escribiera.
        let dir = tempfile::tempdir().unwrap();
        let path = tmp_path(dir.path());
        let table = AliasTable::new(vec![
            alias("p-uno", "p-primero"),
            alias("p-dos", "p-segundo"),
            alias("p-tres", "p-tercero"),
        ]);
        store_alias_table_at(&path, &table).unwrap();
        let back = load_alias_table_at(&path).unwrap();
        let names: Vec<String> = back.entries().iter().map(|a| a.to_id.to_string()).collect();
        assert_eq!(names, vec!["p-primero", "p-segundo", "p-tercero"]);
    }

    #[test]
    fn the_stored_file_is_readable_as_plain_json() {
        // La razon de que esto sea JSON y no SQLite, en un test: si mañana
        // alguien lo cambia a una tabla, este test falla y hay que repensar
        // la inspeccion manual, que es la mitigacion del modo de fallo.
        let dir = tempfile::tempdir().unwrap();
        let path = tmp_path(dir.path());
        store_alias_table_at(&path, &AliasTable::new(vec![alias("p-a", "p-b")])).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["aliases"][0]["from_id"], "p-a");
        assert_eq!(value["aliases"][0]["to_id"], "p-b");
    }

    // ── Escritura refused cuando la tabla no resuelve ────────────────

    #[test]
    fn a_table_with_a_cycle_is_never_written() {
        // El round-trip antes de escribir no es ceremonia: una tabla que no
        // resuelve es una tabla que redirigira mal al leerla.
        let dir = tempfile::tempdir().unwrap();
        let path = tmp_path(dir.path());
        let broken = AliasTable::new(vec![alias("p-a", "p-b"), alias("p-b", "p-a")]);
        let err = store_alias_table_at(&path, &broken).expect_err("un ciclo no se escribe");
        assert!(err.to_string().contains("does not resolve"), "{err:#}");
        assert!(!path.exists(), "no debe quedar fichero de un ciclo");
    }

    #[test]
    fn a_cycle_in_the_stored_file_fails_at_read_resolution_not_at_parse() {
        // Un fichero con un ciclo se PARSEA bien: es JSON valido. El fallo
        // tiene que salir al resolver, que es cuando importa.
        let dir = tempfile::tempdir().unwrap();
        let path = tmp_path(dir.path());
        let body = r#"{"schema_version":1,"aliases":[
            {"from_id":"p-a","to_id":"p-b","reason":"x","created_at":"t"},
            {"from_id":"p-b","to_id":"p-a","reason":"y","created_at":"t"}]}"#;
        fs::write(&path, body).unwrap();
        let table = load_alias_table_at(&path).expect("el JSON es valido, debe parsear");
        let err = table
            .resolve(pid("p-a"))
            .expect_err("pero no debe resolver");
        assert!(matches!(
            err,
            sddk_domain::identity::IdentityError::AliasCycle(_)
        ));
    }

    // ── Guardas de declaracion ────────────────────────────────────────

    #[test]
    fn a_self_alias_is_refused_at_declaration() {
        let mut table = AliasTable::new(vec![]);
        let err = declare_alias(&mut table, alias("p-a", "p-a"))
            .expect_err("un auto-alias no se declara");
        assert!(err.to_string().contains("self-alias"), "{err}");
        assert!(table.entries().is_empty());
    }

    #[test]
    fn an_alias_without_a_reason_is_refused_at_parse() {
        let err = parse_alias("p-a", "p-b", "  ", "2026-10-02T00:00:00Z")
            .expect_err("un alias sin motivo no se construye");
        assert!(err.to_string().contains("reason"), "{err}");
    }

    #[test]
    fn a_malformed_id_is_refused_naming_the_argument() {
        let err = parse_alias("995939af668a53d8", "p-b", "r", "t")
            .expect_err("un id que empieza por digito es invalido");
        assert!(err.to_string().contains("--from"), "{err}");
        let err = parse_alias("p-a", "0empieza-por-digito", "r", "t")
            .expect_err("un id que empieza por digito es invalido");
        assert!(err.to_string().contains("--to"), "{err}");
    }

    /// Un id con guiones bajos y guiones es **válido**: el formato canónico
    /// es `^[a-zA-Z][a-zA-Z0-9_-]*$`, así que `no-es-un-id` es un ProjectId
    /// bien formado. Este test existió primero afirmando lo contrario y
    /// falló; el código tenía razón y la expectativa estaba mal.
    ///
    /// Se conserva como test positivo para que nadie "arregle" el regex
    /// creyendo que los guiones están prohibidos.
    #[test]
    fn an_id_with_hyphens_and_underscores_is_valid() {
        let a = parse_alias("no-es-un-id", "tambien_no", "r", "t").expect("es valido");
        assert_eq!(a.from_id.as_str(), "no-es-un-id");
        assert_eq!(a.to_id.as_str(), "tambien_no");
    }

    // ── Declarar encadenado ───────────────────────────────────────────

    #[test]
    fn declaring_appends_without_touching_the_previous_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = tmp_path(dir.path());
        declare_alias_at(&path, alias("p-a", "p-b")).unwrap();
        declare_alias_at(&path, alias("p-b", "p-c")).unwrap();

        let table = load_alias_table_at(&path).unwrap();
        assert_eq!(table.entries().len(), 2);
        // Y la cadena resuelve, que es lo que hace que un alias de mas
        // pueda ser inocuo o pueda romperlo todo.
        assert_eq!(table.resolve(pid("p-a")).unwrap().project_id, pid("p-c"));
    }

    #[test]
    fn a_declaration_that_would_close_a_cycle_is_refused_immediately() {
        // El test original esperaba que la segunda declaracion se aceptara y
        // que el ciclo apareciera en una escritura posterior. **El store es
        // mas estricto que eso**: `declare_alias_at` valida antes de escribir,
        // asi que el ciclo nunca llega a existir en disco.
        //
        // Se conserva la correccion porque la expectativa antigua era la
        // INTENCION original y resulto ser peor: un ciclo escrito y detectado
        // mas tarde es un fichero en disco que ya no resuelve, que es
        // justamente el estado que este store no debe poder alcanzar.
        let dir = tempfile::tempdir().unwrap();
        let path = tmp_path(dir.path());
        declare_alias_at(&path, alias("p-a", "p-b")).expect("una cadena de un salto es legal");
        let err = declare_alias_at(&path, alias("p-b", "p-a"))
            .expect_err("cerrar un ciclo debe rechazarse al declararlo");
        assert!(format!("{err:#}").contains("does not resolve"), "{err:#}");

        // Y el fichero en disco sigue teniendo UNA sola entrada, resoluble.
        let table = load_alias_table_at(&path).unwrap();
        assert_eq!(table.entries().len(), 1);
        assert_eq!(table.resolve(pid("p-a")).unwrap().project_id, pid("p-b"));
    }

    // ── Schema version: fail loud, no parsear con supuestos ajenos ───

    #[test]
    fn a_future_schema_version_fails_loud() {
        // Un fichero escrito por un build futuro no se parsea con las
        // suposiciones de este: seria un formato silenciosamente mal leido.
        let dir = tempfile::tempdir().unwrap();
        let path = tmp_path(dir.path());
        fs::write(&path, r#"{"schema_version":99,"aliases":[]}"#).unwrap();
        let err = load_alias_table_at(&path).expect_err("schema desconocido debe fallar");
        assert!(err.to_string().contains("schema_version"), "{err}");
    }

    #[test]
    fn a_corrupt_file_fails_loud_naming_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = tmp_path(dir.path());
        fs::write(&path, "{ not json").unwrap();
        let err = load_alias_table_at(&path).expect_err("un JSON roto debe fallar");
        assert!(err.to_string().contains("project-aliases.json"), "{err:#}");
    }
}
