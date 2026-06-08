use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::analyzer::GenericAnalyzer;
use crate::config::Config;
use crate::context::ContextBuilder;
use crate::memory::MemoryStore;
use crate::observations::ObservationStore;
use crate::training::{SearchCriteria, TrainingManager};
use crate::types::{CodePattern, MemoryScope, MemorySearchCriteria, RememberInput};

/// MCP Server implementation
pub struct Server {
    config: Config,
    training_manager: TrainingManager,
    /// Persistent memory store (global + per-project decisions, conventions, gotchas, etc.)
    memory_store: MemoryStore,
    /// When true, all tool responses use compact single-line format (~95% token reduction).
    /// Full outputs are archived on disk and retrievable via `get-observation`.
    endless_mode: bool,
    /// Two-tier storage for Endless Mode observations.
    observations: ObservationStore,
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

        let obs_cache_dir = config
            .storage
            .base_path
            .join(&config.storage.cache_dir)
            .join("observations");

        // Memory store (Phase 1)
        let memories_path = config.storage.base_path.join(&config.storage.memories_dir);
        tracing::info!(path = %memories_path.display(), "Initializing memory store");
        let mut memory_store = MemoryStore::new(memories_path.clone());
        match memory_store.load().await {
            Ok(_) => {
                tracing::info!(count = memory_store.len(), "Memories loaded");
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load memories, starting empty");
            }
        }

        Ok(Self {
            config,
            training_manager,
            memory_store,
            endless_mode: false,
            observations: ObservationStore::new(obs_cache_dir),
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
                        "search-patterns",
                        "get-statistics",
                        "get-help",
                        "set-endless-mode",
                        "get-observation",
                        "remember",
                        "recall",
                        "get-memory",
                        "get-context"
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
                    "description": "Analyze any project (Rust, Node, Python, .NET, Go, Java, PHP/Laravel/Vue) and get intelligent context about its structure, dependencies, suggestions, and relevant persistent memories",
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
                    "name": "get-context",
                    "description": "UNIFIED CONTEXT TOOL (recommended): Get a single rich context combining project analysis + relevant persistent memories (global + project) + best matching patterns + suggestions. Use the optional 'task' for better relevance. One call instead of multiple.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": {
                                "type": "string",
                                "description": "Path to the project directory"
                            },
                            "task": {
                                "type": "string",
                                "description": "Optional current task or focus (e.g. 'implementing JWT auth' or 'debugging performance') to rank memories and patterns better"
                            },
                            "max_memories": {
                                "type": "integer",
                                "description": "Max relevant memories to include (default 8)"
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
                            },
                            "max_results": {
                                "type": "integer",
                                "description": "Maximum number of results to return (default: 20)"
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
                },
                {
                    "name": "set-endless-mode",
                    "description": "Toggle Endless Mode to reduce token usage by ~95%. When enabled, all tool responses use a compact single-line format and full outputs are archived on disk with an obs_id. Use get-observation to retrieve full details when needed. Allows up to 20x more tool calls before the context window fills.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "enabled": {
                                "type": "boolean",
                                "description": "true to enable compact output (Endless Mode ON), false to restore full verbose output"
                            }
                        },
                        "required": ["enabled"]
                    }
                },
                {
                    "name": "get-observation",
                    "description": "Retrieve the full archived output of a previous tool call by its obs_id. Use this when Endless Mode is active and you need the complete details that were compressed.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "obs_id": {
                                "type": "string",
                                "description": "The observation UUID returned by a previous tool call in Endless Mode"
                            }
                        },
                        "required": ["obs_id"]
                    }
                },
                {
                    "name": "remember",
                    "description": "Store an important fact, decision, convention, gotcha, architecture note, or user preference into persistent memory. Memories survive across sessions and are automatically surfaced during analyze-project and recall.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": {
                                "type": "string",
                                "enum": ["global", "project"],
                                "description": "'global' for user-wide knowledge or 'project' for project-specific memory"
                            },
                            "project_path": {
                                "type": "string",
                                "description": "Required when scope='project'. Absolute path to the project root."
                            },
                            "category": {
                                "type": "string",
                                "description": "Category e.g. decision, convention, gotcha, architecture, preference, fact, security, performance"
                            },
                            "title": {
                                "type": "string",
                                "description": "Short title for the memory"
                            },
                            "content": {
                                "type": "string",
                                "description": "The actual content to remember (markdown supported)"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Optional tags for search"
                            },
                            "importance": {
                                "type": "number",
                                "description": "Importance 0.0-1.0 (default 0.7)"
                            }
                        },
                        "required": ["scope", "category", "title", "content"]
                    }
                },
                {
                    "name": "recall",
                    "description": "Search persistent memories using free-text query, scope, category, or tags. Returns scored results (highest relevance first).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Free text search across title, content, category, tags"
                            },
                            "scope": {
                                "type": "string",
                                "enum": ["global", "project"],
                                "description": "Limit to global or project memories"
                            },
                            "project_path": {
                                "type": "string",
                                "description": "When scope=project, the project root path"
                            },
                            "category": {
                                "type": "string",
                                "description": "Filter by category"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Only memories containing ALL these tags"
                            },
                            "min_score": {
                                "type": "number",
                                "description": "Minimum relevance score (0.0-2.0)"
                            },
                            "max_results": {
                                "type": "integer",
                                "description": "Maximum results to return (default 20)"
                            }
                        }
                    }
                },
                {
                    "name": "get-memory",
                    "description": "Retrieve the most relevant persistent memories (global + project) for the current task. Use this (or rely on analyze-project which calls it automatically) to give the AI your previous decisions and context.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_path": {
                                "type": "string",
                                "description": "Project root (for project memories). If omitted only globals are returned."
                            },
                            "task": {
                                "type": "string",
                                "description": "Short description of the current task (e.g. 'adding authentication' or 'debugging prod error') to improve relevance ranking"
                            },
                            "max_results": {
                                "type": "integer",
                                "description": "Max memories (default 10)"
                            }
                        }
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
            "get-context" => self.tool_get_context(arguments).await,
            "get-patterns" => self.tool_get_patterns(arguments).await,
            "search-patterns" => self.tool_search_patterns(arguments).await,
            "train-pattern" => self.tool_train_pattern(arguments).await,
            "get-statistics" => self.tool_get_statistics().await,
            "get-help" => self.tool_get_help().await,
            "set-endless-mode" => self.tool_set_endless_mode(arguments).await,
            "get-observation" => self.tool_get_observation(arguments).await,
            "remember" => self.tool_remember(arguments).await,
            "recall" => self.tool_recall(arguments).await,
            "get-memory" => self.tool_get_memory(arguments).await,
            _ => Err(format!("Unknown tool: {}", tool_name)),
        }
    }

    /// Analyzes a project and returns structured context.
    /// Also surfaces relevant persistent memories (global + project-scoped).
    async fn tool_analyze_project(
        &mut self,
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

        // Prefer canonical path for stable memory scoping (prevents duplicate entries due to symlinks/casing)
        let canonical_path = std::fs::canonicalize(&path)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

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

        // Save .rustscp to project directory (non-fatal on failure)
        match crate::rustscp::ProjectContext::from_analysis(&analysis).save(&path) {
            Ok(p) => tracing::info!(path = %p.display(), "Saved .rustscp"),
            Err(e) => tracing::warn!(error = %e, "Failed to save .rustscp (non-fatal)"),
        }

        // Generate formatted context
        let mut full_output = context_builder.build_generic_context_string(&analysis);

        // === Phase 1: Surface relevant persistent memories (global + this project) ===
        let relevant = self
            .memory_store
            .get_relevant_for_project(Some(&canonical_path), None);

        if !relevant.is_empty() {
            let mut mem_section = String::from("\n\n## Relevant Persistent Memories\n\n");
            mem_section.push_str(&format!(
                "_Auto-surfaced {} memories (global + project). Use get-memory or recall for more targeted retrieval._\n\n",
                relevant.len()
            ));

            // Collect ids first so we can drop the immutable borrow before mutating the store
            let bump_ids: Vec<String> = relevant.iter().map(|(m, _)| m.id.clone()).collect();

            for (mem, score) in &relevant {
                mem_section.push_str(&format!("### {} (relevance {:.2})\n", mem.title, score));
                mem_section.push_str(&format!(
                    "**Scope:** {} | **Category:** {} | recalls: {}\n\n",
                    mem.scope, mem.category, mem.recall_count
                ));
                // Truncate very long contents for the main response (user can recall specific id later if needed)
                let content = if mem.content.len() > 1200 {
                    format!("{}...\n\n_(content truncated; use recall with id or get-observation style for full if archived)_", &mem.content[..1200])
                } else {
                    mem.content.clone()
                };
                mem_section.push_str(&content);
                if !mem.tags.is_empty() {
                    mem_section.push_str(&format!("\n\n*Tags:* `{}`", mem.tags.join("`, `")));
                }
                mem_section.push_str("\n\n---\n\n");
            }

            full_output.push_str(&mem_section);

            // Now safe to mutate: bump recall stats for the ones we surfaced
            for id in &bump_ids {
                let _ = self.memory_store.mark_recalled(id);
            }
            // Persist the recall bumps (small file, acceptable on analyze)
            if let Err(e) = self.memory_store.save().await {
                tracing::warn!(error = %e, "Failed to persist memory recall stats");
            }
        }

        let output = if self.endless_mode {
            let mut compact = context_builder.build_compact_context_string(&analysis);
            if !relevant.is_empty() {
                compact.push_str(&format!(
                    " | memories:{} (use get-memory for details)",
                    relevant.len()
                ));
            }
            let obs_id: String = self
                .observations
                .save("analyze-project", &full_output)
                .await
                .map_err(|e| format!("Failed to archive observation: {}", e))?;
            format!(
                "{}\nobs_id:{} (call get-observation to see full analysis including memories)",
                compact, obs_id
            )
        } else {
            full_output
        };

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": output
            }],
            "isError": false
        }))
    }

    /// Unified context tool (Phase 2). Combines analysis + memories (with optional task for relevance) + patterns + suggestions.
    /// This is intended as the primary "one call" tool for rich context.
    async fn tool_get_context(
        &mut self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let project_path = args["project_path"]
            .as_str()
            .ok_or("Missing project_path")?;

        let task = args["task"].as_str(); // optional

        tracing::debug!(path = %project_path, task = ?task, "Getting unified context");

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

        let canonical_path = std::fs::canonicalize(&path)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        // Analyze
        let project = GenericAnalyzer::analyze(path.as_path())
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Analysis failed");
                format!("Failed to analyze project: {}. Make sure the directory contains a valid project file (Cargo.toml, package.json, .csproj, pyproject.toml, go.mod, or pom.xml).", e)
            })?;

        let context_builder =
            ContextBuilder::new().with_training_manager(self.training_manager.clone());

        let analysis = context_builder
            .build_generic_analysis(project)
            .await
            .map_err(|e| format!("Failed to build analysis: {}", e))?;

        // Save .rustscp (non-fatal)
        match crate::rustscp::ProjectContext::from_analysis(&analysis).save(&path) {
            Ok(p) => tracing::info!(path = %p.display(), "Saved .rustscp"),
            Err(e) => tracing::warn!(error = %e, "Failed to save .rustscp (non-fatal)"),
        }

        let mut full_output = context_builder.build_generic_context_string(&analysis);

        // Memories, using task if provided for better ranking (Phase 2 unification)
        let relevant = self
            .memory_store
            .get_relevant_for_project(Some(&canonical_path), task);

        if !relevant.is_empty() {
            let mut mem_section = String::from("\n\n## Relevant Persistent Memories\n\n");
            mem_section.push_str(&format!(
                "_Auto-surfaced {} memories (global + project). Task-aware ranking used when 'task' provided._\n\n",
                relevant.len()
            ));

            let bump_ids: Vec<String> = relevant.iter().map(|(m, _)| m.id.clone()).collect();

            for (mem, score) in &relevant {
                mem_section.push_str(&format!("### {} (relevance {:.2})\n", mem.title, score));
                mem_section.push_str(&format!(
                    "**Scope:** {} | **Category:** {} | recalls: {}\n\n",
                    mem.scope, mem.category, mem.recall_count
                ));
                let content = if mem.content.len() > 1200 {
                    format!("{}...\n\n_(content truncated)_", &mem.content[..1200])
                } else {
                    mem.content.clone()
                };
                mem_section.push_str(&content);
                if !mem.tags.is_empty() {
                    mem_section.push_str(&format!("\n\n*Tags:* `{}`", mem.tags.join("`, `")));
                }
                mem_section.push_str("\n\n---\n\n");
            }

            full_output.push_str(&mem_section);

            for id in &bump_ids {
                let _ = self.memory_store.mark_recalled(id);
            }
            if let Err(e) = self.memory_store.save().await {
                tracing::warn!(error = %e, "Failed to persist memory recall stats");
            }
        }

        let output = if self.endless_mode {
            let mut compact = context_builder.build_compact_context_string(&analysis);
            if !relevant.is_empty() {
                compact.push_str(&format!(
                    " | memories:{} (task-aware) (use get-memory for details)",
                    relevant.len()
                ));
            }
            let obs_id: String = self
                .observations
                .save("get-context", &full_output)
                .await
                .map_err(|e| format!("Failed to archive observation: {}", e))?;
            format!(
                "{}\nobs_id:{} (call get-observation to see full unified context)",
                compact, obs_id
            )
        } else {
            full_output
        };

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": output
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
                max_results: None,
            };
            self.training_manager
                .search_patterns(&criteria)
                .into_iter()
                .map(|(p, _)| p)
                .collect()
        };

        // Build verbose output (always needed: either returned directly or archived)
        let mut full_output = String::new();
        full_output.push_str(&format!("# Patterns for {}\n\n", framework));

        if patterns.is_empty() {
            full_output.push_str("No patterns found.\n");
        } else {
            for pattern in &patterns {
                full_output.push_str(&format!("## {}\n\n", pattern.title));
                full_output.push_str(&format!("**Category:** {}\n", pattern.category));
                full_output.push_str(&format!("**ID:** {}\n", pattern.id));
                full_output.push_str(&format!("{}\n\n", pattern.description));
                full_output.push_str("```csharp\n");
                full_output.push_str(&pattern.code);
                full_output.push_str("\n```\n\n");
                full_output.push_str(&format!("**Tags:** {}\n", pattern.tags.join(", ")));
                full_output.push_str(&format!("**Usage Count:** {}\n", pattern.usage_count));
                full_output.push_str(&format!(
                    "**Relevance:** {:.2}\n\n",
                    pattern.relevance_score
                ));
                full_output.push_str("---\n\n");
            }
        }

        let output = if self.endless_mode {
            let compact = if patterns.is_empty() {
                format!("Patterns {}(0): none", framework)
            } else {
                let entries: Vec<String> = patterns
                    .iter()
                    .take(10)
                    .enumerate()
                    .map(|(i, p)| {
                        let tag = p.tags.first().map(|t| t.as_str()).unwrap_or("general");
                        format!("{}.{}[{},{:.2}]", i + 1, p.title, tag, p.relevance_score)
                    })
                    .collect();
                format!(
                    "Patterns {}({}): {}",
                    framework,
                    patterns.len(),
                    entries.join(" ")
                )
            };
            let obs_id: String = self
                .observations
                .save("get-patterns", &full_output)
                .await
                .map_err(|e| format!("Failed to archive observation: {}", e))?;
            format!(
                "{}\nobs_id:{} (call get-observation for full code examples)",
                compact, obs_id
            )
        } else {
            full_output
        };

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
            max_results: args["max_results"].as_u64().map(|n| n as usize),
        };

        let results = self.training_manager.search_patterns(&criteria);

        // Build verbose output (always needed: returned directly or archived)
        let mut full_output = String::new();
        full_output.push_str("# Pattern Search Results\n\n");
        full_output.push_str(&format!("Found {} patterns\n\n", results.len()));

        for (pattern, score) in &results {
            full_output.push_str(&format!("## {} (Score: {:.2})\n\n", pattern.title, score));
            full_output.push_str(&format!(
                "**Framework:** {} | **Category:** {}\n",
                pattern.framework, pattern.category
            ));
            full_output.push_str(&format!("{}\n\n", pattern.description));
            full_output.push_str("```csharp\n");
            full_output.push_str(&pattern.code);
            full_output.push_str("\n```\n\n");
            full_output.push_str("---\n\n");
        }

        let output = if self.endless_mode {
            let compact = if results.is_empty() {
                "Found 0: (no matches)".to_string()
            } else {
                let entries: Vec<String> = results
                    .iter()
                    .take(10)
                    .map(|(p, score)| {
                        format!("[{:.2}]{}|{}|{}", score, p.title, p.category, p.framework)
                    })
                    .collect();
                format!("Found {}: {}", results.len(), entries.join(" "))
            };
            let obs_id: String = self
                .observations
                .save("search-patterns", &full_output)
                .await
                .map_err(|e| format!("Failed to archive observation: {}", e))?;
            format!(
                "{}\nobs_id:{} (call get-observation for full code examples)",
                compact, obs_id
            )
        } else {
            full_output
        };

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

        let full_output = format!(
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

        let output = if self.endless_mode {
            let total = stats["total_patterns"].as_u64().unwrap_or(0);
            let frameworks: Vec<String> = stats["frameworks"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let fw_list = if frameworks.is_empty() {
                "none".to_string()
            } else {
                frameworks.join(",")
            };
            let obs_id: String = self
                .observations
                .save("get-statistics", &full_output)
                .await
                .map_err(|e| format!("Failed to archive observation: {}", e))?;
            format!(
                "DB: {} patterns across {} frameworks ({})\nobs_id:{}",
                total,
                frameworks.len(),
                fw_list,
                obs_id
            )
        } else {
            full_output
        };

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
- Devuelve: estructura, dependencias, framework detectado, sugerencias **+ memorias persistentes relevantes**

### 1b. Memory tools (Phase 1 - NEW)
**remember** - Guarda decisiones, convenciones, gotchas, arquitectura, preferencias.
```
remember { "scope": "project", "project_path": "C:/mi/proyecto", "category": "decision", "title": "Usamos Axum", "content": "Elegimos Axum tras benchmark vs Actix. Ver ADR-003." }
```
**recall** - Búsqueda avanzada de memorias.
```
recall { "query": "error handling", "project_path": "C:/mi/proyecto", "scope": "project" }
```
**get-memory** - Recupera lo más relevante para la tarea actual (global + proyecto).
```
get-memory { "project_path": "C:/mi/proyecto", "task": "añadir auth" }
```
Las memorias se auto-superponen en analyze-project.

### 1c. get-context (Phase 2 - NEW unified tool - recomendado)
**Cuándo usar:** Para obtener TODO el contexto de una sola vez (análisis + memorias + patrones + sugerencias).
```
get-context { "project_path": "C:/mi/proyecto", "task": "implementando login con JWT" }
```
- Combina análisis del proyecto + memorias relevantes (con ranking por tarea) + mejores patrones + sugerencias.
- Reduce llamadas (un tool en vez de analyze + get-memory + search-patterns).
- El 'task' mejora la relevancia de memorias y patrones.

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

1. **Usuario menciona proyecto** → `get-context` (o `analyze-project`)
2. **Usuario pregunta cómo hacer algo** → `search-patterns` o `get-context` con "task"
3. **Usuario quiere ejemplos de framework** → `get-patterns`
4. **Usuario comparte código útil** → `train-pattern`
5. **Recordar decisiones** → `remember` (luego aparece en get-context/analyze)

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

        let output = if self.endless_mode {
            let obs_id: String = self
                .observations
                .save("get-help", help_text)
                .await
                .map_err(|e| format!("Failed to archive observation: {}", e))?;
            format!(
                "Tools: analyze-project|get-context|get-patterns|search-patterns|train-pattern|get-statistics|set-endless-mode|get-observation|remember|recall|get-memory\nobs_id:{} (call get-observation for full usage guide)",
                obs_id
            )
        } else {
            help_text.to_string()
        };

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": output
            }],
            "isError": false
        }))
    }

    /// Tool: set-endless-mode
    /// Toggles compact output mode. Modifies runtime state; resets on server restart.
    async fn tool_set_endless_mode(
        &mut self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let enabled = args["enabled"]
            .as_bool()
            .ok_or("Missing or invalid 'enabled' field: must be a boolean")?;

        self.endless_mode = enabled;

        let message = if enabled {
            "Endless Mode ON. All responses now use compact format (~95% token reduction). Full outputs archived with obs_id — use get-observation{obs_id} to retrieve. Disable with set-endless-mode{\"enabled\":false}.".to_string()
        } else {
            "Endless Mode OFF. All responses restored to full verbose format.".to_string()
        };

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": message
            }],
            "isError": false
        }))
    }

    /// Tool: get-observation
    /// Retrieves a previously archived full tool output by its obs_id.
    async fn tool_get_observation(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let obs_id = args["obs_id"].as_str().ok_or("Missing obs_id")?;

        let content: Option<String> = self
            .observations
            .get(obs_id)
            .await
            .map_err(|e| format!("Invalid obs_id: {}", e))?;

        let output = match content {
            Some(text) => text,
            None => format!(
                "No observation found with id '{}'. Check that the obs_id is correct (observations are saved to data/cache/observations/).",
                obs_id
            ),
        };

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": output
            }],
            "isError": false
        }))
    }

    // =====================================================================
    // Phase 1: Memory Core Tools (remember / recall / get-memory)
    // =====================================================================

    /// Tool: remember
    /// Stores a new memory. Scope can be "global" or "project".
    async fn tool_remember(
        &mut self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let scope_str = args["scope"]
            .as_str()
            .ok_or("Missing 'scope' (global|project)")?;

        let scope = match scope_str {
            "global" => MemoryScope::Global,
            "project" => {
                let pp = args["project_path"]
                    .as_str()
                    .ok_or("project_path is required when scope=\"project\"")?;
                // Try to canonicalize for stable keys
                let canon = std::fs::canonicalize(pp)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| pp.to_string());
                MemoryScope::for_project(canon)
            }
            other => {
                return Err(format!(
                    "Invalid scope '{}'. Use 'global' or 'project'.",
                    other
                ))
            }
        };

        let category = args["category"]
            .as_str()
            .ok_or("Missing 'category'")?
            .to_string();
        let title = args["title"].as_str().ok_or("Missing 'title'")?.to_string();
        let content = args["content"]
            .as_str()
            .ok_or("Missing 'content'")?
            .to_string();

        let tags: Vec<String> = args["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let importance = args["importance"].as_f64().unwrap_or(0.7) as f32;

        let input = RememberInput {
            scope,
            category,
            title,
            content,
            tags,
            importance,
        };

        let mem = self
            .memory_store
            .remember(input)
            .map_err(|e| format!("Failed to remember: {}", e))?;

        self.memory_store
            .save()
            .await
            .map_err(|e| format!("Failed to persist memory: {}", e))?;

        let output = format!(
            "✅ Memory stored (id: {})\n\n**Scope:** {}\n**Category:** {}\n**Title:** {}\n\n{}",
            mem.id, mem.scope, mem.category, mem.title, mem.content
        );

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": output
            }],
            "isError": false,
            "memory_id": mem.id
        }))
    }

    /// Tool: recall
    /// Advanced search over memories.
    async fn tool_recall(&mut self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut criteria = MemorySearchCriteria::default();

        if let Some(q) = args["query"].as_str() {
            criteria.query = Some(q.to_string());
        }
        if let Some(cat) = args["category"].as_str() {
            criteria.category = Some(cat.to_string());
        }
        if let Some(min) = args["min_score"].as_f64() {
            criteria.min_score = min as f32;
        }
        if let Some(maxr) = args["max_results"].as_u64() {
            criteria.max_results = Some(maxr as usize);
        }

        if let Some(ts) = args["tags"].as_array() {
            criteria.tags = ts
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        if let Some(sc) = args["scope"].as_str() {
            match sc {
                "global" => criteria.scope = Some(MemoryScope::Global),
                "project" => {
                    let pp = args["project_path"]
                        .as_str()
                        .ok_or("project_path required when scope=project")?;
                    let canon = std::fs::canonicalize(pp)
                        .ok()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| pp.to_string());
                    criteria.scope = Some(MemoryScope::for_project(canon));
                }
                _ => return Err("scope must be 'global' or 'project'".to_string()),
            }
        }

        let results = self.memory_store.recall(&criteria);

        // Collect ids before any mutation (results hold & into the store)
        let ids: Vec<String> = results.iter().map(|(m, _)| m.id.clone()).collect();
        for id in &ids {
            let _ = self.memory_store.mark_recalled(id);
        }
        if !ids.is_empty() {
            let _ = self.memory_store.save().await;
        }

        let mut full_output = String::new();
        full_output.push_str(&format!("# Memory Recall ({} results)\n\n", results.len()));

        for (mem, score) in &results {
            full_output.push_str(&format!("## {} (score: {:.2})\n", mem.title, score));
            full_output.push_str(&format!(
                "id: {} | scope: {} | category: {} | recalls: {}\n\n",
                mem.id, mem.scope, mem.category, mem.recall_count
            ));
            full_output.push_str(&mem.content);
            if !mem.tags.is_empty() {
                full_output.push_str(&format!("\n\nTags: {}", mem.tags.join(", ")));
            }
            full_output.push_str("\n\n---\n\n");
        }

        if results.is_empty() {
            full_output.push_str("No memories matched your criteria.\n");
        }

        let output = if self.endless_mode {
            let compact = format!(
                "recall:{} memories (top score {:.2}) | use get-memory or recall with tighter query",
                results.len(),
                results.first().map(|(_, s)| *s).unwrap_or(0.0)
            );
            let obs_id = self
                .observations
                .save("recall", &full_output)
                .await
                .map_err(|e| format!("archive failed: {}", e))?;
            format!("{}\nobs_id:{}", compact, obs_id)
        } else {
            full_output
        };

        Ok(serde_json::json!({
            "content": [{ "type": "text", "text": output }],
            "isError": false
        }))
    }

    /// Tool: get-memory
    /// High-level "give me what I need to know right now" for a project/task.
    async fn tool_get_memory(
        &mut self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let project_path = args["project_path"].as_str();
        let task = args["task"].as_str();
        let max_results = args["max_results"].as_u64().unwrap_or(10) as usize;

        let canon = project_path.map(|pp| {
            std::fs::canonicalize(pp)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| pp.to_string())
        });

        let mut relevant = self
            .memory_store
            .get_relevant_for_project(canon.as_deref(), task);

        if relevant.len() > max_results {
            relevant.truncate(max_results);
        }

        // Collect ids first (relevant holds borrows into store vec)
        let ids: Vec<String> = relevant.iter().map(|(m, _)| m.id.clone()).collect();
        for id in &ids {
            let _ = self.memory_store.mark_recalled(id);
        }
        if !ids.is_empty() {
            let _ = self.memory_store.save().await;
        }

        let mut full_output = String::new();
        full_output.push_str("# Relevant Memories\n\n");

        if let Some(t) = task {
            full_output.push_str(&format!("Task hint: {}\n\n", t));
        }

        if relevant.is_empty() {
            full_output.push_str("No memories found for this scope.\n");
        } else {
            for (mem, score) in &relevant {
                full_output.push_str(&format!("## {} (relevance: {:.2})\n", mem.title, score));
                full_output.push_str(&format!(
                    "id:{} | {} | {}\n\n",
                    mem.id, mem.scope, mem.category
                ));
                let c = if mem.content.len() > 800 {
                    format!("{}…", &mem.content[..800])
                } else {
                    mem.content.clone()
                };
                full_output.push_str(&c);
                full_output.push_str("\n\n---\n\n");
            }
        }

        let output = if self.endless_mode {
            let compact = format!(
                "mem:{} relevant (use get-observation or full recall for details)",
                relevant.len()
            );
            let obs_id = self
                .observations
                .save("get-memory", &full_output)
                .await
                .map_err(|e| format!("archive: {}", e))?;
            format!("{}\nobs_id:{}", compact, obs_id)
        } else {
            full_output
        };

        Ok(serde_json::json!({
            "content": [{ "type": "text", "text": output }],
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
