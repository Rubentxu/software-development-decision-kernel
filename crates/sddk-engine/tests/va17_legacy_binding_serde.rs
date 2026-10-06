//! ¿Es `#[serde(default)]` load-bearing en `workspace_id`?
//!
//! MEDIDO (session-86, al volver estricto el canario de worktree): la
//! mutacion M2 quita ese `#[serde(default)]` y el comportamiento NO cambia:
//! un binding sin `workspace_id` se sigue leyendo y se sigue rechazando con
//! el mismo error. O sea que M2 era una FALSACION MUERTA — una mutacion que
//! no puede mover el veredicto, y cuya unica razon de existir era dar dientes
//! a una comprobacion que no los tenia.
//!
//! Antes de concluir que el atributo sobra, esto mide POR QUE. La hipotesis es
//! que serde_derive trate `Option<T>` como ausente->`None` por su cuenta, con
//! lo que el atributo solo documenta. Si es asi, quitarlo es cosmetico, y la
//! PRE-FLIGHT deberia decirlo: el codigo afirma que los bindings antiguos
//! siguen siendo legibles, y esa garantia no depende de nada que este ahi.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct ConDefault {
    session: String,
    #[serde(default)]
    workspace_id: Option<String>,
    refs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SinDefault {
    session: String,
    workspace_id: Option<String>,
    refs: Vec<String>,
}

const SIN_WORKSPACE: &str = r#"{"session":"S","refs":[]}"#;

#[test]
fn serde_deriva_option_ausente_a_none_sin_atributo() {
    let con: Result<ConDefault, _> = serde_json::from_str(SIN_WORKSPACE);
    let sin: Result<SinDefault, _> = serde_json::from_str(SIN_WORKSPACE);

    assert!(
        con.is_ok(),
        "con `#[serde(default)]` un binding sin workspace_id deberia leerse"
    );
    // Asercion load-bearing: si serde ya tratarse `Option` como ausente -> None,
    // quitar el atributo no cambia NADA y M2 es decoracion, no falsacion.
    assert!(
        sin.is_ok(),
        "sin `#[serde(default)]` serde deberia rechazar el campo ausente, \
         pero lo acepto: el atributo no es load-bearing"
    );

    assert_eq!(con.unwrap().workspace_id, None);
    assert_eq!(sin.unwrap().workspace_id, None);
}

#[test]
fn un_campo_no_option_si_exige_el_atributo() {
    // CONTROL: el mismo approach sobre un tipo que NO es `Option` SI falla.
    // Sin esta fila, el test de arriba pasaria por la razon equivocada: que
    // `serde_json::from_str` no estaria deserializando nada.
    #[derive(Debug, Deserialize)]
    struct CampoObligatorio {
        session: String,
        revision: String,
    }

    match serde_json::from_str::<CampoObligatorio>(SIN_WORKSPACE) {
        Ok(c) => panic!(
            "un String ausente debe fallar, pero se leyo session={} revision={}",
            c.session, c.revision
        ),
        Err(e) => assert!(
            e.to_string().contains("revision"),
            "el error deberia nombrar el campo que falta, no fallar por otra cosa: {e}"
        ),
    }
}
