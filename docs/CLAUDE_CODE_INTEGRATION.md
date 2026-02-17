# Integración con Claude Code CLI

Guía completa para usar `mcp-context-rust` con Claude Code (`claude` CLI).

---

## 1. Instalación del binario

### Opción A — binario en sitio (instalación actual)

El binario compilado está en:

```
/home/user/mcp-context-rust/target/release/mcp-context-rust
```

Para recompilar tras cambios en el código:

```bash
cargo build --release --manifest-path /home/user/mcp-context-rust/Cargo.toml
```

### Opción B — instalar en PATH (recomendado para uso regular)

```bash
cargo install --path /home/user/mcp-context-rust
# Instala en ~/.cargo/bin/mcp-context-rust
```

Si usas la opción B, actualiza el hook en `~/.claude/settings.json` para usar
`mcp-context-rust` sin ruta absoluta.

---

## 2. Registrar el servidor MCP en Claude Code

Edita o crea `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "mcp-context-rust": {
      "command": "/home/user/mcp-context-rust/target/release/mcp-context-rust",
      "args": [],
      "env": {
        "RUST_LOG": "error"
      }
    }
  }
}
```

Tras guardar, reinicia Claude Code. Puedes verificar que el servidor está activo
con `/mcp` dentro de una sesión de Claude Code.

---

## 3. Hook SessionStart — memoria automática de proyectos

El hook está ya configurado en `~/.claude/settings.json`. Al abrir Claude Code
en cualquier directorio, el sistema busca un `.rustscp` y, si existe, inyecta
el contexto del proyecto automáticamente.

```json
"SessionStart": [
    {
        "matcher": "",
        "hooks": [
            {
                "type": "command",
                "command": "/home/user/mcp-context-rust/target/release/mcp-context-rust read-context ."
            }
        ]
    }
]
```

**Resultado:** Al abrir Claude Code en un proyecto analizado, verás algo como:

```
# Project Context (.rustscp)

**Project:** mi-api v0.3.1
**Type:** rust | **Framework:** axum
**Last analyzed:** 2026-02-17 10:00 UTC

**Summary:** [RUST:mi-api v0.3.1] files:38 deps:12(2dev) ...

**Suggestions from last analysis:**
- No patterns found for framework 'axum'. Consider adding with train-pattern.
```

---

## 4. Flujo de trabajo diario

### Primera vez en un proyecto

```
# En Claude Code, con el MCP activo:
analyze-project { "project_path": "/ruta/a/mi-proyecto" }
```

Esto crea un `.rustscp` en la raíz del proyecto. La próxima vez que abras
Claude Code en ese directorio, el contexto se carga automáticamente.

### Sesiones siguientes

Claude Code ya tiene contexto. Puedes trabajar directamente. Si el proyecto
ha cambiado, re-ejecuta `analyze-project` para actualizar el `.rustscp`.

### Recuperar el análisis completo

Cuando Endless Mode está activo, el `.rustscp` incluye un `obs_id`:

```
get-observation { "obs_id": "550e8400-..." }
```

---

## 5. Herramientas disponibles en Claude Code

| Herramienta | Cuándo usarla |
|---|---|
| `analyze-project` | Primera vez o tras cambios grandes en el proyecto |
| `get-patterns` | Quieres ejemplos de un framework concreto |
| `search-patterns` | Buscas cómo hacer algo ("autenticación jwt", "manejo errores") |
| `train-pattern` | Quieres guardar un snippet como patrón reutilizable |
| `get-statistics` | Ver cuántos patrones hay en la base de datos |
| `set-endless-mode` | Reducir uso de tokens ~95% en sesiones largas |
| `get-observation` | Recuperar análisis completo archivado (requiere obs_id) |

---

## 6. El archivo `.rustscp`

Cada vez que ejecutas `analyze-project`, se crea o actualiza un `.rustscp`
en la raíz del proyecto analizado. Ejemplo de contenido:

```json
{
  "version": "1",
  "created_at": "2026-02-17T10:00:00Z",
  "updated_at": "2026-02-17T11:30:00Z",
  "project_name": "mi-api",
  "project_type": "rust",
  "project_version": "0.3.1",
  "framework": "axum",
  "summary": "[RUST:mi-api v0.3.1] files:38 deps:12(2dev) edition:2021 ...",
  "obs_id": "550e8400-e29b-41d4-a716-446655440000",
  "suggestions": [
    "No patterns found for framework 'axum'..."
  ]
}
```

**Notas:**
- `created_at` se preserva entre análisis (marca cuándo se analizó por primera vez)
- `updated_at` se actualiza en cada análisis
- `obs_id` solo aparece si Endless Mode estaba activo durante el análisis
- El archivo **no se commitea** (está en `.gitignore`)

---

## 7. Subcomando read-context

Puedes usarlo manualmente para inspeccionar el contexto de cualquier proyecto:

```bash
mcp-context-rust read-context /ruta/al/proyecto
mcp-context-rust read-context .   # directorio actual
```

Si no existe `.rustscp`, el comando no imprime nada (comportamiento correcto
para el hook — silencioso en proyectos sin contexto previo).

---

## 8. Endless Mode en sesiones largas

Para proyectos grandes o sesiones donde se usan muchas herramientas:

```
set-endless-mode { "enabled": true }
```

Tras activarlo, todas las respuestas usan formato compacto (~95% menos tokens).
El análisis completo queda archivado en `data/cache/observations/` y el
`.rustscp` guarda el `obs_id` para recuperarlo cuando sea necesario.

```
set-endless-mode { "enabled": false }   # volver a modo normal
```
