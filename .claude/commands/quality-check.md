# Quality Check - Pipeline completo de calidad para mcp-context-rust

Ejecuta el pipeline completo de calidad del proyecto Rust MCP server. Reporta cada paso con su resultado y al final genera un resumen ejecutivo.

## Pasos a ejecutar (en orden)

### 1. Formateo (cargo fmt)
```bash
cargo fmt --check
```
Si falla, ejecuta `cargo fmt` para corregir automáticamente y reporta los archivos modificados.

### 2. Linting (cargo clippy)
```bash
cargo clippy --all-targets -- -D warnings
```
Si hay warnings, lista cada uno con archivo y línea. Sugiere correcciones concretas.

### 3. Build
```bash
cargo build --release 2>&1
```
Reporta si compila correctamente. Si falla, analiza el error y sugiere la corrección.

### 4. Tests
```bash
cargo test 2>&1
```
Reporta: tests totales, pasados, fallidos, ignorados. Si alguno falla, muestra el detalle del fallo.

### 5. Auditoría de seguridad (cargo audit)
Si `cargo-audit` está instalado:
```bash
cargo audit
```
Si no está instalado, indica cómo instalarlo: `cargo install cargo-audit`

### 6. Verificación de patrones
Verifica que todos los archivos JSON en `data/patterns/` son JSON válido y siguen el schema esperado (campo `patterns` como array, cada patrón con `id`, `category`, `framework`, `title`, `description`, `code`, `tags`).

## Formato del resumen final

Genera una tabla resumen:

| Paso | Estado | Detalles |
|------|--------|----------|
| fmt | OK/FIXED/FAIL | ... |
| clippy | OK/WARNINGS/FAIL | ... |
| build | OK/FAIL | ... |
| tests | X/Y passed | ... |
| audit | OK/VULNERABILITIES | ... |
| patterns | X valid / Y total | ... |

Si todo pasa, confirma que el proyecto está listo para commit/release.
Si algo falla, prioriza los problemas por severidad y sugiere el orden de corrección.
