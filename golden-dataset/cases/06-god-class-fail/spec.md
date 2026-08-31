# Spec: User account management

## Requirement

Create a `UserService` class that handles:
1. User registration (creates account, sends welcome email)
2. Authentication (login, logout, token validation)
3. Profile updates (name, avatar, preferences)
4. Billing (subscription tier, payment method, invoice history)
5. Notifications (email preferences, in-app alerts)

## Why this is a golden FAIL case

This is a **god-class** by construction. A single class with 5 distinct domain concerns, ~350 LOC, ~15 public methods, and ~8 dependencies. Every cluster should catch this.
