# Test MCP - Pruebas del protocolo MCP server

Ejecuta pruebas del servidor MCP enviando requests JSON-RPC 2.0 via stdio y verificando las respuestas.

## Pre-requisitos

Primero compila el servidor:
```bash
cargo build --release 2>&1
```

## Pruebas a ejecutar

### 1. Initialize handshake
```bash
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}},"id":1}' | timeout 5 cargo run --release 2>/dev/null | head -1
```

Verificar que la respuesta contiene:
- `"protocolVersion": "2024-11-05"`
- `"serverInfo"` con name y version
- `"capabilities"` con `"tools": {}`

### 2. Tools list
```bash
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}\n' | timeout 5 cargo run --release 2>/dev/null
```

Verificar que la respuesta del id:2 contiene los tools:
- `analyze-project`
- `get-patterns`
- `search-patterns`
- `train-pattern`
- `get-statistics`
- `get-help`
- `set-endless-mode`
- `get-observation`

### 3. Get statistics
```bash
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get-statistics","arguments":{}},"id":2}\n' | timeout 5 cargo run --release 2>/dev/null
```

Verificar que retorna estadisticas con `total_patterns`, `categories`, `frameworks`.

### 4. Get help
```bash
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get-help","arguments":{}},"id":2}\n' | timeout 5 cargo run --release 2>/dev/null
```

Verificar que retorna la guia de uso.

### 5. Analyze this project (self-analysis)
```bash
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"analyze-project","arguments":{"project_path":"'"$(pwd)"'"}},"id":2}\n' | timeout 10 cargo run --release 2>/dev/null
```

Verificar que detecta el proyecto como tipo Rust con las dependencias correctas.

### 6. Error handling - invalid path
```bash
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"analyze-project","arguments":{"project_path":"/nonexistent/path"}},"id":2}\n' | timeout 5 cargo run --release 2>/dev/null
```

Verificar que retorna un error descriptivo, no un crash.

### 7. Error handling - unknown tool
```bash
printf '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}\n{"jsonrpc":"2.0","method":"tools/call","params":{"name":"nonexistent-tool","arguments":{}},"id":2}\n' | timeout 5 cargo run --release 2>/dev/null
```

Verificar que retorna error JSON-RPC apropiado.

## Formato del reporte

Para cada prueba, reporta:
- **Nombre**: Nombre de la prueba
- **Comando**: El comando ejecutado
- **Resultado**: PASS / FAIL
- **Respuesta**: Respuesta recibida (truncada si es muy larga)
- **Validacion**: Que se verifico

Al final, tabla resumen:

| # | Test | Resultado |
|---|------|-----------|
| 1 | Initialize | PASS/FAIL |
| 2 | Tools list | PASS/FAIL |
| ... | ... | ... |

**Total: X/7 tests passed**
