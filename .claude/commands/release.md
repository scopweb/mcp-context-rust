# Release - Preparacion de release del MCP server

Prepara una nueva version del servidor MCP para release. Ejecuta todas las validaciones y genera los artefactos necesarios.

## Argumentos

El usuario debe indicar el tipo de version: `patch`, `minor`, o `major`. Si no lo indica, pregunta.

## Proceso

### 1. Determinar nueva version
Lee la version actual de `Cargo.toml` (campo `version` en `[package]`).
Calcula la nueva version segun semver:
- `patch`: 0.1.0 -> 0.1.1
- `minor`: 0.1.0 -> 0.2.0
- `major`: 0.1.0 -> 1.0.0

Confirma con el usuario antes de proceder.

### 2. Quality check completo
Ejecuta el pipeline completo (equivalente al skill `/quality-check`):
```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test
```

Si algo falla, detente y reporta. No se puede hacer release con fallos.

### 3. Actualizar version
Actualiza la version en:
- `Cargo.toml` - campo `version` en `[package]`
- `src/config.rs` - constante de version en `ServerConfig::default()` si existe
- `src/mcp/mod.rs` - cualquier referencia hardcodeada a la version

### 4. Actualizar CHANGELOG.md
Lee el CHANGELOG.md existente. Agrega una nueva entrada al inicio con:
- Numero de version y fecha
- Lista de cambios desde la ultima version (usa `git log` desde el ultimo tag)
- Agrupados en: Added, Changed, Fixed, Removed (si aplica)

### 5. Verificar build release
```bash
cargo build --release 2>&1
```

Reporta el tamano del binario resultante:
```bash
ls -lh target/release/mcp-context-rust
```

### 6. Crear tag (solo si el usuario confirma)
Pregunta al usuario si quiere crear el tag git:
```bash
git tag -a v{VERSION} -m "Release v{VERSION}"
```

### 7. Resumen de release

```
# Release v{VERSION} preparado

## Cambios
- [lista de cambios]

## Verificaciones
- fmt: OK
- clippy: OK
- build: OK (binary: X MB)
- tests: X/Y passed

## Archivos modificados
- Cargo.toml
- CHANGELOG.md
- [otros]

## Siguiente paso
git push origin main --tags
```
