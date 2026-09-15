// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_lens/probes.rs — A3-S8 / AC7 deterministic source heuristics.
//
// Pure text heuristics (AC6 style) that derive lens observations from source.
// They are deliberately coarse: each emits observations, never verdicts, and
// every probe has a documented negative control in the tests.

use super::types::LensObservation;

/// Count lines containing `needle`.
fn count_lines(src: &str, needle: &str) -> usize {
    src.lines().filter(|l| l.contains(needle)).count()
}

fn has(src: &str, needle: &str) -> bool {
    src.contains(needle)
}

/// Object-oriented lens observations.
///
/// - `EncapsulationPresent`: an `impl` block exists (behaviour is attached).
/// - `AnemicModelDetected`: a `struct` with fields but no `impl` in the file.
/// - `InheritanceOverComposition`: a `dyn ` trait object used as a struct field
///   alongside a concrete nomethod override is out of scope; we only flag an
///   explicit `impl Trait for` pair count above 1 as a coarse proxy.
pub fn probe_oo_observations(source: &str) -> Vec<LensObservation> {
    let mut out = Vec::new();
    let has_struct = has(source, "struct ");
    let has_impl = has(source, "impl ");
    let pub_field_count = source
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("pub ") && (t.ends_with(',') || t.ends_with(";"))
        })
        .count();

    if has_impl {
        out.push(LensObservation::EncapsulationPresent);
    } else if has_struct && (pub_field_count >= 1 || has(source, "{ pub ")) {
        // `pub_field_count` catches multi-line field declarations; the
        // `{ pub ` scan catches single-line structs whose fields are public.
        out.push(LensObservation::AnemicModelDetected);
    }
    if count_lines(source, "impl ") >= 2 && has(source, " for ") {
        out.push(LensObservation::InheritanceOverComposition);
    }
    out
}

/// Functional / pure-functional lens observations.
///
/// - `HiddenMutationDetected`: `&mut self`, `let mut `, `Cell<`, `RefCell<`,
///   `Mutex<` or `Atomic*`.
/// - `ImmutableValuesPresent`: no hidden-mutation marker at all.
pub fn probe_functional_observations(source: &str) -> Vec<LensObservation> {
    let mut out = Vec::new();
    let markers = [
        "&mut self",
        "let mut ",
        "Cell<",
        "RefCell<",
        "Mutex<",
        "AtomicU",
    ];
    let hidden = markers.iter().any(|m| has(source, m));
    if hidden {
        out.push(LensObservation::HiddenMutationDetected);
    } else {
        out.push(LensObservation::ImmutableValuesPresent);
    }
    if has(source, "String") && has(source, "match ") && !has(source, "enum ") {
        out.push(LensObservation::SentinelControlFlowDetected);
    }
    out
}

/// ADT lens observations.
///
/// - `StringlyTypedStatus`: a `status`/`kind`/`state` field typed `String`.
/// - `InvalidStatesRepresentable`: two or more `Option<...>` fields in one file.
/// - `BooleanBlindness`: a `: bool` field.
pub fn probe_adt_observations(source: &str) -> Vec<LensObservation> {
    let mut out = Vec::new();

    let stringly = source.lines().any(|l| {
        let t = l.trim().trim_end_matches(',');
        let lowercase = t.to_ascii_lowercase();
        (lowercase.starts_with("status: string")
            || lowercase.starts_with("kind: string")
            || lowercase.starts_with("state: string")
            || lowercase.starts_with("pub status: string")
            || lowercase.starts_with("pub kind: string")
            || lowercase.starts_with("pub state: string"))
            || (lowercase.contains("status")
                && lowercase.contains("string")
                && lowercase.contains(':'))
    });
    if stringly {
        out.push(LensObservation::StringlyTypedStatus);
    }

    let optional_fields = count_lines(source, "Option<");
    if optional_fields >= 2 {
        out.push(LensObservation::InvalidStatesRepresentable);
    }

    if count_lines(source, ": bool") >= 1 {
        out.push(LensObservation::BooleanBlindness);
    }

    if has(source, "enum ") && !out.contains(&LensObservation::InvalidStatesRepresentable) {
        out.push(LensObservation::TypedSumTypePresent);
    }
    out
}

/// DSL lens observations.
///
/// - `TypedAstPresent`: an `enum ...Ast` / `struct ...Ir` / `enum ...Ir`.
/// - `ValidatesBeforeExecution`: `fn validate(` appears before `fn execute(`.
/// - `InvalidProgramsRepresentable`: no typed AST/IR was found.
pub fn probe_dsl_observations(source: &str) -> Vec<LensObservation> {
    let mut out = Vec::new();

    let typed_model = source.lines().any(|l| {
        let t = l.trim();
        (t.contains("enum ") || t.contains("struct "))
            && (t.contains("Ast") || t.contains("AST") || t.contains("Ir") || t.contains("IR"))
    });
    if typed_model {
        out.push(LensObservation::TypedAstPresent);
    } else {
        out.push(LensObservation::InvalidProgramsRepresentable);
    }

    let validate_at = source.find("fn validate(");
    let execute_at = source.find("fn execute(");
    if let (Some(v), Some(e)) = (validate_at, execute_at)
        && v < e
    {
        out.push(LensObservation::ValidatesBeforeExecution);
    }

    // A typed model with a separate compile step is the separation signal.
    if typed_model && has(source, "fn compile(") {
        out.push(LensObservation::SyntaxSeparatedFromEffects);
    }
    out
}
