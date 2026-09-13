# Cross-project integration contract

1. SDDK, CogniCode y Chronos evolucionan independientemente.
2. SDDK define ports propios; protobuf externos terminan en adapters.
3. CogniCode/Chronos gRPC exponen **casos de uso agregados**, no internals ni MCP 1:1.
4. Capability negotiation es obligatoria y versionada.
5. Toda evidence importada lleva basis/provider/protocol/analyzer versions.
6. Providers pueden estar UNAVAILABLE o DORMANT sin romper SDDK Base.
7. Daemons on-demand son caches de ejecución, nunca authority.
8. MCP permanece disponible para exploración de agentes, pero no es write-path de Knowledge.
9. Alignment consume normalized evidence; nunca DTOs provider-specific.
10. Nueva capability externa no se incorpora automáticamente a SDDK: se añade sólo si un use case/lens/verification plan demuestra valor.
