//! Acceptance for release targets, measured on real trees.
//!
//! # The four cases the block asks for, and what each one is really about
//!
//! 1. A root with no version and a product one level down resolves. The
//!    question is whether the composition looks at all — or whether it only
//!    ever looks at the root, which is what it did before this block.
//! 2. Two products with different versions resolve **individually**. This is
//!    the case that a monorepo is, and it has to stop being an error.
//! 3. Two products with nothing to tell them apart fails closed. And it has to
//!    fail with the paths in the message, because a refusal the operator
//!    cannot act on is a refusal that gets bypassed.
//! 4. Two observations that disagree **inside the same target** is a
//!    conflict. Which sounds like the previous block, and is the point: after
//!    this one, "disagreement" has a level, and the levels do not mix.
//!
//! Every case here builds a directory tree and uses the production registry.
//! A fixture that used a fake provider would pass with the scan broken.

use sddk_domain::version_authority::{
    PRODUCT_VERSION_OBSERVATION, ProductVersion, ReleaseTarget, TargetSelectionError,
    TargetSelector, VersionAuthority, select_target,
};
use sddk_gateway::version_provider::{
    DEFAULT_TARGET_DEPTH, TargetSet, default_version_registry, release_targets,
};
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn write(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut f = File::create(&path).unwrap();
    writeln!(f, "{body}").unwrap();
}

fn candidates(root: &Path) -> TargetSet {
    release_targets(root, &default_version_registry())
}

fn version_of_root(root: &Path) -> Result<Option<String>, String> {
    let set = candidates(root);
    let selected = select_target(&set.targets, &TargetSelector::unsolicited()).map_err(|e| {
        format!(
            "{e:?} en {} (candidatos: {:?})",
            set.provenance(),
            set.targets
        )
    })?;
    let authority = default_version_registry().resolve(PRODUCT_VERSION_OBSERVATION, selected);
    Ok(authority.version().map(|v| v.to_string()))
}

fn assert_ambiguous(root: &Path) {
    let set = candidates(root);
    let err = select_target(&set.targets, &TargetSelector::unsolicited())
        .expect_err("varios productos y nada que diga cual: tiene que cerrarse");
    assert!(err.is_ambiguous(), "{err:?}");
    assert_eq!(
        err.candidates(),
        ["packages/alpha", "packages/beta"],
        "el error tiene que NOMBRAR los dos, y en orden canonico para que el \
         mismo conjunto de productos de el mismo error en cualquier orden: {err:?}"
    );
    // Y el mensaje de la composicion tiene que decir que se busco y hasta
    // donde, para que un refusal sin contexto sea accionable.
    let provenance = set.provenance();
    assert!(
        provenance.contains("buscaron 2"),
        "la procedencia dice cuantos se encontraron: {provenance}"
    );
    assert!(
        provenance.contains(&format!("hasta {DEFAULT_TARGET_DEPTH} nivel")),
        "la procedencia dice hasta donde se miro: {provenance}"
    );
}

// ── 1. Raiz sin version, producto un nivel por debajo ──────────────────────

/// El caso que pide el bloque: la raiz no declara nada y hay un producto debajo.
///
/// MEDIDO sobre el camino de produccion: antes de este bloque la
/// composicion fabricaba un unico target con la raiz y no miraba mas, luego
/// aqui no habia plan que ver.
#[test]
fn raiz_sin_version_y_producto_debajo_resuelve() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "README.md",
        "un repositorio que no es el producto",
    );
    write(
        d.path(),
        "packages/runtime/package.json",
        r#"{"name":"runtime","version":"0.47.0"}"#,
    );

    let set = candidates(d.path());
    assert!(!set.root_resolved, "la raiz no declara version: {set:?}");
    assert_eq!(
        set.targets.len(),
        1,
        "un solo producto debajo de una raiz muda: {set:?}"
    );
    assert_eq!(set.targets[0].id(), "packages/runtime");
    assert_eq!(
        version_of_root(d.path()).unwrap().as_deref(),
        Some("0.47.0")
    );
}

// ── 2. Dos productos con versiones distintas ───────────────────────────────

/// Un monorepo no es un conflicto. Los dos resuelven, y cada uno con SU
/// version.
#[test]
fn dos_productos_con_versiones_distintas_resuelven_uno_a_uno() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "README.md", "monorepo");
    write(
        d.path(),
        "packages/alpha/package.json",
        r#"{"name":"alpha","version":"1.0.0"}"#,
    );
    write(
        d.path(),
        "packages/beta/pyproject.toml",
        "[project]\nname = \"beta\"\nversion = \"2.0.0\"\n",
    );

    let set = candidates(d.path());
    assert_eq!(
        set.targets.iter().map(|t| t.id()).collect::<Vec<_>>(),
        ["packages/alpha", "packages/beta"],
        "los dos productos estan, en orden canonico: {set:?}"
    );

    let registry = default_version_registry();
    for (id, expected) in [("packages/alpha", "1.0.0"), ("packages/beta", "2.0.0")] {
        let selected = select_target(&set.targets, &TargetSelector::named(id))
            .unwrap_or_else(|e| panic!("{id} esta nombrado: {e:?}"));
        let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, selected);
        assert_eq!(
            authority.version().map(|v| v.to_string()),
            Some(expected.to_owned()),
            "{id} resolvio otra cosa: {authority:?}"
        );
        assert!(!authority.is_failure(), "{id} no esta roto: {authority:?}");
    }
}

/// Y el mismo monorepo con dos productos que SIEMPRE hacen falta ambos nombres:
/// sin selector no hay respuesta, y con selector sí. Es la misma situacion
/// desde los dos lados.
#[test]
fn dos_productos_sin_selector_fallan_y_con_selector_no() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "packages/alpha/package.json",
        r#"{"name":"alpha","version":"1.0.0"}"#,
    );
    write(
        d.path(),
        "packages/beta/package.json",
        r#"{"name":"beta","version":"2.0.0"}"#,
    );
    assert_ambiguous(d.path());

    // Sin selector no hay version, y eso es un rechazo y no un default: la
    // misma llamada con un nombre de target responde, lo que demuestra que el
    // fallo era la falta de un selector y no la incapacidad de resolver.
    let set = candidates(d.path());
    assert!(
        select_target(&set.targets, &TargetSelector::unsolicited()).is_err(),
        "sin selector no hay eleccion"
    );
    let named = select_target(&set.targets, &TargetSelector::named("packages/alpha"))
        .expect("con selector se resuelve");
    let authority = default_version_registry().resolve(PRODUCT_VERSION_OBSERVATION, named);
    assert_eq!(
        authority.version().map(|v| v.to_string()),
        Some("1.0.0".into()),
        "y lo que se resolvio es el target nombrado: {authority:?}"
    );
}

// ── 3. Dos productos y nada que diga cual ──────────────────────────────────

/// El caso que pide el bloque, medido: exit cerrado, con los dos nombrados.
#[test]
fn dos_productos_sin_target_especificado_falla_cerrado() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "packages/alpha/package.json",
        r#"{"name":"alpha","version":"1.0.0"}"#,
    );
    write(
        d.path(),
        "packages/beta/package.json",
        r#"{"name":"beta","version":"2.0.0"}"#,
    );
    assert_ambiguous(d.path());
}

/// Y un nombre que no existe no se resuelve con «el primero que encuentre».
#[test]
fn un_target_que_no_existe_no_cae_a_otro_producto() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "packages/alpha/package.json",
        r#"{"name":"alpha","version":"1.0.0"}"#,
    );
    write(
        d.path(),
        "packages/beta/package.json",
        r#"{"name":"beta","version":"2.0.0"}"#,
    );
    let set = candidates(d.path());
    let err = select_target(&set.targets, &TargetSelector::named("packages/alfa"))
        .expect_err("ese producto no existe");
    assert!(
        matches!(err, TargetSelectionError::UnknownTarget { .. }),
        "{err:?}"
    );
    assert_eq!(
        err.candidates(),
        ["packages/alpha", "packages/beta"],
        "el error lista los dos, que es lo que hace la correccion mecanica: {err:?}"
    );
}

// ── 4. Discrepancia DENTRO de un target ───────────────────────────────────

/// Dos declaraciones que discrepan dentro del MISMO target siguen siendo un
/// conflicto.
///
/// Esto es lo que el nivel da: antes, «dos ficheros que discrepan» y «dos
/// productos» eran la misma pregunta; ahora son dos, y esta no se relaja
/// porque el target este en un subdirectorio.
#[test]
fn una_discrepancia_dentro_del_target_sigue_siendo_conflicto() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "packages/alpha/package.json",
        r#"{"name":"alpha","version":"1.0.0"}"#,
    );
    write(
        d.path(),
        "packages/alpha/pyproject.toml",
        "[project]\nname = \"alpha\"\nversion = \"9.9.9\"\n",
    );
    write(
        d.path(),
        "packages/beta/package.json",
        r#"{"name":"beta","version":"2.0.0"}"#,
    );

    let set = candidates(d.path());
    let alpha =
        select_target(&set.targets, &TargetSelector::named("packages/alpha")).expect("nombrado");
    let authority = default_version_registry().resolve(PRODUCT_VERSION_OBSERVATION, alpha);
    match &authority {
        VersionAuthority::Ambiguous { candidates, .. } => {
            assert_eq!(candidates.len(), 2, "{candidates:?}");
        }
        other => panic!("dos declaraciones que discrepan dentro de alpha: {other:?}"),
    }

    // Y beta, que esta al lado, no se ve afectado. Un conflicto es local.
    let beta =
        select_target(&set.targets, &TargetSelector::named("packages/beta")).expect("nombrado");
    let beta_authority = default_version_registry().resolve(PRODUCT_VERSION_OBSERVATION, beta);
    assert_eq!(
        beta_authority.version().map(|v| v.to_string()),
        Some("2.0.0".into()),
        "la discrepancia de alpha no toca a beta: {beta_authority:?}"
    );
}

// ── La precedencia de la raiz, y por que esta es una decision de ubicacion ──

/// Una raiz que declara su version ES el target, y no se busca debajo.
///
/// La razon por la que esto no es una prioridad prohibida: la prioridad
/// prohibida es entre **respuestas** —dos declaraciones que discrepan, en las
/// que ninguna tecnologia gana— y esto es entre **entidades** —este
/// repositorio y los productos que contiene—, que son preguntas distintas.
/// MEDIDO tambien en la direccion contraria: si se buscara siempre, un
/// repositorio con un producto y un target mas haria indistinguible, para
/// quien lee el plan, el caso de un solo producto del caso de dos.
#[test]
fn una_raiz_que_declara_es_el_target_y_no_se_busca_debajo() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "Cargo.toml",
        "[workspace.package]\nversion = \"1.0.0\"",
    );
    write(
        d.path(),
        "packages/alpha/package.json",
        r#"{"name":"alpha","version":"0.1.0"}"#,
    );

    let set = candidates(d.path());
    assert!(set.root_resolved, "la raiz declara version: {set:?}");
    assert_eq!(set.targets.len(), 1, "y no se buscan los de abajo: {set:?}");
    assert_eq!(set.targets[0].id(), ".");
    assert!(
        set.scan.is_none(),
        "no se escaneo nada, y el plan tiene que poder decirlo: {set:?}"
    );
    assert_eq!(version_of_root(d.path()).unwrap().as_deref(), Some("1.0.0"));
}

/// Y una raiz que declara que su version la lleva la release ref tambien es el
/// target. Si no, un repositorio Go en la raiz con un modulo vendorizado debajo
/// iria a buscar una version de producto que acaba de decir que no tiene.
#[test]
fn una_raiz_que_declara_la_release_ref_tambien_es_el_target() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "go.mod", "module example.com/f\n\ngo 1.22\n");
    write(
        d.path(),
        "packages/vendored/package.json",
        r#"{"name":"vendored","version":"0.0.1"}"#,
    );

    let set = candidates(d.path());
    assert!(
        set.root_resolved,
        "una ausencia declarada tambien es una respuesta: {set:?}"
    );
    assert_eq!(set.targets.len(), 1, "{set:?}");
    assert_eq!(set.targets[0].id(), ".");
}

// ── Lo que el scan no debe encontrar ───────────────────────────────────────

/// Directorios de construccion y copias vendorizadas no son productos.
///
/// Y la direccion del fallo importa: un nombre en la lista de omitidos que en
/// realidad fuera un producto lo haria INVISIBLE, y un producto invisible sale
/// como «no declaro version» —un rechazo que le dice al operador que mire—,
/// nunca como una respuesta equivocada. Por eso la lista vive en el adapter y
/// no en el nucleo: es una opinion sobre la forma del arbol de ficheros, y el
/// nucleo no tiene por que tenerla.
#[test]
fn los_directorios_de_construccion_no_son_productos() {
    let d = tempfile::tempdir().unwrap();
    for skipped in sddk_gateway::version_provider::NON_PRODUCT_DIRECTORIES {
        write(
            d.path(),
            &format!("{skipped}/pkg/package.json"),
            r#"{"name":"not-a-product","version":"0.0.1"}"#,
        );
    }
    write(d.path(), "README.md", "nada que declarar aqui");

    let set = candidates(d.path());
    assert!(
        set.targets.is_empty(),
        "un repositorio con solo directorios de construccion no tiene productos: {set:?}"
    );
    let mut expected_skipped = sddk_gateway::version_provider::NON_PRODUCT_DIRECTORIES
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    expected_skipped.sort();
    assert_eq!(
        set.scan.as_ref().expect("hubo escaneo").skipped,
        expected_skipped,
        "y el escaneo dice que omitio y por que puede importar, en orden canonico"
    );
}

/// Un directorio oculto no es un producto, y no se enumera.
///
/// Sin esta regla, un `.sddk` con algo dentro seria un target mas y haria
/// ambiguo cualquier repositorio.
#[test]
fn los_directorios_ocultos_no_se_enumeran() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        ".cache/pkg/package.json",
        r#"{"name":"x","version":"1.0.0"}"#,
    );
    write(d.path(), "README.md", "nada");
    let set = candidates(d.path());
    assert!(set.targets.is_empty(), "{set:?}");
}

/// La profundidad es un parametro y viaja con la respuesta.
///
/// Un conjunto de targets sin statement de lo que se busco es indistinguible
/// del conjunto de todos los targets que existen, y esas dos afirmaciones son
/// muy distintas.
///
/// Y el numero por defecto no es uno: `packages/alpha` esta **dos** niveles
/// debajo de la raiz, porque `packages` agrupa y no es un producto. Con un solo
/// nivel el escaneo encuentra el directorio agrupador —que no declara nada— y
/// declara que un monorepo con un producto no tiene ninguno. MEDIDO: asi
/// fallaba la primera version de este escaneo, y por eso el numero esta
/// justificado en el sitio donde vive y no aqui solo.
#[test]
fn la_profundidad_es_acotada_y_visible() {
    let d = tempfile::tempdir().unwrap();
    // `uno` esta a un nivel por debajo de la raiz, `uno/dos/tres` a tres.
    write(
        d.path(),
        "uno/package.json",
        r#"{"name":"uno","version":"1.0.0"}"#,
    );
    write(
        d.path(),
        "uno/dos/tres/package.json",
        r#"{"name":"tres","version":"3.0.0"}"#,
    );

    let one = sddk_gateway::version_provider::scan_release_targets(d.path(), 1);
    assert_eq!(one.roots, ["uno"], "{one:?}");
    assert_eq!(one.depth, 1);

    let three = sddk_gateway::version_provider::scan_release_targets(d.path(), 3);
    assert_eq!(three.roots, ["uno", "uno/dos/tres"], "{three:?}");

    // Y el `TargetSet` de la composicion declara la profundidad usada, que es
    // la que hace alcanzable la forma `packages/<nombre>`.
    let set = candidates(d.path());
    assert_eq!(
        set.scan.as_ref().expect("hubo escaneo").depth,
        DEFAULT_TARGET_DEPTH,
        "la profundidad por defecto es un parametro declarado, no una constante invisible"
    );
    assert!(
        set.targets.iter().any(|t| t.id() == "uno"),
        "con la profundidad por defecto, un producto a un nivel se encuentra: {set:?}"
    );
}

/// Un target es su RUTA, no su nombre de directorio.
///
/// Dos productos llamados `index` en carpetas distintas son dos productos, y
/// una identidad que colisiona es un problema distinto de un producto que esta
/// en dos sitios.
#[test]
fn la_identidad_de_un_target_es_su_ruta() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "one/index/package.json",
        r#"{"name":"one","version":"1.0.0"}"#,
    );
    write(
        d.path(),
        "two/index/package.json",
        r#"{"name":"two","version":"2.0.0"}"#,
    );
    let set = candidates(d.path());
    let ids: Vec<&str> = set.targets.iter().map(|t| t.id()).collect();
    assert_eq!(ids, ["one/index", "two/index"], "{ids:?}");

    // Y cada uno abre su propia raiz, no la del repositorio.
    for target in &set.targets {
        let expected = d.path().join(target.id());
        assert_eq!(Path::new(target.root()), expected.as_path(), "{target:?}");
    }
}

/// La identidad se puede pedir por la ruta, y la seleccion es la del dominio.
#[test]
fn un_target_se_pide_por_su_ruta() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "packages/runtime/package.json",
        r#"{"name":"runtime","version":"0.47.0"}"#,
    );
    let set = candidates(d.path());
    let selected = select_target(&set.targets, &TargetSelector::named("packages/runtime"))
        .expect("esta nombrado");
    let authority = default_version_registry().resolve(PRODUCT_VERSION_OBSERVATION, selected);
    assert_eq!(
        authority.version().map(|v| v.to_string()),
        Some(ProductVersion::new("0.47.0").unwrap().to_string()),
        "{authority:?}"
    );
}

/// Un target con la misma identidad construido dos veces sigue siendo una
/// colision, y el dominio lo dice — no el escaneo, que no puede.
#[test]
fn una_colision_de_identidad_la_reporta_el_dominio() {
    let colliding = vec![
        ReleaseTarget::at("runtime", "/repo/packages/a"),
        ReleaseTarget::at("runtime", "/repo/packages/b"),
    ];
    let err = select_target(&colliding, &TargetSelector::named("runtime")).unwrap_err();
    assert!(err.is_ambiguous(), "{err:?}");
}
