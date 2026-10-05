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

Verificar que la respuesta del id:2 contiene los tools (12 total):
- `analyze-project`
- `get-context` (nuevo unificado Phase 2)
- `get-patterns`
- `search-patterns`
- `train-pattern`
- `get-statistics`
- `get-help`
- `set-endless-mode`
- `get-observation`
- `remember`
- `recall`
- `get-memory`

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

### 8. Remember a project memory (news / Phase 1)
```powershell
$proj = (Get-Location).Path
$init = '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'
$remember = '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"remember","arguments":{"scope":"project","project_path":"' + $proj + '","category":"test","title":"Test memory from MCP test","content":"This decision was made during automated testing of news updates.","tags":["testing","memory-phase1"],"importance":0.8}},"id":2}'
$init + "`n" + $remember | cargo run --release 2>$null
```
Verificar que retorna éxito y un `memory_id`.

### 9. get-memory for current project (news)
```powershell
$proj = (Get-Location).Path
$init = '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'
$getmem = '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get-memory","arguments":{"project_path":"' + $proj + '","task":"testing memory features"}},"id":2}'
$init + "`n" + $getmem | cargo run --release 2>$null
```
Verificar que la respuesta contiene el título "Test memory from MCP test" o "Relevant Persistent Memories" y el contenido del test anterior.

### 10. recall memory (news)
```powershell
$init = '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'
$recall = '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"recall","arguments":{"query":"Test memory from MCP test","max_results":5}},"id":2}'
$init + "`n" + $recall | cargo run --release 2>$null
```
Verificar resultados con score y el memory creado.

### 11. analyze-project surfaces memories (news integration)
```powershell
$proj = (Get-Location).Path
$init = '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'
$analyze = '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"analyze-project","arguments":{"project_path":"' + $proj + '"}},"id":2}'
$init + "`n" + $analyze | cargo run --release 2>$null | Select-String -Pattern "Relevant Persistent Memories|Test memory from MCP test" -Quiet
```
Verificar que el output de analyze-project menciona las memorias persistentes (o el test memory específico).

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
| 3 | Get statistics | PASS/FAIL |
| 4 | Get help | PASS/FAIL |
| 5 | Analyze this project | PASS/FAIL |
| 6 | Error handling - invalid path | PASS/FAIL |
| 7 | Error handling - unknown tool | PASS/FAIL |
| 8 | Remember a project memory (news) | PASS/FAIL |
| 9 | get-memory for current project (news) | PASS/FAIL |
| 10 | recall memory (news) | PASS/FAIL |
| 11 | analyze-project surfaces memories (news integration) | PASS/FAIL |

**Total: X/12 tests passed** (incl. get-context for Phase 2 unification)
