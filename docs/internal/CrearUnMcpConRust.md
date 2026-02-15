# 🦀 Guía Definitiva: Crear un MCP Server con Rust

**Fecha:** 2025-10-25
**Autor:** Lecciones aprendidas del desarrollo de mcp-rust-context
**Versión Protocolo MCP:** 2024-11-05

---

## 📋 Tabla de Contenidos

1. [Estructura del Proyecto](#estructura-del-proyecto)
2. [Dependencias Esenciales](#dependencias-esenciales)
3. [Protocolo MCP: Errores Comunes](#protocolo-mcp-errores-comunes)
4. [Sistema de Logging Correcto](#sistema-de-logging-correcto)
5. [Manejo de Rutas y Archivos](#manejo-de-rutas-y-archivos)
6. [Implementación del Servidor](#implementación-del-servidor)
7. [Configuración de Claude Desktop](#configuración-de-claude-desktop)
8. [Testing y Debugging](#testing-y-debugging)
9. [Checklist Final](#checklist-final)

---

## 🏗️ Estructura del Proyecto

```
mi-mcp-server/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library root
│   ├── config.rs            # Configuración con env vars
│   ├── types.rs             # Tipos compartidos
│   ├── mcp/                 # Protocolo MCP
│   │   └── mod.rs           # Servidor JSON-RPC
│   └── handlers/            # Lógica de negocio
│       └── mod.rs
├── data/                    # Datos (si aplica)
├── Cargo.toml
├── README.md
└── CrearUnMcpConRust.md    # Esta guía
```

---

## 📦 Dependencias Esenciales

### Cargo.toml

```toml
[package]
name = "mi-mcp-server"
version = "0.1.0"
edition = "2021"

[dependencies]
# Runtime async
tokio = { version = "1", features = ["full"] }

# Serialización JSON
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Manejo de errores
anyhow = "1"
thiserror = "1"

# Logging (OPCIONAL - solo si necesitas debug)
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilidades
dirs = "5"  # Para obtener directorios del sistema
```

**⚠️ IMPORTANTE:** No uses logging con colores ANSI en producción para MCPs.

---

## 🚨 Protocolo MCP: Errores Comunes

### ❌ ERROR 1: Logs con colores ANSI en stdout

**Problema:**
```rust
// ❌ MAL - Envía colores ANSI a stdout
tracing_subscriber::fmt::init();
```

**Solución:**
```rust
// ✅ BIEN - Sin ANSI, solo a stderr
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "error".into()),  // Solo errores por defecto
    )
    .with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)           // ⚠️ SIN COLORES
            .with_writer(std::io::stderr) // ⚠️ A STDERR
    )
    .init();
```

**Regla de oro:**
- **stdout** = Solo JSON-RPC (protocolo MCP)
- **stderr** = Logs, debug, errores humanos

---

### ❌ ERROR 2: Versión de protocolo incorrecta

**Problema:**
```rust
// ❌ MAL
"protocolVersion": "0.1.0"  // Versión del servidor, NO del protocolo
```

**Solución:**
```rust
// ✅ BIEN
async fn handle_initialize(&self) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "protocolVersion": "2024-11-05",  // ⚠️ Versión del PROTOCOLO MCP
        "serverInfo": {
            "name": "mi-mcp-server",
            "version": "0.1.0"  // Esta sí es tu versión
        },
        "capabilities": {
            "tools": {}
        }
    }))
}
```

**Versiones válidas del protocolo MCP:**
- `2024-11-05` (actual)
- `2024-10-07`
- Consultar: https://spec.modelcontextprotocol.io/

---

### ❌ ERROR 3: Responder a notificaciones

**Problema:**
```rust
// ❌ MAL - Responde a TODAS las requests
match serde_json::from_str::<JsonRpcRequest>(&line) {
    Ok(request) => {
        let response = self.handle_request(request).await;
        // Esto falla para notificaciones sin 'id'
    }
}
```

**Solución:**
```rust
// ✅ BIEN - Ignora notificaciones
match serde_json::from_str::<JsonRpcRequest>(&line) {
    Ok(request) => {
        // Las notificaciones NO tienen 'id' y NO se responden
        if request.id.is_none() && request.method.starts_with("notifications/") {
            eprintln!("Received notification: {}, ignoring", request.method);
            continue;  // ⚠️ NO responder
        }

        let response = self.handle_request(request).await;
        // ... enviar respuesta
    }
}
```

**Notificaciones comunes:**
- `notifications/initialized`
- `notifications/cancelled`
- `notifications/progress`

---

### ❌ ERROR 4: No implementar métodos opcionales

**Problema:**
```rust
// ❌ MAL - Retorna error para métodos opcionales
match request.method.as_str() {
    "initialize" => self.handle_initialize().await,
    "tools/list" => self.handle_tools_list().await,
    _ => Err(format!("Unknown method: {}", request.method)),  // ❌
}
```

**Solución:**
```rust
// ✅ BIEN - Retorna listas vacías para métodos opcionales
match request.method.as_str() {
    "initialize" => self.handle_initialize().await,
    "tools/list" => self.handle_tools_list().await,
    "tools/call" => self.handle_tool_call(request.params).await,

    // Métodos opcionales - retornar vacío en lugar de error
    "prompts/list" => Ok(serde_json::json!({"prompts": []})),
    "resources/list" => Ok(serde_json::json!({"resources": []})),

    _ => Err(format!("Unknown method: {}", request.method)),
}
```

---

### ❌ ERROR 5: Enviar info no solicitada al iniciar

**Problema:**
```rust
// ❌ MAL - Envía mensajes antes del handshake
pub async fn run(mut self) -> Result<()> {
    let mut stdout = tokio::io::stdout();

    // ❌ NO hacer esto
    self.send_server_info(&mut stdout).await?;

    // Esperar requests...
}
```

**Solución:**
```rust
// ✅ BIEN - Solo responde a requests del cliente
pub async fn run(mut self) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    // ⚠️ NO enviar nada primero, ESPERAR al cliente

    loop {
        match reader.next_line().await {
            Ok(Some(line)) => {
                // Procesar request y responder
            }
            Ok(None) => break,
            Err(e) => {
                eprintln!("Error reading: {}", e);
                break;
            }
        }
    }

    Ok(())
}
```

---

## 🪵 Sistema de Logging Correcto

### main.rs

```rust
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // ⚠️ IMPORTANTE: Solo si REALMENTE necesitas logs
    // Para producción, considera NO inicializar logging

    #[cfg(debug_assertions)]  // Solo en modo debug
    {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "error".into()),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(std::io::stderr)
            )
            .init();
    }

    // En producción, usa eprintln! para debug crítico
    eprintln!("MCP server starting...");

    let config = Config::load()?;
    let server = Server::new(config).await?;
    server.run().await?;

    Ok(())
}
```

### Para debugging en desarrollo

```rust
// ✅ BIEN - Solo a stderr
eprintln!("Debug: variable = {:?}", variable);

// ❌ MAL - NUNCA usar println! en un MCP
println!("Debug: ...");  // ❌ Contamina stdout
```

---

## 📁 Manejo de Rutas y Archivos

### ❌ ERROR 6: Rutas relativas al directorio de trabajo

**Problema:**
```rust
// ❌ MAL - Depende del directorio donde se ejecuta
let data_path = PathBuf::from("data/patterns");
```

Claude Desktop ejecuta el binario desde un directorio arbitrario, **NO** desde donde está el ejecutable.

**Solución 1: Variable de entorno (RECOMENDADO)**

```rust
// config.rs
pub fn default() -> Self {
    // Permitir configurar por variable de entorno
    let data_path = std::env::var("MCP_DATA_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Fallback: relativo al ejecutable
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .map(|exe_dir| exe_dir.join("data"))
                .unwrap_or_else(|| PathBuf::from("data"))
        });

    Self {
        data_path,
        // ...
    }
}
```

**Configuración en Claude Desktop:**

```json
{
  "mcpServers": {
    "mi-servidor": {
      "command": "C:\\path\\to\\mi-mcp.exe",
      "args": [],
      "env": {
        "MCP_DATA_PATH": "C:\\path\\to\\data"
      }
    }
  }
}
```

**Solución 2: Ruta absoluta hardcodeada**

```rust
// Solo si la ruta es fija
let data_path = PathBuf::from("C:\\MCPs\\mi-servidor\\data");
```

---

## 🖥️ Implementación del Servidor

### Estructura mínima de src/mcp/mod.rs

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

/// JSON-RPC Request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,  // ⚠️ Opcional para notificaciones
    method: String,
    params: Option<serde_json::Value>,
}

/// JSON-RPC Response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

pub struct Server {
    // Tu estado aquí
}

impl Server {
    pub async fn new() -> Result<Self> {
        eprintln!("Initializing MCP server...");
        Ok(Self {})
    }

    pub async fn run(mut self) -> Result<()> {
        eprintln!("MCP server ready on stdio");

        let stdin = tokio::io::stdin();
        let mut reader = tokio::io::BufReader::new(stdin).lines();
        let mut stdout = tokio::io::stdout();

        loop {
            match reader.next_line().await {
                Ok(Some(line)) => {
                    if line.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<JsonRpcRequest>(&line) {
                        Ok(request) => {
                            // Ignorar notificaciones
                            if request.id.is_none() && request.method.starts_with("notifications/") {
                                continue;
                            }

                            let response = self.handle_request(request).await;

                            if let Ok(response_str) = serde_json::to_string(&response) {
                                let _ = stdout.write_all(response_str.as_bytes()).await;
                                let _ = stdout.write_all(b"\n").await;
                                let _ = stdout.flush().await;
                            }
                        }
                        Err(e) => {
                            eprintln!("Parse error: {}", e);
                        }
                    }
                }
                Ok(None) => {
                    eprintln!("stdin closed");
                    break;
                }
                Err(e) => {
                    eprintln!("Read error: {}", e);
                    break;
                }
            }
        }

        eprintln!("MCP server shutting down");
        Ok(())
    }

    async fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize().await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tool_call(request.params).await,
            "prompts/list" => Ok(serde_json::json!({"prompts": []})),
            "resources/list" => Ok(serde_json::json!({"resources": []})),
            _ => Err(format!("Unknown method: {}", request.method)),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            },
            Err(msg) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: msg,
                    data: None,
                }),
            },
        }
    }

    async fn handle_initialize(&self) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",  // ⚠️ Versión del protocolo
            "serverInfo": {
                "name": "mi-mcp-server",
                "version": "0.1.0"
            },
            "capabilities": {
                "tools": {}
            }
        }))
    }

    async fn handle_tools_list(&self) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "tools": [
                {
                    "name": "mi-tool",
                    "description": "Descripción de mi tool",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "param1": {
                                "type": "string",
                                "description": "Primer parámetro"
                            }
                        },
                        "required": ["param1"]
                    }
                }
            ]
        }))
    }

    async fn handle_tool_call(&mut self, params: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        let params = params.ok_or("Missing params")?;
        let tool_name = params["name"].as_str().ok_or("Missing tool name")?;

        match tool_name {
            "mi-tool" => {
                // Lógica del tool
                Ok(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": "Resultado del tool"
                    }]
                }))
            }
            _ => Err(format!("Unknown tool: {}", tool_name))
        }
    }
}
```

---

## ⚙️ Configuración de Claude Desktop

### Windows

**Ruta:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "mi-servidor": {
      "command": "C:\\ruta\\completa\\target\\release\\mi-mcp-server.exe",
      "args": [],
      "env": {
        "MCP_DATA_PATH": "C:\\ruta\\completa\\data"
      }
    }
  }
}
```

### Linux/Mac

**Ruta:** `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "mi-servidor": {
      "command": "/ruta/completa/target/release/mi-mcp-server",
      "args": [],
      "env": {
        "MCP_DATA_PATH": "/ruta/completa/data"
      }
    }
  }
}
```

### ⚠️ IMPORTANTE: Rutas absolutas

- **SIEMPRE** usa rutas absolutas en `command`
- **NUNCA** uses rutas relativas
- Usa `\\` en Windows (JSON escaping)

---

## 🧪 Testing y Debugging

### Test manual con stdio

```bash
# Test 1: Initialize
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | ./target/release/mi-mcp-server

# Test 2: Tools list
(echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'; sleep 0.5; echo '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}') | ./target/release/mi-mcp-server
```

### Verificar logs de Claude Desktop

**Windows:**
```
%APPDATA%\Claude\logs\mcp-server-mi-servidor.log
```

**Linux/Mac:**
```
~/.config/Claude/logs/mcp-server-mi-servidor.log
```

### Errores comunes en logs

#### Error: "Unexpected token '\x1B'"
```
Causa: Logs con colores ANSI en stdout
Solución: Usar .with_ansi(false) y .with_writer(stderr)
```

#### Error: "Server disconnected"
```
Causa: Proceso termina prematuramente
Posibles causas:
- Error al cargar archivos (usar Result y manejar errores)
- Panic no capturado
- Ruta de archivo incorrecta
Solución: Añadir eprintln! para debug, verificar rutas
```

#### Error: "Invalid protocol version"
```
Causa: protocolVersion incorrecta en initialize
Solución: Usar "2024-11-05" (o versión actual del protocolo MCP)
```

#### Error: ZodError - "Required field 'id'"
```
Causa: Responder a notificaciones (que no tienen id)
Solución: Ignorar mensajes con method "notifications/*" y sin id
```

---

## ✅ Checklist Final

### Antes de compilar

- [ ] Cargo.toml tiene todas las dependencias
- [ ] Sin `println!` en el código (solo `eprintln!`)
- [ ] Logging configurado correctamente (sin ANSI, a stderr)
- [ ] Rutas usando variables de entorno o absolutas

### Antes de configurar en Claude Desktop

- [ ] `cargo build --release` exitoso
- [ ] Ejecutable existe en `target/release/`
- [ ] Test manual con echo funciona
- [ ] Responde a `initialize` con protocolo correcto

### Configuración de Claude Desktop

- [ ] Ruta absoluta al ejecutable
- [ ] Variables de entorno configuradas (si aplica)
- [ ] JSON válido (sin comas finales)
- [ ] Rutas con `\\` en Windows

### Después de reiniciar Claude Desktop

- [ ] No hay errores en logs (`%APPDATA%\Claude\logs\`)
- [ ] Mensaje "Server started and connected successfully"
- [ ] Tools disponibles en la UI de Claude

### Testing funcional

- [ ] `tools/list` funciona
- [ ] Tool calls funcionan correctamente
- [ ] Sin errores de ZodError
- [ ] Sin errores de "Server disconnected"

---

## 🎯 Template Completo Mínimo

### Cargo.toml

```toml
[package]
name = "mi-mcp-server"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
```

### src/main.rs

```rust
use anyhow::Result;

mod mcp;

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("Starting MCP server...");

    let server = mcp::Server::new().await?;
    server.run().await?;

    Ok(())
}
```

### src/mcp.rs

```rust
// Usar el código de "Implementación del Servidor" de arriba
```

---

## 📚 Referencias

- **Especificación MCP:** https://spec.modelcontextprotocol.io/
- **JSON-RPC 2.0:** https://www.jsonrpc.org/specification
- **Claude Desktop MCP Docs:** https://modelcontextprotocol.io/docs/tools/debugging

---

## 🐛 Troubleshooting Rápido

| Síntoma | Causa Probable | Solución |
|---------|---------------|----------|
| "Unexpected token '\x1B'" | Colores ANSI en stdout | `with_ansi(false)` + `with_writer(stderr)` |
| "Server disconnected" | Proceso termina | Verificar errores con `eprintln!`, manejar `Result` |
| "Invalid protocol version" | Versión incorrecta | Usar `"2024-11-05"` |
| ZodError sobre 'id' | Responder a notificaciones | Ignorar `notifications/*` sin id |
| "Pattern file not found" | Ruta relativa incorrecta | Usar env var `MCP_DATA_PATH` |
| "Failed to start" | Ruta ejecutable incorrecta | Verificar ruta absoluta en config |

---

## 💡 Tips Finales

1. **Empieza simple:** Primero haz que funcione con `initialize` y `tools/list`
2. **No uses logging en producción:** Solo `eprintln!` para debug crítico
3. **SIEMPRE usa rutas absolutas** en la configuración de Claude Desktop
4. **Test manual primero:** Usa `echo` + pipe antes de probar en Claude Desktop
5. **Lee los logs:** El archivo `.log` en `%APPDATA%\Claude\logs\` es tu amigo
6. **Versión del protocolo:** Consulta la spec actual, puede cambiar
7. **Manejo de errores:** Usa `Result` y maneja todos los casos
8. **No respondas a notificaciones:** Solo responde a requests con `id`

---

**🦀 ¡Feliz desarrollo de MCPs con Rust!**

---

**Última actualización:** 2025-10-25
**Basado en:** Desarrollo de mcp-rust-context v0.1.0
**Errores corregidos:** 6 errores críticos identificados y documentados
