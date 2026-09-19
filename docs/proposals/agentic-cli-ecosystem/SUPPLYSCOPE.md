# SupplyScope — presencia contextual de componentes en artefactos y entornos

**Estado:** candidato de CLI Rust de correlación, sujeto a demostrar valor frente a scanner + inventario de despliegues existente. **Responsabilidad única:** relacionar componente/hallazgo → artefacto de build → digest de imagen → versión desplegada y evidencia de utilización **sin** producir vulnerabilidades por cuenta propia.

## 1. Pregunta de producto

«Una dependencia tiene un hallazgo: ¿está en una imagen concreta, cuál de nuestros servicios ejecuta **ese digest** y qué evidencia falta para evaluar el impacto?» Un lockfile modificado no prueba que se haya reconstruido y desplegado la imagen; una imagen escaneada no prueba por sí sola cuál es la imagen activa de todos los pods.

**Pensamiento lateral:** investigar las *rutas de propagación* desde el componente hasta el entorno, conservando una cadena de identidades verificables y los cortes donde la cadena queda desconocida.

## 2. Ecosistema existente

- [Syft](https://github.com/anchore/syft) genera SBOM de imágenes/directorios y formatos de intercambio como SPDX y CycloneDX. No producir otro inventariador.
- [OSV-Scanner](https://github.com/google/osv-scanner/blob/main/docs/usage.md) y [Grype](https://github.com/anchore/grype) analizan vulnerabilidades en dependencias, imágenes o SBOM según su alcance. No crear base CVE duplicada.
- [Cosign](https://docs.sigstore.dev/cosign/) firma/verifica artefactos y attestations. No derivar autenticidad de un digest sin verificación criptográfica cuando el contrato la exige.
- Kubernetes/Argo CD/registries y las herramientas de despliegue aportan identidades declaradas y observadas; distinguir desired state de workloads realmente activos.

La correlación multi-fuente podría ser valiosa incluso cuando el análisis de alcanzabilidad (p. ej. con CogniCode) sea parcial o no esté disponible; **ausencia de una llamada observada no prueba ausencia de explotabilidad**.

## 3. MVP estrecho

Un proyecto, una imagen OCI fijada por digest, un SBOM producido por Syft, un hallazgo fijado y un entorno de Kubernetes de lectura autorizada. Conectar \`component identity\` → \`SBOM subject\` → \`image digest\` → \`workload pod imageID\`; registrar el estado deseado por separado, si se consulta. No escanear clusters enteros ni aplicar remedios.

### Resultado candidato

\`input_basis\` con scanner/version y fecha de feed, \`artifact_digests[]\`, \`deployed_instances[]\` por UID/namespace/imagen efectiva, \`vulnerability_observations[]\` refs, \`coverage\`, \`unknowns\`, \`source_attestations[]\` verificadas o no verificadas. El algoritmo de digest y la identidad de paquete (purl cuando corresponda) deben quedar explícitos. Un hallazgo con feed caducado queda \`stale\`, no desaparece.

## 4. Diseño Rust

Lectores acotados para SPDX/CycloneDX/Syft JSON **solo si el MVP los necesita**; adaptador a OSV/Grype para consumir JSON; lector Kubernetes read-only por credenciales delegadas o salida suministrada por CI. Un correlador puro sobre entradas pinadas con tests de fixtures. No almacenar credenciales ni clonar el SBOM entero en Knowledge de SDDK. La inspección de entorno requiere RBAC read-only y filtrado por namespace/labels; la confirmación de un despliegue no autoriza cambios.

## 5. UAT

1. Componente presente en SBOM de imagen A y ausente en B; solo relacionar A.
2. Deployment declara etiqueta \`latest\` pero pod ejecuta digest antiguo; informar ambos sin confundir intención y realidad.
3. Pod reiniciado o entorno inaccesible: presencia anterior queda histórica, no actual.
4. Hallazgo en SBOM de build que no está en imagen final no se etiqueta automáticamente como componente desplegado.
5. Dos versiones del mismo paquete y dos imágenes con mismo tag no se fusionan por nombre.
6. Firma no verificada/no disponible no se anuncia como cadena de suministro confiable.
7. No hay call path en el scope de CogniCode: declarar límite del análisis, no «no explotable».
8. Empleado sin permiso de leer otro namespace recibe error de autorización, no inventario parcial presentado como completo.
9. Comparar coste y calidad frente a Syft + OSV/Grype + \`kubectl\` directo; si la correlación no añade valor material, descartar binario.

## 6. Integración

En SDDK se conserva evidencia y alcance, no se modifica Authority ni se crea un modelo global de criticidad. En CI puede funcionar solo con SBOM + inventario de deployments capturado por otro actor; ejecución bajo shell del agente es opcional, no obligatoria. Future use: generar la pregunta «¿qué revisamos primero?» con facts disponibles; no decidir prioridad por una clasificación heurística no validada.
