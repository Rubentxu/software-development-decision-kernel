# Extension Platform bounded context

## Owns

Pack SDK, provider SPI, capability negotiation, RPC adapters y lifecycle/discovery de servicios opcionales.

## Explicitly does not own

No filtra protobuf/provider internals al dominio.

## Source layout

Domain types: `crates/sddk-domain/src/extension_platform/` (except Shared Kernel -> `shared/`).

Application/use cases: `crates/sddk-engine/src/extension_platform/` where applicable.

Concrete persistence remains in `sddk-storage` adapters/projections.
