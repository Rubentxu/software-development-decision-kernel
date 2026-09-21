# Fuentes primarias, matriz de encaje y protocolo de evaluación

**Estado:** soporte de investigación. Fecha de consulta: 19-09-2026. Los enlaces son referencias de capacidades documentadas, **no** prueba de que estas herramientas estén instaladas, licenciadas o integradas en un entorno SDDK concreto. Verificar versiones, contratos y disponibilidad durante cada spike.

## A. Evidencia externa utilizada

| Tema | Herramienta / fuente primaria | Capacidad documentada y límite para nuestras propuestas |
|---|---|---|
| SUT remoto | [mirrord traffic steal](https://metalbear.co/mirrord/docs/using-mirrord/steal), [configuración](https://metalbear.co/mirrord/docs/reference/configuration/) | Mirror o steal de tráfico; mirror puede duplicar efectos si el local y remoto actúan. |
| SUT remoto | [Telepresence attach](https://telepresence.io/docs/howtos/attach), [modos](https://telepresence.io/docs/concepts/attachments) | Replace/intercept/wiretap/ingest y filtros. Requiere evaluar permisos, tráfico y efecto sobre el workload. |
| Dev remoto | [DevSpace dev](https://www.devspace.sh/docs/configuration/dev/), [sync](https://www.devspace.sh/docs/cli/devspace_sync) | Sync y conexiones para desarrollo; no contiene por sí solo el contrato completo de escenario propuesto. |
| Dependencias locales | [testcontainers-rs](https://github.com/testcontainers/testcontainers-rs) | Entornos de test con contenedores, no es sustituto de un stack remoto compartido. |
| Pipelines | [Dagger](https://docs.dagger.io/getting-started/introduction/) | Composición portable de tareas en contenedores; no implementar otro engine. |
| Toolchains | [mise lock](https://mise.jdx.dev/dev-tools/mise-lock.html) | Resuelve toolchains concretos; EnvLens debe consumir sus declaraciones y metadatos. |
| Selección tests | [cargo-affected](https://github.com/max-sixty/cargo-affected) | Selección por cobertura/diff; sus límites declarados prohíben considerarla prueba global de ausencia de regresión. |
| Ejecución tests | [cargo-nextest](https://nexte.st/) y [pytest-testmon](https://testmon.org/) | Runners/selección por lenguaje; TestAtlas no los reimplementa. |
| Contratos de servicio | [Pact Broker can-i-deploy](https://docs.pact.io/pact_broker/can_i_deploy) | Matriz de pares de versiones; no construir duplicado para un caso solo Pact. |
| Contratos de esquema | [Buf breaking](https://buf.build/docs/breaking/) | Cambios incompatibles en Protobuf con reglas configuradas; no prueba de comportamiento runtime. |
| Contratos API | [Schemathesis](https://schemathesis.readthedocs.io/en/stable/quick-start/) | Tests generativos de API; ContractProbe coordina resultados, no genera un motor equivalente. |
| Inventario | [Syft](https://github.com/anchore/syft) | SBOM de imágenes y fuentes. Un SBOM de build no es el inventario del runtime desplegado. |
| Vulnerabilidades | [OSV-Scanner](https://github.com/google/osv-scanner/blob/main/docs/usage.md), [Grype](https://github.com/anchore/grype) | Hallazgos de vulnerabilidad; la correlación despliegue/alcance requiere evidencias adicionales. |
| Firmas y procedencia | [Sigstore Cosign](https://docs.sigstore.dev/cosign/) | Verificación de artefactos; un digest solo no garantiza autenticidad. |
| Políticas | [Conftest](https://www.conftest.dev/) | Policy-as-code sobre datos estructurados; su salida no equivale a observación del estado remoto si solo evalúa manifiestos. |
| Telemetría | [OpenTelemetry CI/CD](https://opentelemetry.io/docs/specs/semconv/cicd/cicd-spans/) | Convenciones para runs/tareas de pipelines; observabilidad, no autoridad de decisiones de SDDK. |

### Cautelas de investigación

- No se ha probado en esta documentación ningún CLI propuesto ni la instalación de los productos externos.
- Las afirmaciones de valor novedoso son **hipótesis**, no benchmarking. Cada ficha incluye competidor y experimento de falsificación.
- No identificar un fallo de producto a partir de una salida incompleta; ausencia observada está limitada por scope, versión y accesos del productor.
- La ejecución por agente vía shell y la invocación directa del runtime son modalidades de entrada; no deben crear semánticas de evidencia distintas.

## B. Encaje preciso con SDDK actual

Referencia canónica: [arquitectura](../../architecture/README.md), [roadmap A5→A8](../../architecture/a5/A5-CURRENT-ROADMAP.md), [evolutivo de inputs](../2026-09-19-adaptive-inputs-workflows/README.md). La inspección del repositorio en \`d3194ff74abe14025964a9d95a96e5c21dda81ee\` muestra **contratos/reutilización posibles**, no UAT integrado de esta cartera.

| Responsabilidad | Propietario objetivo | Antirrequisito |
|---|---|---|
| Objetivos, agenda, autoridad, decisión, replan, ejecución de workflows | SDDK | Los nuevos CLI no mutan Planning ni deciden permisos de SDDK. |
| Análisis estático | CogniCode vía CodeIntelligencePort A6 cuando el proveedor real esté integrado | No asumir que CC-S0 Fake produce evidencia productiva. |
| Observación de ejecución | Chronos vía A7 cuando exista integración real | No asignar a Chronos scheduling, test selection ni verdad universal. |
| Escenario/condiciones de prueba | ScenarioLab; EnvLens para comparabilidad | Ninguno conserva segunda fuente de verdad de workflows SDDK. |
| Relación cambio→test y resultados | TestAtlas opcional / Verify actual | No declarar PASS por selector ni por código de salida aislado. |
| Compatibilidad | ContractProbe opcional; Pact/Buf/Schemathesis canónicos en sus ámbitos | No crear una matriz rival a Pact si Pact responde toda la pregunta. |
| Supply chain | SupplyScope opcional con scanners externos | No escanear de nuevo lo ya escaneado; no afirmar «no vulnerable» por falta de uso observado. |
| Efectos | Gateway + Verify; EffectCheck externo solo si se justifica | No segundo ledger, segundo Authority ni reintento ciego. |

### Contrato de entrega aconsejado

- **Fuente**: \`producer_name/version\`, artefactos de entrada, refs; quién solicitó y autorizó la operación cuando importe.
- **Base**: identidad de código, dirty state, configuración y ámbito; versión/escenario/entorno; identidades semánticas estables donde existan.
- **Outcome**: éxito operativo, fallo, cancelación, timeout y resultado parcial diferenciados; stderr diagnóstico y salida JSON sin secretos.
- **Evidencia**: refs a resultados completos cuando se necesiten, checksum/algoritmo de identidad y qué afirmación/claim cubren *si se ha validado*.
- **Completitud**: \`complete|partial|unknown\` y dimensiones ausentes; no convertir \`not observed\` en \`not exists\`.
- **Autoridad**: el CLI no construye recibos de «aprobación de SDDK» por sí mismo; un consumidor independiente decide sus gates propios.

No adoptar un «JSON universal» para todos los productos: estos son invariantes del **adaptador de integración**, no exigencia de que todos los CLI almacenen el mismo agregado.

## C. Batería transversal antes de promocionar una herramienta

1. **Doble consumidor**: uso real en SDDK y una CI/automatización independiente, o razón técnica concreta para permitir una primera adopción restringida.
2. **Sin LLM**: el CLI produce un resultado verificable mediante datos/algoritmos/herramientas propias; un LLM puede elegir parámetros e interpretar, pero no redactar el único receipt.
3. **Negativos reales**: herramienta faltante, credencial denegada, timeout, cancelación, truncado, resultados contradictorios, bases stale y entorno incorrecto.
4. **Seguridad de efectos**: pruebas de aislamiento, autenticación, canary de secreto, permiso mínimo, cleanup y operación concurrente cuando corresponda.
5. **Reanudación**: lectura de resultados persistidos sin transcript; no duplicar efectos al recuperar un intento.
6. **Coste**: comparar con composición de herramientas previas en mismo fixture y scope: tiempo de preparación, calidad, acciones manuales, consumo de tokens y falsos informes.
7. **Estado de proyecto**: si la prueba es verde, abrir una propuesta específica por el mecanismo actual; **no modificar automáticamente el roadmap canónico desde este directorio**.

## D. Disposición sobre Jev/TypeSafe

Esta cartera no necesita un proveedor probabilístico para producir sus hechos. Solo estudiar routing semántico de herramientas o sugerencias de experimentos sobre un corpus y baseline determinista **después** de que existan inputs fiables. No consultar Jev para derivar autorización, confirmar efectos o suplir evidencias ausentes.
