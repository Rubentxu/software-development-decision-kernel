//! Proptest: `Budgets` algebraic invariants.
//!
//! Cycle 3 REQ-K3-002 acceptance scenario 5.
//!
//! Properties:
//! 1. **Identity**: `b.consume(&Budgets::zero()) == Ok(b)` (zero is additive identity).
//! 2. **Underflow**: `b.consume(&b + ε)` fails with `Underflow` for any field.
//! 3. **Hard limits**: `Budgets::hard_limits()` is the absolute ceiling —
//!    no field can exceed its hard limit.
//! 4. **Fits-within**: `b.fits_within(c)` is true iff `b ≤ c` component-wise.
//! 5. **Monotonic**: if `b1 ≤ b2` component-wise, then `b1.consume(&s)` and
//!    `b2.consume(&s)` both succeed for small `s` and `b1.remaining ≤ b2.remaining`.
//!
//! 500 iterations per arm.

#![cfg(test)]

use proptest::prelude::*;
use sddk_domain::workflow_ir::{BudgetError, Budgets};

fn arb_budgets() -> impl Strategy<Value = Budgets> {
    (
        0u64..=Budgets::hard_limits().max_wall_ms,
        0u64..=Budgets::hard_limits().max_tokens,
        0u64..=Budgets::hard_limits().max_cost_micros,
        0u64..=Budgets::hard_limits().max_depth,
        0u64..=Budgets::hard_limits().max_nodes,
        proptest::option::of(0u64..=Budgets::hard_limits().max_tokens),
    )
        .prop_map(|(wall, tokens, cost, depth, nodes, remaining)| Budgets {
            max_wall_ms: wall,
            max_tokens: tokens,
            max_cost_micros: cost,
            max_depth: depth,
            max_nodes: nodes,
            remaining_tokens: remaining,
            no_progress_threshold: u32::MAX,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Property 1: `Budgets::zero()` is the additive identity for `consume`.
    #[test]
    fn zero_is_identity(b in arb_budgets()) {
        let result = b.consume(&Budgets::zero());
        prop_assert!(result.is_ok(), "consume(zero) must succeed: {:?}", result);
        let remaining = result.unwrap();
        prop_assert_eq!(remaining.max_wall_ms, b.max_wall_ms);
        prop_assert_eq!(remaining.max_tokens, b.max_tokens);
        prop_assert_eq!(remaining.max_cost_micros, b.max_cost_micros);
        prop_assert_eq!(remaining.max_depth, b.max_depth);
        prop_assert_eq!(remaining.max_nodes, b.max_nodes);
    }

    /// Property 2: `consume(&b_plus_epsilon)` underflows on the overflowing field.
    #[test]
    fn underflow_when_sub_exceeds_self(
        b in arb_budgets(),
        eps_wall in 1u64..=1000,
        eps_tokens in 1u64..=1000,
        eps_cost in 1u64..=1000,
        eps_depth in 0u64..=64,
        eps_nodes in 0u64..=1000,
    ) {
        // Build a "sub" that exceeds `b` on at least one field.
        let sub = Budgets {
            max_wall_ms: b.max_wall_ms.saturating_add(eps_wall),
            max_tokens: b.max_tokens.saturating_add(eps_tokens),
            max_cost_micros: b.max_cost_micros.saturating_add(eps_cost),
            max_depth: b.max_depth.saturating_add(eps_depth),
            max_nodes: b.max_nodes.saturating_add(eps_nodes),
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        };
        let result = b.consume(&sub);
        prop_assert!(
            matches!(result, Err(BudgetError::Underflow { .. })),
            "consume(sub > self) must underflow, got {:?}",
            result
        );
    }

    /// Property 3: `hard_limits()` is the upper bound — no field exceeds it.
    #[test]
    fn hard_limits_is_ceiling(_x in 0..=1u32) {
        let h = Budgets::hard_limits();
        prop_assert!(h.max_wall_ms <= 86_400_000);
        prop_assert!(h.max_tokens <= 100_000_000);
        prop_assert!(h.max_cost_micros <= 1_000_000_000);
        prop_assert!(h.max_depth <= 64);
        prop_assert!(h.max_nodes <= 10_000);
    }

    /// Property 4: `fits_within` is component-wise ≤ for the 5 ceiling fields.
    #[test]
    fn fits_within_is_componentwise_le(b in arb_budgets(), c in arb_budgets()) {
        let expected = b.max_wall_ms <= c.max_wall_ms
            && b.max_tokens <= c.max_tokens
            && b.max_cost_micros <= c.max_cost_micros
            && b.max_depth <= c.max_depth
            && b.max_nodes <= c.max_nodes;
        prop_assert_eq!(b.fits_within(&c), expected);
    }

    /// Property 5: if `b1 ≤ b2` component-wise and `s` is "small enough"
    /// (fits within `b1`), then `b1.remaining ≤ b2.remaining` after
    /// consuming `s`. We construct `s`, `b1`, `b2` in order so that the
    /// relations are always satisfied (no global rejects).
    #[test]
    fn monotonic_consume(
        s_fields in (0u64..=1000, 0u64..=1000, 0u64..=1000, 0u64..=32, 0u64..=512),
        extra_b1 in (0u64..=1000, 0u64..=1000, 0u64..=1000, 0u64..=32, 0u64..=512),
        extra_b2 in (0u64..=1000, 0u64..=1000, 0u64..=1000, 0u64..=32, 0u64..=512),
    ) {
        let h = Budgets::hard_limits();
        let (sw, st, sc, sd, sn) = s_fields;
        let (xw, xt, xc, xd, xn) = extra_b1;
        let (yw, yt, yc, yd, yn) = extra_b2;

        let s = Budgets {
            max_wall_ms: sw.min(h.max_wall_ms),
            max_tokens: st.min(h.max_tokens),
            max_cost_micros: sc.min(h.max_cost_micros),
            max_depth: sd.min(h.max_depth),
            max_nodes: sn.min(h.max_nodes),
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        };
        let b1 = Budgets {
            max_wall_ms: (sw + xw).min(h.max_wall_ms),
            max_tokens: (st + xt).min(h.max_tokens),
            max_cost_micros: (sc + xc).min(h.max_cost_micros),
            max_depth: (sd + xd).min(h.max_depth),
            max_nodes: (sn + xn).min(h.max_nodes),
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        };
        let b2 = Budgets {
            max_wall_ms: (sw + xw + yw).min(h.max_wall_ms),
            max_tokens: (st + xt + yt).min(h.max_tokens),
            max_cost_micros: (sc + xc + yc).min(h.max_cost_micros),
            max_depth: (sd + xd + yd).min(h.max_depth),
            max_nodes: (sn + xn + yn).min(h.max_nodes),
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        };

        // Sanity-check the construction: s ≤ b1 ≤ b2.
        prop_assert!(s.fits_within(&b1));
        prop_assert!(b1.fits_within(&b2));

        let r1 = b1.consume(&s);
        let r2 = b2.consume(&s);
        prop_assert!(r1.is_ok() && r2.is_ok(), "both consumes must succeed: {:?} / {:?}", r1, r2);
        let r1 = r1.unwrap();
        let r2 = r2.unwrap();
        prop_assert!(
            r1.remaining_tokens.unwrap_or(r1.max_tokens) <= r2.remaining_tokens.unwrap_or(r2.max_tokens),
            "monotonicity violated: r1.remaining > r2.remaining"
        );
    }
}

/// Deterministic regression test: consuming `b` by itself returns `Ok(b' == zero)`.
#[test]
fn consume_self_yields_zero_ceilings() {
    let b = Budgets {
        max_wall_ms: 100,
        max_tokens: 50,
        max_cost_micros: 10,
        max_depth: 4,
        max_nodes: 8,
        remaining_tokens: None,
        no_progress_threshold: u32::MAX,
    };
    let result = b.consume(&b).expect("consume(self) must succeed");
    assert_eq!(result.max_wall_ms, 0);
    assert_eq!(result.max_tokens, 0);
    assert_eq!(result.max_cost_micros, 0);
    assert_eq!(result.max_depth, 0);
    assert_eq!(result.max_nodes, 0);
}
