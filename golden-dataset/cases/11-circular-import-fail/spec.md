# Spec: Order processing module

## Requirement

Create an order processing module with:
- `OrderService` that creates orders and notifies inventory
- `InventoryService` that reserves stock and creates back-orders

## Why this is a golden FAIL case

`OrderService` imports `InventoryService` and `InventoryService` imports `OrderService` — a **circular dependency**. This is a CRITICAL coupling finding.
