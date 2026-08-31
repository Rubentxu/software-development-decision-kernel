# Spec: Order discount calculator

## Requirement

A `DiscountCalculator` that applies a discount to an order based on the customer's loyalty tier.

## Why this is a golden PW case

The `applyDiscount` method in `DiscountCalculator` makes **4 calls to methods on the `Order` object** (getCustomerId, getItems, getTotal, setDiscount). This is **feature envy** — the method is more interested in `Order` than in its own class. But it's subtle: the logic is correct, the code is clean, and a naive reviewer might not catch it.
