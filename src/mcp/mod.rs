use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::analyzer::GenericAnalyzer;
use crate::config::Config;
use crate::context::ContextBuilder;
use crate::training::{SearchCriteria, TrainingManager};
use crate::types::CodePattern;

/// MCP Server implementation
pub struct Server {
    config: Config,
    training_manager: TrainingManager,
}

/// JSON-RPC Request structure
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

/// JSON-RPC Response structure
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

/// JSON-RPC Error structure
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

impl Server {
    /// Creates a new MCP server instance.
    ///
    /// # Arguments
    /// * `config` - Server configuration
    ///
    /// # Returns
    /// A configured Server instance ready to run
    pub async fn new(config: Config) -> Result<Self> {
        // Initialize training manager
        let patterns_path = config.storage.base_path.join(&config.storage.patterns_file);
        tracing::info!(path = %patterns_path.display(), "Looking for patterns");
        let mut training_manager = TrainingManager::new(patterns_path.clone());

        // Load existing patterns
        match training_manager.load_patterns().await {
            Ok(_) => {
                tracing::info!(
                    count = training_manager.get_all_patterns().len(),
                    "Successfully loaded patterns"
                );
            }
            Err(e) => {
                tracing::warn!(
                    path = %patterns_path.display(),
                    error = %e,
                    "Error loading patterns, continuing with empty database"
                );
            }
        }

        Ok(Self {
            config,
            training_manager,
        })
    }

    /// Runs the MCP server main loop.
    ///
    /// Listens on stdio for JSON-RPC requests and processes them.
    pub async fn run(mut self) -> Result<()> {
        tracing::info!("MCP server starting on stdio transport");

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();

        tracing::debug!("Waiting for requests...");

        // Track if client uses Content-Length framing
        let mut use_framing = false;

        // Process requests
        loop {
            // Read message (auto-detect framing on first message)
            match Self::read_mcp_message(&mut reader, &mut use_framing).await {
                Ok(Some(json_body)) => {
                    if json_body.is_empty() {
                        continue;
                    }
tracing::debug!(
                        request = %&json_body[..json_body.len().min(100)],
                        "Received request"
                    );

                    match serde_json::from_str::<JsonRpcRequest>(&json_body) {
                        Ok(request) => {
                            // Check if this is a notification (no id field)
                            if request.id.is_none() && request.method.starts_with("notifications/")
                            {
                                tracing::trace!(method = %request.method, "Received notification, ignoring");
                                continue;
                            }

                            let response = self.handle_request(request).await;
                            match serde_json::to_string(&response) {
                                Ok(response_str) => {
                                    tracing::trace!(framing = use_framing, "Sending response");
                                    // Send response matching client's framing style
                                    if let Err(e) = Self::write_mcp_message(
                                        &mut stdout,
                                        &response_str,
                                        use_framing,
                                    )
                                    .await
                                    {
tracing::error!(error = %e, "Error writing response");
                                        break;
                                    }
                                    tracing::trace!("Response sent successfully");
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "Error serializing response");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to parse request");
                            let error_response = JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: None,
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32700,
                                    message: "Parse error".to_string(),
                                    data: Some(serde_json::json!({ "error": e.to_string() })),
                                }),
                            };

                            if let Ok(error_str) = serde_json::to_string(&error_response) {
                                let _ =
                                    Self::write_mcp_message(&mut stdout, &error_str, use_framing)
                                        .await;
                            }
                        }
                    }
                }
                Ok(None) => {
                    tracing::info!("stdin closed (EOF)");
                    break;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Error reading from stdin");
                    break;
                }
            }
        }

        tracing::info!("MCP server shutting down");
        Ok(())
    }

    /// Reads a single MCP message from stdin.
    /// Auto-detects framing style (Content-Length headers vs newline-delimited JSON).
    /// Sets `use_framing` to true if Content-Length headers are detected.
    async fn read_mcp_message(
        reader: &mut BufReader<tokio::io::Stdin>,
        use_framing: &mut bool,
    ) -> Result<Option<String>> {
        let mut first_line = String::new();

        // Read the first line to determine framing type
        let bytes_read = reader.read_line(&mut first_line).await?;
        if bytes_read == 0 {
            return Ok(None); // EOF
        }

        let trimmed = first_line.trim();

        // Check if this is Content-Length header (MCP standard framing)
        if trimmed.to_lowercase().starts_with("content-length:") {
            *use_framing = true;

            // Parse Content-Length value
            let length_str = trimmed
                .split(':')
                .nth(1)
                .ok_or_else(|| anyhow::anyhow!("Invalid Content-Length header"))?
                .trim();

            let content_length: usize = length_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid Content-Length value: {}", length_str))?;

            // Read remaining headers until empty line
            loop {
                let mut header_line = String::new();
                reader.read_line(&mut header_line).await?;

                // Empty line (just \r\n or \n) marks end of headers
                if header_line.trim().is_empty() {
                    break;
                }
            }

            // Read exactly content_length bytes for the JSON body
            let mut body = vec![0u8; content_length];
            reader.read_exact(&mut body).await?;

            let json_body = String::from_utf8(body)
                .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in message body: {}", e))?;

            Ok(Some(json_body))
        } else if trimmed.starts_with('{') {
            // Legacy: newline-delimited JSON (no Content-Length header)
            // Claude Desktop uses this mode
            Ok(Some(trimmed.to_string()))
        } else if trimmed.is_empty() {
            // Empty line, continue reading
            Ok(Some(String::new()))
        } else {
            // Unknown format - try to parse as JSON anyway
tracing::warn!(
                content = %&trimmed[..trimmed.len().min(50)],
                "Unexpected line format, attempting to parse"
            );
            Ok(Some(trimmed.to_string()))
        }
    }

    /// Writes a JSON-RPC response.
    /// If use_framing is true, adds Content-Length header.
    /// Otherwise, sends newline-delimited JSON (for Claude Desktop compatibility).
    async fn write_mcp_message(
        stdout: &mut tokio::io::Stdout,
        json: &str,
        use_framing: bool,
    ) -> Result<()> {
        if use_framing {
            let content_length = json.len();
            let header = format!("Content-Length: {}\r\n\r\n", content_length);
            stdout.write_all(header.as_bytes()).await?;
        }

        stdout.write_all(json.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;

        Ok(())
    }

    #[allow(dead_code)]
    async fn send_server_info(&self, stdout: &mut tokio::io::Stdout) -> Result<()> {
        let info = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "server/info",
            "params": {
                "name": self.config.server.name,
                "version": self.config.server.version,
                "capabilities": {
                    "tools": [
                        "analyze-project",
                        "get-patterns",
                        "train-pattern",
                        "search-patterns"
                    ]
                }
            }
        });

        let info_str = serde_json::to_string(&info)?;
        stdout.write_all(info_str.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;

        Ok(())
    }

    async fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        tracing::info!("Handling method: {}", request.method);

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize().await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tool_call(request.params).await,
            "prompts/list" => self.handle_prompts_list().await,
            "resources/list" => self.handle_resources_list().await,
            _ => Err(format!("Unknown method: {}", request.method)),
        };

        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            },
            Err(error_msg) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: error_msg,
                    data: None,
                }),
            },
        }
    }

    async fn handle_initialize(&self) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": self.config.server.name,
                "version": self.config.server.version,
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
                    "name": "analyze-project",
                    "description": "Analyze any project (Rust, Node, Python, .NET, Go, Java, PHP/Laravel/Vue) and get intelligent context about its structure, dependencies, and suggestions",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": {
                                "type": "string",
                                "description": "Path to the project directory (containing Cargo.toml, package.json, .csproj, pyproject.toml, go.mod, pom.xml, or composer.json)"
                            }
                        },
                        "required": ["project_path"]
                    }
                },
                {
                    "name": "get-patterns",
                    "description": "Get code patterns for a specific framework and category",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "framework": {
                                "type": "string",
                                "description": "Framework name (e.g., 'blazor-server', 'aspnet-core')"
                            },
                            "category": {
                                "type": "string",
                                "description": "Pattern category (e.g., 'lifecycle', 'dependency-injection')"
                            }
                        },
                        "required": ["framework"]
                    }
                },
                {
                    "name": "search-patterns",
                    "description": "Search for patterns with advanced criteria including query text, tags, and minimum score",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query text (searches in title, description, and code)"
                            },
                            "framework": {
                                "type": "string",
                                "description": "Filter by framework"
                            },
                            "category": {
                                "type": "string",
                                "description": "Filter by category"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Filter by tags"
                            },
                            "min_score": {
                                "type": "number",
                                "description": "Minimum relevance score (0.0 - 1.0)"
                            }
                        }
                    }
                },
                {
                    "name": "train-pattern",
                    "description": "Add a new code pattern to the training system",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Unique identifier for the pattern"
                            },
                            "category": {
                                "type": "string",
                                "description": "Pattern category"
                            },
                            "framework": {
                                "type": "string",
                                "description": "Target framework"
                            },
                            "version": {
                                "type": "string",
                                "description": "Framework version"
                            },
                            "title": {
                                "type": "string",
                                "description": "Pattern title"
                            },
                            "description": {
                                "type": "string",
                                "description": "Pattern description"
                            },
                            "code": {
                                "type": "string",
                                "description": "Code example"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Pattern tags"
                            }
                        },
                        "required": ["id", "category", "framework", "title", "description", "code"]
                    }
                },
                {
                    "name": "get-statistics",
                    "description": "Get statistics about the pattern database",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "get-help",
                    "description": "Get usage instructions for this MCP server. Call this first to understand how to use the available tools effectively.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]
        }))
    }

    async fn handle_tool_call(
        &mut self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let params = params.ok_or("Missing params")?;
        let tool_name = params["name"].as_str().ok_or("Missing tool name")?;
        let arguments = &params["arguments"];

        tracing::info!("Calling tool: {}", tool_name);

        match tool_name {
            "analyze-project" => self.tool_analyze_project(arguments).await,
            "get-patterns" => self.tool_get_patterns(arguments).await,
            "search-patterns" => self.tool_search_patterns(arguments).await,
            "train-pattern" => self.tool_train_pattern(arguments).await,
            "get-statistics" => self.tool_get_statistics().await,
            "get-help" => self.tool_get_help().await,
            _ => Err(format!("Unknown tool: {}", tool_name)),
        }
    }

    /// Analyzes a project and returns structured context.
    async fn tool_analyze_project(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let project_path = args["project_path"]
            .as_str()
            .ok_or("Missing project_path")?;

        tracing::debug!(path = %project_path, "Analyzing project");

        // Validate path exists
        let path = PathBuf::from(project_path);
        if !path.exists() {
            return Err(format!(
                "Project path does not exist: '{}'. Please provide an absolute path to a project directory.",
                project_path
            ));
        }

        if !path.is_dir() {
            return Err(format!(
                "Path is not a directory: '{}'. Please provide a directory path, not a file path.",
                project_path
            ));
        }

        tracing::debug!("Path validated, detecting project type");

        // Use the new generic analyzer
        let project = GenericAnalyzer::analyze(path.as_path())
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Analysis failed");
                format!("Failed to analyze project: {}. Make sure the directory contains a valid project file (Cargo.toml, package.json, .csproj, pyproject.toml, go.mod, or pom.xml).", e)
            })?;

        tracing::debug!(project_type = ?project.project_type, "Project analyzed successfully");

        // Build context with patterns
        let context_builder =
            ContextBuilder::new().with_training_manager(self.training_manager.clone());

        let analysis = context_builder
            .build_generic_analysis(project)
            .await
            .map_err(|e| format!("Failed to build analysis: {}", e))?;

        // Generate formatted context
        let context_string = context_builder.build_generic_context_string(&analysis);

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": context_string
            }],
            "isError": false
        }))
    }

    // Tool: get-patterns
    async fn tool_get_patterns(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let framework = args["framework"].as_str().ok_or("Missing framework")?;
        let category = args["category"].as_str();

        let patterns = if let Some(cat) = category {
            self.training_manager
                .search_by_framework_and_category(framework, cat)
        } else {
            let criteria = SearchCriteria {
                query: None,
                category: None,
                framework: Some(framework.to_string()),
                tags: vec![],
                min_score: 0.0,
            };
            self.training_manager
                .search_patterns(&criteria)
                .into_iter()
                .map(|(p, _)| p)
                .collect()
        };

        let mut output = String::new();
        output.push_str(&format!("# Patterns for {}\n\n", framework));

        if patterns.is_empty() {
            output.push_str("No patterns found.\n");
        } else {
            for pattern in patterns {
                output.push_str(&format!("## {}\n\n", pattern.title));
                output.push_str(&format!("**Category:** {}\n", pattern.category));
                output.push_str(&format!("**ID:** {}\n", pattern.id));
                output.push_str(&format!("{}\n\n", pattern.description));
                output.push_str("```csharp\n");
                output.push_str(&pattern.code);
                output.push_str("\n```\n\n");
                output.push_str(&format!("**Tags:** {}\n", pattern.tags.join(", ")));
                output.push_str(&format!("**Usage Count:** {}\n", pattern.usage_count));
                output.push_str(&format!(
                    "**Relevance:** {:.2}\n\n",
                    pattern.relevance_score
                ));
                output.push_str("---\n\n");
            }
        }

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": output
            }],
            "isError": false
        }))
    }

    // Tool: search-patterns
    async fn tool_search_patterns(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let criteria = SearchCriteria {
            query: args["query"].as_str().map(|s| s.to_string()),
            category: args["category"].as_str().map(|s| s.to_string()),
            framework: args["framework"].as_str().map(|s| s.to_string()),
            tags: args["tags"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            min_score: args["min_score"].as_f64().unwrap_or(0.0) as f32,
        };

        let results = self.training_manager.search_patterns(&criteria);

        let mut output = String::new();
        output.push_str("# Pattern Search Results\n\n");
        output.push_str(&format!("Found {} patterns\n\n", results.len()));

        for (pattern, score) in results {
            output.push_str(&format!("## {} (Score: {:.2})\n\n", pattern.title, score));
            output.push_str(&format!(
                "**Framework:** {} | **Category:** {}\n",
                pattern.framework, pattern.category
            ));
            output.push_str(&format!("{}\n\n", pattern.description));
            output.push_str("```csharp\n");
            output.push_str(&pattern.code);
            output.push_str("\n```\n\n");
            output.push_str("---\n\n");
        }

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": output
            }],
            "isError": false
        }))
    }

    // Tool: train-pattern
    async fn tool_train_pattern(
        &mut self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let pattern = CodePattern {
            id: args["id"].as_str().ok_or("Missing id")?.to_string(),
            category: args["category"]
                .as_str()
                .ok_or("Missing category")?
                .to_string(),
            framework: args["framework"]
                .as_str()
                .ok_or("Missing framework")?
                .to_string(),
            version: args["version"].as_str().unwrap_or("10.0").to_string(),
            title: args["title"].as_str().ok_or("Missing title")?.to_string(),
            description: args["description"]
                .as_str()
                .ok_or("Missing description")?
                .to_string(),
            code: args["code"].as_str().ok_or("Missing code")?.to_string(),
            tags: args["tags"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            usage_count: 0,
            relevance_score: 0.8, // Default relevance
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Add pattern with validation (prevents path traversal)
        self.training_manager
            .add_pattern(pattern.clone())
            .map_err(|e| format!("Invalid pattern: {}", e))?;

        // Save to disk
        self.training_manager
            .save_patterns()
            .await
            .map_err(|e| format!("Failed to save patterns: {}", e))?;

        let output = format!(
            "✅ Pattern '{}' added successfully!\n\n**ID:** {}\n**Category:** {}\n**Framework:** {}",
            pattern.title, pattern.id, pattern.category, pattern.framework
        );

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": output
            }],
            "isError": false
        }))
    }

    // Tool: get-statistics
    async fn tool_get_statistics(&self) -> Result<serde_json::Value, String> {
        let stats = self.training_manager.get_statistics();

        let output = format!(
            "# Pattern Database Statistics\n\n\
            **Total Patterns:** {}\n\
            **Total Usage:** {}\n\
            **Average Relevance:** {:.2}\n\n\
            ## Categories\n{}\n\n\
            ## Frameworks\n{}",
            stats["total_patterns"],
            stats["total_usage"],
            stats["avg_relevance"],
            stats["categories"]
                .as_array()
                .map(|arr| arr
                    .iter()
                    .map(|v| format!("- {}", v.as_str().unwrap_or("")))
                    .collect::<Vec<_>>()
                    .join("\n"))
                .unwrap_or_default(),
            stats["frameworks"]
                .as_array()
                .map(|arr| arr
                    .iter()
                    .map(|v| format!("- {}", v.as_str().unwrap_or("")))
                    .collect::<Vec<_>>()
                    .join("\n"))
                .unwrap_or_default()
        );

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": output
            }],
            "isError": false
        }))
    }

    // Tool: get-help
    async fn tool_get_help(&self) -> Result<serde_json::Value, String> {
        let help_text = r#"# MCP Context Rust - Guía de Uso

## Qué es esto
Servidor MCP que analiza proyectos de código y proporciona patrones de buenas prácticas.

## Herramientas disponibles

### 1. analyze-project
**Cuándo usar:** El usuario menciona un proyecto o ruta de código.
```
analyze-project { "project_path": "C:/ruta/al/proyecto" }
```
- Detecta automáticamente: Rust, Node, Python, PHP, Go, Java, .NET
- Devuelve: estructura, dependencias, framework detectado, sugerencias

### 2. search-patterns
**Cuándo usar:** El usuario pregunta "cómo hacer X" o busca buenas prácticas.
```
search-patterns { "query": "autenticación jwt" }
search-patterns { "query": "manejo errores", "framework": "laravel" }
```

### 3. get-patterns
**Cuándo usar:** El usuario quiere patrones de un framework específico.
```
get-patterns { "framework": "laravel" }
get-patterns { "framework": "react", "category": "hooks" }
```

### 4. train-pattern
**Cuándo usar:** El usuario quiere guardar código como patrón reutilizable.
```
train-pattern {
  "id": "mi-patron-001",
  "framework": "vue",
  "category": "composables",
  "title": "useAuth composable",
  "description": "Manejo de autenticación con Vue 3",
  "code": "export function useAuth() { ... }",
  "tags": ["auth", "vue3", "composable"]
}
```

### 5. get-statistics
**Cuándo usar:** Para saber cuántos patrones hay disponibles.
```
get-statistics {}
```

## Flujo recomendado

1. **Usuario menciona proyecto** → `analyze-project`
2. **Usuario pregunta cómo hacer algo** → `search-patterns`
3. **Usuario quiere ejemplos de framework** → `get-patterns`
4. **Usuario comparte código útil** → `train-pattern`

## Frameworks soportados
- **PHP:** laravel, symfony, wordpress
- **JavaScript:** react, vue, nextjs, express
- **Python:** django, flask, fastapi
- **Rust:** actix-web, axum, tokio
- **.NET:** blazor-server, aspnet-core
- **Go:** gin, fiber
- **Java:** spring

## Notas
- Usar rutas absolutas en analyze-project
- Los patrones se guardan en data/patterns/
- El servidor detecta automáticamente el tipo de proyecto
"#;

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": help_text
            }],
            "isError": false
        }))
    }

    async fn handle_prompts_list(&self) -> Result<serde_json::Value, String> {
        // Return empty prompts list (not implemented yet)
        Ok(serde_json::json!({
            "prompts": []
        }))
    }

    async fn handle_resources_list(&self) -> Result<serde_json::Value, String> {
        // Return empty resources list (not implemented yet)
        Ok(serde_json::json!({
            "resources": []
        }))
    }
}
