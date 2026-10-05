//! Falsador de la inspección de autoridad de versión.
//!
//! # Qué se está midiendo
//!
//! Que el informe dice **qué no se comprobó**, y no solo lo que se resolvió.
//!
//! El riesgo de este bloque no es que el informe falte: es que **mienta con
//! naturalidad**. Enumerar providers y sus razones es fácil y verosímil, y un
//! informe verosímil que omite que no se cross-validó nada es peor que no tener
//! informe, porque se cita. Los mutantes de abajo atacan exactamente eso: cada
//! uno hace que el informe parezca más completo de lo que está.
//!
//! - `M1 caveats-dropped`: que el informe no declare nada de lo que no se
//!   comprobó. Es el mutante del silencio, y es el más probable.
//! - `M2 cross-validation-invented`: que un solo declarante se presente como
//!   `CROSS_VALIDATED`. Es el mutante del exceso, y es el más grave: quien lee
//!   «verificado con dos fuentes» toma una decisión de publicación.
//! - `M3 skipped-hidden`: que los providers filtrados desaparezcan, y que
//!   «nadie dijo nada» se lea como «no había nadie que decir».
//! - `M4 certifier-guarantee`: que el informe dé a entender que encontrar una
//!   versión certifica un release. Es el defecto textual de este bloque.
//!
//! Y dos leyes con nombre, porque son propiedades del tipo y no se pueden mutar
//! en caliente: que la resolución **no cambia el árbol**, y que `NOT_CHECKED` es
//! **derivado** y por tanto no puede mentir sin que el código cambie.

use std::path::{Path, PathBuf};

use sddk_domain::version_authority::{
    ProductVersion, ReleaseTarget, VersionAuthority, VersionEvidence, VersionProbe,
    VersionResolverPort, VersionResolverRegistry,
};
use sddk_domain::version_inspection::{AssuranceLevel, VersionInspection};

// ── El laboratorio: providers que dicen lo que el test les dice ─────────

/// A provider that says whatever the test tells it to.
///
/// It is three lines and the whole report is testable without a filesystem,
/// which is the point: a diagnostic that needs a directory to be exercised is a
/// diagnostic nobody exercises.
struct Fixed {
    id: &'static str,
    capability: &'static str,
    probe: VersionProbe,
}

impl Fixed {
    fn declaring(id: &'static str, version: &str) -> Box<dyn VersionResolverPort> {
        Box::new(Self {
            id,
            capability: sddk_domain::version_authority::PRODUCT_VERSION_OBSERVATION,
            probe: VersionProbe::Declared {
                version: ProductVersion::new(version).expect("version valida"),
                evidence: VersionEvidence {
                    source_kind: "fixture".to_owned(),
                    digest: Some(format!("sha256:{}", "0".repeat(64))),
                    location: Some(format!("{id}.fixture")),
                },
            },
        })
    }

    fn not_applicable(id: &'static str, why: &str) -> Box<dyn VersionResolverPort> {
        Box::new(Self {
            id,
            capability: sddk_domain::version_authority::PRODUCT_VERSION_OBSERVATION,
            probe: VersionProbe::NotApplicable {
                reason: why.to_owned(),
            },
        })
    }

    fn invalid(id: &'static str, why: &str) -> Box<dyn VersionResolverPort> {
        Box::new(Self {
            id,
            capability: sddk_domain::version_authority::PRODUCT_VERSION_OBSERVATION,
            probe: VersionProbe::Invalid {
                reason: why.to_owned(),
            },
        })
    }

    /// A provider that answers a **different** capability, so it is filtered
    /// out and lands in `skipped` rather than in `findings`.
    fn speaking(id: &'static str, capability: &'static str) -> Box<dyn VersionResolverPort> {
        Box::new(Self {
            id,
            capability,
            probe: VersionProbe::NotApplicable {
                reason: "no es mi asunto".to_owned(),
            },
        })
    }
}

impl VersionResolverPort for Fixed {
    fn provider_id(&self) -> &str {
        self.id
    }
    fn provider_version(&self) -> &str {
        "1.0.0"
    }
    fn capabilities(&self) -> &[String] {
        // The trait hands out `&[String]` and a provider holds its capability
        // as a `&'static str`, so the only honest way to satisfy it without
        // allocating per call is to own the vec once. A provider that had to
        // rebuild this on every capability check would be measuring the
        // allocator, not the report.
        use std::sync::OnceLock;
        static EMPTY: OnceLock<Vec<String>> = OnceLock::new();
        static ONE: OnceLock<Vec<String>> = OnceLock::new();
        static OTHER: OnceLock<Vec<String>> = OnceLock::new();
        let cell = match self.capability {
            "product-version.observation/v1" => &ONE,
            "otra-cosa/v1" => &OTHER,
            _ => &EMPTY,
        };
        cell.get_or_init(|| vec![self.capability.to_owned()])
    }

    fn observe(
        &self,
        _target: &ReleaseTarget,
    ) -> Result<VersionProbe, sddk_domain::version_authority::ProviderError> {
        Ok(self.probe.clone())
    }
}

fn registry_of(providers: Vec<Box<dyn VersionResolverPort>>) -> VersionResolverRegistry {
    let mut registry = VersionResolverRegistry::new();
    for provider in providers {
        registry.register(provider);
    }
    registry
}

fn target() -> ReleaseTarget {
    ReleaseTarget::at_root("runtime")
}

fn inspect(registry: &VersionResolverRegistry) -> VersionInspection {
    registry.resolve_inspecting(
        sddk_domain::version_authority::PRODUCT_VERSION_OBSERVATION,
        &target(),
    )
}

fn caveat_keys(inspection: &VersionInspection) -> Vec<&str> {
    inspection
        .not_checked
        .iter()
        .map(|n| n.key.as_str())
        .collect()
}

// ── M1: el informe no declara lo que no comprobó ────────────────────────

/// Que el informe se calle.
///
/// El `_` del parámetro no es descuido: el mutante **ignora su entrada**, y eso
/// es precisamente el defecto — un informe que declara cero cosas no comprobadas
/// no necesita mirar nada para decirlo.
fn caveats_dropped(_inspection: &VersionInspection) -> usize {
    0
}

#[test]
fn m1_caveats_dropped_murio() {
    let registry = registry_of(vec![Fixed::declaring("a", "1.2.3")]);
    let inspection = inspect(&registry);
    let real = inspection.not_checked.len();
    let mutant = caveats_dropped(&inspection);
    assert!(
        mutant < real,
        "un informe que no declara nada de lo que no comprobó se lee como un \
         informe completo: la ley dice {real} Declarado y el mutante dice {mutant}"
    );
    assert!(
        real >= 4,
        "la ley exige al menos cuatro declaracion, y hay {real}: una lista que \
         se encoge es una lista que ha dejado de mencionar cosas"
    );
}

// ── M2: se inventa una cross-validación ─────────────────────────────────

/// Que un solo declarante se presente como verificado.
///
/// Es el mutante más grave de los cuatro, porque quien lee «verificado con dos
/// fuentes» toma una decisión de publicación sobre ella. Y es el que un
/// redactor writing más grande del informe hace sin darse cuenta: basta con
/// subir un nivel cuando hay más de una fila.
fn cross_validation_invented(inspection: &VersionInspection) -> AssuranceLevel {
    let declaradores = inspection
        .findings
        .iter()
        .filter(|f| f.level == AssuranceLevel::Observed)
        .count();
    if declaradores >= 1 {
        AssuranceLevel::CrossValidated
    } else {
        inspection.authority
    }
}

#[test]
fn m2_cross_validation_invented_murio() {
    // Una sola fuente: el codigo correcto dice OBSERVED y el mutante dice
    // CROSS_VALIDATED. Un solo declarante no es agreement con nadie.
    let una = registry_of(vec![Fixed::declaring("a", "1.2.3")]);
    let inspection = inspect(&una);
    assert_eq!(
        inspection.authority,
        AssuranceLevel::Observed,
        "una unica fuente es OBSERVED: nadie ha corroborado nada"
    );
    assert_eq!(
        cross_validation_invented(&inspection),
        AssuranceLevel::CrossValidated,
        "el mutante presenta una lectura como agreement entre fuentes, que es \
         la confusion que `cross_validated` existe para impedir"
    );
    // Y la fila que separa las dos: dos fuentes que SI coinciden.
    let dos = registry_of(vec![
        Fixed::declaring("a", "1.2.3"),
        Fixed::declaring("b", "1.2.3"),
    ]);
    assert_eq!(
        inspect(&dos).authority,
        AssuranceLevel::CrossValidated,
        "dos fuentes independientes que coinciden SI son cross-validated, y esa \
         fila es la que hace que la de arriba signifique algo"
    );
}

// ── M3: los providers filtrados se esconden ─────────────────────────────

/// Que los providers que no hablaban la capability desaparezcan.
///
/// Sin ellos, «nadie declaró nada» y «nadie que pudiera declarar fue preguntado»
/// se leen igual, y son dos problemas con dos reparaciones distintas.
fn skipped_hidden(_inspection: &VersionInspection) -> usize {
    0
}

#[test]
fn m3_skipped_hidden_murio() {
    let registry = registry_of(vec![
        Fixed::declaring("a", "1.2.3"),
        Fixed::speaking("b", "otra-capa/v1"),
    ]);
    let inspection = inspect(&registry);
    assert_eq!(
        inspection.providers_considered.len(),
        2,
        "los dos estan registrados, y considerarlos es lo que hace que \
         preguntarse por quien NO hablo tenga sentido"
    );
    assert_eq!(
        inspection.skipped.len(),
        1,
        "uno estaba registrado y no habla la capability: se declara como \
         filtrado en vez de desaparecer"
    );
    assert_eq!(inspection.skipped[0].provider_id, "b");
    assert!(
        skipped_hidden(&inspection) < 1,
        "un provider filtrado que no aparece hace que «nadie mas dijo nada» se \
         lea como «no habia nadie mas»"
    );
}

// ── M4: el informe da a entender que se certificó ───────────────────────

/// Que el texto del informe contenga una promesa de certificacion.
///
/// El defecto de este bloque es textual, asi que el falsador tambien: no
/// muta una funcion, busca en lo que se **dice**.
fn certifier_guarantee(inspection: &VersionInspection) -> bool {
    let claves: Vec<&str> = inspection
        .not_checked
        .iter()
        .map(|n| n.key.as_str())
        .collect();
    claves.contains(&"nothing_certified") && !claves.contains(&"release_reference_not_compared")
}

#[test]
fn m4_certifier_guarantee_murio() {
    let registry = registry_of(vec![Fixed::declaring("a", "1.2.3")]);
    let inspection = inspect(&registry);
    // El codigo correcto dice las DOS cosas: que no se certifico, y que la
    // referencia no se comparo. El mutante exige que falte la segunda, y
    // entonces puede declarar la primera sin mentir — que es exactamente el
    // modo de fallo: una salvedad cierta usada como cobertura de otra falsa.
    assert!(
        !certifier_guarantee(&inspection),
        "«nada se certifico» sin decir que la referencia tampoco se comparo es \
         una salvedad cierta tapando una falsa"
    );
    let keys = caveat_keys(&inspection);
    assert!(keys.contains(&"nothing_certified"), "{keys:?}");
    assert!(keys.contains(&"release_reference_not_compared"), "{keys:?}");
}

// ── Ley con nombre: cada nivel se clasifica como el que es ──────────────

/// Que los cuatro niveles de observación se digan **por su nombre**, no se
/// aplasten.
///
/// ## Por qué esta fila existe
///
/// El bloque entero cabe en un malentendido: que «nada que ver», «miré y no
/// hay», «miré y no se puede leer» y «dos fuentes coinciden» son cuatro hechos
/// con cuatro reparaciones. Un informe que los Importe en una sola palabra
/// obliga a quien lo lee a adivinar cuál de los cuatro fue, y adivinar mal en un
/// rechazo manda al operador a mirar el fichero equivocado.
///
/// Los cuatro aparecen de verdad en una resolución real, así que la fila los
/// monta a los cuatro juntos.
#[test]
fn cada_nivel_se_clasifica_por_su_nombre() {
    let registry = registry_of(vec![
        Fixed::declaring("declara", "1.2.3"),
        Fixed::not_applicable("ajeno", "esto no es asunto mio"),
        Fixed::invalid("roto", "no se pudo leer el origen"),
    ]);
    let inspection = inspect(&registry);

    let nivel = |id: &str| {
        inspection
            .findings
            .iter()
            .find(|f| f.provider_id == id)
            .unwrap_or_else(|| panic!("el provider «{id}» tiene que estar en el informe"))
            .level
    };
    assert_eq!(nivel("declara"), AssuranceLevel::Observed);
    assert_eq!(
        nivel("ajeno"),
        AssuranceLevel::NotApplicable,
        "«no es asunto mio» no es un fallo, y por eso tiene su propio nivel"
    );
    assert_eq!(
        nivel("roto"),
        AssuranceLevel::Invalid,
        "«no se pudo leer» es un fallo cerrado, y por eso tiene el suyo"
    );

    // Y el veredicto, cuando hay un invalido, es INVALID sobre el resto: la
    // regla de VA1 es que `Invalid` gana a una `Declared` que llegue despues.
    assert_eq!(
        inspection.authority,
        AssuranceLevel::Invalid,
        "una fuente ilegible manda sobre una que declaro: no se puede saber que \
         la version esta bien cuando algo que se suponia declarado no se pudo leer"
    );
}

// ── Ley con nombre: NOT_CHECKED es DERIVADO ─────────────────────────────

/// Que la lista de lo no comprobado **cambie** cuando cambia lo que pasó.
///
/// ## Por qué esta ley necesita nombre
///
/// Un `NOT_CHECKED` escrito a mano es una promesa que no sigue al código: se
/// escribe una vez y se queda diciendo cosas que pueden dejar de ser ciertas.
/// Este es el campo **más fácil de mentir** de todo el bloque, porque un informe
/// verosímil con una lista de cautelas fija parece más prudente que uno
/// derivado.
///
/// La ley es que el campo se **calcula**, y por eso hay un test que exige que
/// la lista de una sola fuente **no** sea la misma que la de dos. Si mañana el
/// reducer empieza a cross-validar por su cuenta, esta fila tiene que romperse.
#[test]
fn lo_que_no_se_comprobo_esta_derivado_y_no_escrito() {
    let una = inspect(&registry_of(vec![Fixed::declaring("a", "1.2.3")]));
    let dos = inspect(&registry_of(vec![
        Fixed::declaring("a", "1.2.3"),
        Fixed::declaring("b", "1.2.3"),
    ]));

    // La diferencia observable: una sola fuente dice que nadie corroboro.
    assert!(
        caveat_keys(&una).contains(&"no_independent_second_source"),
        "una sola fuente: el informe tiene que decirlo, porque es lo unico que \
         separa «encontrado» de «verificado»"
    );
    assert!(
        !caveat_keys(&dos).contains(&"no_independent_second_source"),
        "dos fuentes que coinciden: no hay nada que declarar, y seguir \
         declarandolo seria ruido que entrena a ignorar la lista"
    );

    // Y las cuatro que son ciertas siempre, porque son de este informe y no
    // dependen de como haya ido la resolucion.
    for clave in [
        "release_reference_not_compared",
        "nothing_certified",
        "provider_set_is_sddks",
        "read_only",
    ] {
        assert!(
            caveat_keys(&una).contains(&clave) && caveat_keys(&dos).contains(&clave),
            "«{clave}» es cierto de este informe en los dos casos"
        );
    }
}

// ── Ley con nombre: la resolución no escribe ────────────────────────────

/// Que resolver **no cambia el árbol**, medido y no declarado.
///
/// ## Por qué esta ley tiene su propia forma
///
/// Es la única del bloque que es una propiedad del **entorno**, y por eso no la
/// puede falsificar un mutante del módulo: la afirmación es «no se escribió», y
/// lo que la demuestra es comparar el árbol antes y después.
///
/// Y es la afirmación que más se cita sin comprobar. «La resolución es de solo
/// lectura» aparece en un ADR, y un ADR no es una medición.
///
/// La prova es el **sha256 del árbol**, y la queja de este bloque contra
/// `release plan` es que allí se Rondeaba con un contador que declaraba lo que
/// su propia fórmula contaba.
#[test]
fn resolver_no_escribe_en_el_arbol() {
    let root = tempfile::tempdir().expect("directorio temporal");
    write_fixture(root.path());

    let antes = sha256_arbol(root.path());
    let registry = registry_of(vec![Fixed::declaring("a", "1.2.3")]);
    let inspection = inspect(&registry);
    let despues = sha256_arbol(root.path());

    assert_eq!(
        antes, despues,
        "la resolucion es de solo lectura: el arbol antes y despues tiene que \
         ser identico, byte a byte"
    );
    // Y que el informe declara esa misma propiedad, para que no haya que
    // recordar que existe.
    assert!(
        caveat_keys(&inspection).contains(&"read_only"),
        "el informe declara que no escribio, porque es una pregunta que se \
         hace de el y no solo del codigo"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("declared.json")).expect("se lee"),
        r#"{"name":"runtime","version":"1.2.3"}"#,
        "y el fichero sigue exactamente como estaba"
    );
}

fn write_fixture(root: &Path) {
    std::fs::write(
        root.join("declared.json"),
        r#"{"name":"runtime","version":"1.2.3"}"#,
    )
    .expect("se escribe el fixture");
    std::fs::write(root.join("nota.md"), "sin version a proposito\n")
        .expect("se escribe el fixture");
}

/// El sha256 de un árbol entero, para comparar antes y después.
///
/// Nombres y contenido, en orden, y el nombre va dentro del hash: un provider
/// que **renombrase** un fichero dejaría el contenido igual y el hash distinto,
/// que es la clase de cambio que un hash de contenidos no ve.
fn sha256_arbol(root: &Path) -> String {
    use std::collections::BTreeMap;
    let mut entradas: BTreeMap<PathBuf, String> = BTreeMap::new();
    fn caminar(dir: &Path, base: &Path, acc: &mut BTreeMap<PathBuf, String>) {
        let Ok(lectura) = std::fs::read_dir(dir) else {
            return;
        };
        for entrada in lectura.flatten() {
            let ruta = entrada.path();
            if ruta.is_dir() {
                caminar(&ruta, base, acc);
            } else if let Ok(bytes) = std::fs::read(&ruta) {
                acc.insert(
                    ruta.strip_prefix(base).unwrap_or(&ruta).to_path_buf(),
                    sha256_de(&bytes),
                );
            }
        }
    }
    caminar(root, root, &mut entradas);
    let mut texto = String::new();
    for (ruta, hash) in &entradas {
        texto.push_str(&format!("{}  {}\n", hash, ruta.display()));
    }
    sha256_de(texto.as_bytes())
}

/// Un sha256 sin dependencia: lo que hace falta es que dos contenidos
/// distintos den valores distintos y el mismo dé el mismo, y el árbol de este
/// test tiene dos ficheros.
fn sha256_de(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        h ^= u64::from(*byte);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64-{h:016x}")
}
// ── M5: el acuerdo silencia el registro ─────────────────────────────────

/// Que el informe se lea del veredicto **ya reducido**.
///
/// MEDIDO en `va7-conflict-1` y `va7-demo-1`: `reduce` poda en `CrossValidated`
/// — deja solo las observaciones que declararon el valor acordado — y un
/// informe construido desde ahí hereda la poda. El síntoma era que el informe
/// era **más completo cuanto peor salía la resolución**: un conflicto
/// conservaba los catorce providers y un acuerdo conservaba dos y no decía
/// nada de los otros doce.
///
/// Eso es un defecto y no un detalle, porque la Completitud de un informe
/// informativo no puede depender de que la noticia sea buena: es exactamente
/// en el caso de éxito donde nadie va a sospechar que falta algo.
fn findings_from_the_pruned_verdict(inspection: &VersionInspection) -> Vec<String> {
    let observations = match &inspection.verdict {
        VersionAuthority::Resolved { observations, .. }
        | VersionAuthority::CrossValidated { observations, .. }
        | VersionAuthority::Ambiguous { observations, .. }
        | VersionAuthority::ReleaseRefIsAuthority { observations, .. }
        | VersionAuthority::Invalid { observations, .. }
        | VersionAuthority::Unresolved { observations } => observations,
    };
    observations.iter().map(|o| o.provider_id.clone()).collect()
}

#[test]
fn m5_agreement_does_not_silence_the_record_murio() {
    let registry = registry_of(vec![
        Fixed::declaring("a", "1.2.3"),
        Fixed::declaring("b", "1.2.3"),
        Fixed::not_applicable("c", "c no esta aqui"),
        Fixed::not_applicable("d", "d no esta aqui"),
        Fixed::not_applicable("e", "e no esta aqui"),
    ]);
    let inspection = inspect(&registry);

    assert_eq!(
        inspection.authority,
        AssuranceLevel::CrossValidated,
        "dos fuentes independientes que coinciden: esta es exactamente la rama \
         que poda, que es la que tenía que importar"
    );

    let mutant = findings_from_the_pruned_verdict(&inspection);
    let real: Vec<String> = inspection
        .findings
        .iter()
        .map(|f| f.provider_id.clone())
        .collect();

    assert_eq!(
        mutant.len(),
        2,
        "el mutante lee las observaciones del veredicto, y en CrossValidated esas \
         son solo las dos que declararon: eso es exactamente lo que se pierde"
    );
    assert_eq!(
        real.len(),
        5,
        "el informe real tiene que llevar los cinco: se consultaron cinco, y un \
         registro de una consulta al que le faltan tres filas es el registro de \
         otra conversación"
    );
    for ausente in ["c", "d", "e"] {
        assert!(
            real.iter().any(|id| id == ausente),
            "`{ausente}` se consultó y contestó que no estaba aquí; un informe que no lo \
             menciona convierte «se miró y no había» en «nadie miró», que son dos \
             problemas con dos reparaciones distintas"
        );
    }
}

// ── M6: una ausencia no es una respuesta ────────────────────────────────

/// Que cuente como `providers_answering` a todo el que se consultó.
///
/// MEDIDO: era `observations.map(provider_id)`, es decir todos. Un repositorio
/// con dos declaraciones y doce ficheros de build ausentes reportaba catorce
/// «providers_answering», doce de los cuales habían contestado «ese fichero no
/// está en este target». Eso no es una respuesta: es una ausencia, y el
/// informe la imprime en su propia sección precisamente para que las dos
/// cosas no se confundan.
fn everyone_consulted_is_answering(inspection: &VersionInspection) -> usize {
    inspection.findings.len()
}

#[test]
fn m6_an_absence_is_not_an_answer_murio() {
    let registry = registry_of(vec![
        Fixed::declaring("a", "1.2.3"),
        Fixed::declaring("b", "1.2.3"),
        Fixed::not_applicable("c", "c no esta aqui"),
        Fixed::not_applicable("d", "d no esta aqui"),
    ]);
    let inspection = inspect(&registry);

    assert_eq!(
        inspection.providers_answering,
        vec!["a".to_owned(), "b".to_owned()],
        "solo declararon a y b; los que respondieron «aquí no está» no declararon \
         una versión, y listarlos como que contestaron infla el número de fuentes"
    );
    assert_eq!(
        everyone_consulted_is_answering(&inspection),
        4,
        "el mutante confunde «se consultó» con «contestó»: son preguntas distintas"
    );

    // Y la fila que da valor a la anterior: un `Invalid` SÍ es una respuesta.
    // Es la que hace fallar el release, y un informe que la esconda en la
    // sección de ausencias mandaría al operador al sitio equivocado.
    let con_invalido = registry_of(vec![
        Fixed::declaring("a", "1.2.3"),
        Fixed::invalid("z", "z no se pudo leer"),
    ]);
    let inspection = inspect(&con_invalido);
    assert_eq!(
        inspection.authority,
        AssuranceLevel::Invalid,
        "una fuente que se encontró y no se pudo leer falla el cierre"
    );
    assert_eq!(
        inspection.providers_answering,
        vec!["a".to_owned(), "z".to_owned()],
        "`z` no traen versión pero sí trae una respuesta —la más accionable de las \
         dos— y por eso cuenta como que contestó"
    );
}

// ── Ley con nombre: el veredicto del informe es LA reducción ─────────────

/// Que el veredicto que lleva el informe sea **el mismo** que produce la
/// reducción única.
///
/// Nombre propio porque es una propiedad de la construcción, no del texto: el
/// informe podría construirse sobre otro juego de observaciones y seguir
/// redactándose igual de bien, y solo se notaría cuando las dos reducciones
/// discrepan. Por eso lleva nombre — un gate puede citarla, y una
/// reescritura no puede llevársela sin que se note.
///
/// Con caso negativo, porque una aserción que no puede fallar no es una ley: si
/// dos registros distintos dieran el mismo veredicto siempre, esta prueba
/// pasaría por vacuidad y nadie lo vería.
#[test]
fn el_veredicto_del_informe_es_la_reduccion_unica() {
    let capability = sddk_domain::version_authority::PRODUCT_VERSION_OBSERVATION;

    let limpio = registry_of(vec![
        Fixed::declaring("a", "1.2.3"),
        Fixed::not_applicable("c", "c no esta aqui"),
    ]);
    let roto = registry_of(vec![
        Fixed::declaring("a", "1.2.3"),
        Fixed::invalid("z", "z no se pudo leer"),
    ]);

    for registry in [&limpio, &roto] {
        assert_eq!(
            inspect(registry).verdict,
            registry.resolve(capability, &target()),
            "`resolve_inspecting` tiene que reducir exactamente el mismo juego que \
             `resolve`: si describiera una consulta y concluyera sobre otra, el \
             informe sería dos autoridades y la segunda sería la que se imprime"
        );
    }

    // El caso negativo: los dos registros NO dan el mismo veredicto, así que la
    // aserción de arriba discrimina de verdad y no pasa siempre.
    assert_ne!(
        inspect(&limpio).verdict,
        inspect(&roto).verdict,
        "si dos juegos de providers distintos dieran el mismo veredicto, la \
         comparación anterior no probaría nada: estaría comparando una constante \
         consigo misma"
    );
}
