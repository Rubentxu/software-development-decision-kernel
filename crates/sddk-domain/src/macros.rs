//! Compile-time guards for kernel invariants.
//!
//! Provides macros that surface drift early — at build time — rather than
//! via runtime audits or post-mortem debt-reports.

/// Asserts that `$enum` has exactly `$expected` variants AND that the
/// variants listed in the trailing pattern list match them exhaustively.
///
/// Uses two enforcement mechanisms back-to-back:
///
/// 1. A `const`-evaluated counter that compares the literal list length
///    against `$expected`. Catches the "added a variant" case where someone
///    forgets to update the assertion literal.
/// 2. An exhaustive `match` (no wildcard) over an OR-separated pattern list
///    derived from the same tokens. Catches:
///      - adding a variant not listed (non-exhaustive match fails)
///      - removing a variant (literal count drops below expected)
///      - renaming or reshaping a listed variant (match arm fails to bind)
///
/// Stable on all Rust editions ≥ 2018. No nightly features required. Uses
/// only `stringify!` + `const fn` arithmetic + `assert!`, all available in
/// stable Rust.
///
/// # Example
///
/// ```ignore
/// pub enum MyEnum { A, B(u32), C { x: i32 } }
///
/// assert_variant_count_eq!(MyEnum, 3, [
///     MyEnum::A,
///     MyEnum::B(_),
///     MyEnum::C { .. },
/// ]);
/// ```
#[macro_export]
macro_rules! assert_variant_count_eq {
    ($enum:ty, $expected:expr, [$($variant:pat),* $(,)?]) => {
        const _: () = {
            const EXPECTED: usize = $expected;
            const ACTUAL: usize = {
                let mut n: usize = 0;
                $( let _ = stringify!($variant); n += 1; )*
                n
            };
            const PANIC_MSG: &str = concat!(
                "variant count of `", stringify!($enum),
                "` drifted from expected (got ", stringify!(ACTUAL),
                ", want ", stringify!(EXPECTED), ")"
            );
            assert!(ACTUAL == EXPECTED, "{}", PANIC_MSG);

            // Exhaustiveness check: a `match` with no wildcard arm across
            // every listed pattern. Add / rename / reshape any variant and
            // the build fails.
            fn _check_drift(x: $enum) {
                match x {
                    $($variant)|* => {}
                }
            }
        };
    };
}
