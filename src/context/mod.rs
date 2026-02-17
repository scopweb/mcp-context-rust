use crate::training::{SearchCriteria, TrainingManager};
use crate::types::{
    AnalysisResult, CodePattern, DotNetProject, Project, ProjectType, SeverityLevel, Statistics,
    Suggestion,
};
use anyhow::Result;

/// Builds intelligent context for AI assistants based on project analysis
#[derive(Default)]
pub struct ContextBuilder {
    training_manager: Option<TrainingManager>,
}

impl ContextBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the training manager for pattern retrieval
    pub fn with_training_manager(mut self, manager: TrainingManager) -> Self {
        self.training_manager = Some(manager);
        self
    }

    /// Build complete analysis with patterns and suggestions (generic version)
    pub async fn build_generic_analysis(&self, project: Project) -> Result<AnalysisResult> {
        // Detect framework type
        let framework_type = self.detect_framework_from_project(&project);

        // Get relevant patterns
        let patterns = if let Some(ref manager) = self.training_manager {
            self.get_patterns_for_project(manager, &framework_type, &project)?
        } else {
            vec![]
        };

        // Generate suggestions
        let suggestions = self.generate_project_suggestions(&project, &framework_type);

        // Collect statistics
        let statistics = Statistics {
            total_files: project.files.len(),
            total_classes: project.files.iter().map(|f| f.symbols.len()).sum(),
            total_methods: 0, // Would need symbol parsing
            total_lines: 0,
            framework_version: project
                .metadata
                .target_framework
                .clone()
                .or(project.metadata.rust_edition.clone())
                .or(project.metadata.node_version.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            package_count: project.dependencies.len(),
        };

        Ok(AnalysisResult {
            project,
            patterns,
            suggestions,
            statistics,
        })
    }

    /// Detect framework from generic project
    pub fn detect_framework_from_project(&self, project: &Project) -> String {
        match project.project_type {
            ProjectType::DotNet => {
                // Check for Blazor, ASP.NET, etc.
                if project
                    .dependencies
                    .iter()
                    .any(|d| d.name.contains("AspNetCore.Components"))
                {
                    "blazor-server".to_string()
                } else if project
                    .dependencies
                    .iter()
                    .any(|d| d.name.contains("AspNetCore"))
                {
                    "aspnet-core".to_string()
                } else {
                    "dotnet".to_string()
                }
            }
            ProjectType::Rust => {
                if project.dependencies.iter().any(|d| d.name == "actix-web") {
                    "actix-web".to_string()
                } else if project.dependencies.iter().any(|d| d.name == "axum") {
                    "axum".to_string()
                } else if project.dependencies.iter().any(|d| d.name == "tokio") {
                    "tokio".to_string()
                } else {
                    "rust".to_string()
                }
            }
            ProjectType::Node => {
                if let Some(fw) = project.metadata.extra.get("framework") {
                    fw.clone()
                } else if project.dependencies.iter().any(|d| d.name == "express") {
                    "express".to_string()
                } else if project.dependencies.iter().any(|d| d.name == "react") {
                    "react".to_string()
                } else if project.dependencies.iter().any(|d| d.name == "vue") {
                    "vue".to_string()
                } else if project.dependencies.iter().any(|d| d.name == "next") {
                    "nextjs".to_string()
                } else {
                    "node".to_string()
                }
            }
            ProjectType::Python => {
                if project.dependencies.iter().any(|d| d.name == "django") {
                    "django".to_string()
                } else if project.dependencies.iter().any(|d| d.name == "flask") {
                    "flask".to_string()
                } else if project.dependencies.iter().any(|d| d.name == "fastapi") {
                    "fastapi".to_string()
                } else {
                    "python".to_string()
                }
            }
            ProjectType::Go => {
                if project
                    .dependencies
                    .iter()
                    .any(|d| d.name.contains("gin-gonic"))
                {
                    "gin".to_string()
                } else if project
                    .dependencies
                    .iter()
                    .any(|d| d.name.contains("fiber"))
                {
                    "fiber".to_string()
                } else {
                    "go".to_string()
                }
            }
            ProjectType::Java => {
                if project
                    .dependencies
                    .iter()
                    .any(|d| d.name.contains("spring"))
                {
                    "spring".to_string()
                } else {
                    "java".to_string()
                }
            }
            ProjectType::Php => {
                // Use pre-detected framework from metadata
                if let Some(fw) = project.metadata.extra.get("framework") {
                    let frontend = project.metadata.extra.get("frontend");
                    match (fw.as_str(), frontend) {
                        ("laravel", Some(fe)) => format!("laravel-{}", fe),
                        _ => fw.clone(),
                    }
                } else {
                    "php".to_string()
                }
            }
            ProjectType::Unknown => "generic".to_string(),
        }
    }

    /// Get patterns relevant to the project
    fn get_patterns_for_project(
        &self,
        manager: &TrainingManager,
        framework: &str,
        _project: &Project,
    ) -> Result<Vec<CodePattern>> {
        let criteria = SearchCriteria {
            query: None,
            category: None,
            framework: Some(framework.to_string()),
            tags: vec![],
            min_score: 0.7,
        };

        let results = manager.search_patterns(&criteria);
        Ok(results
            .into_iter()
            .take(10)
            .map(|(p, _)| p.clone())
            .collect())
    }

    /// Generate suggestions for generic project
    fn generate_project_suggestions(&self, project: &Project, framework: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Project type specific suggestions
        match project.project_type {
            ProjectType::Node => {
                // Check for security vulnerabilities indicators
                if project
                    .dependencies
                    .iter()
                    .any(|d| d.name == "express" && d.version.starts_with("3."))
                {
                    suggestions.push(Suggestion {
                        severity: SeverityLevel::Warning,
                        category: "security".to_string(),
                        message:
                            "Express 3.x is outdated. Consider upgrading to Express 4.x or 5.x"
                                .to_string(),
                        file: None,
                        line: None,
                    });
                }
            }
            ProjectType::Python => {
                if !project.path.join("requirements.txt").exists()
                    && !project.path.join("pyproject.toml").exists()
                {
                    suggestions.push(Suggestion {
                        severity: SeverityLevel::Info,
                        category: "best-practices".to_string(),
                        message: "Consider adding a requirements.txt or pyproject.toml for dependency management".to_string(),
                        file: None,
                        line: None,
                    });
                }
            }
            ProjectType::Rust => {
                // Check for common patterns
                if project.dependencies.iter().any(|d| d.name == "unwrap") {
                    suggestions.push(Suggestion {
                        severity: SeverityLevel::Warning,
                        category: "error-handling".to_string(),
                        message: "Avoid using .unwrap() in production code. Use proper error handling with Result".to_string(),
                        file: None,
                        line: None,
                    });
                }
            }
            ProjectType::Php => {
                // Laravel specific suggestions
                if let Some(fw) = project.metadata.extra.get("framework") {
                    if fw == "laravel" {
                        // Check for .env file
                        if !project.path.join(".env").exists() {
                            suggestions.push(Suggestion {
                                severity: SeverityLevel::Warning,
                                category: "configuration".to_string(),
                                message: "Missing .env file. Copy .env.example to .env and configure your environment".to_string(),
                                file: None,
                                line: None,
                            });
                        }

                        // Check for outdated Laravel
                        if project
                            .dependencies
                            .iter()
                            .any(|d| d.name == "laravel/framework" && d.version.starts_with("^8"))
                        {
                            suggestions.push(Suggestion {
                                severity: SeverityLevel::Info,
                                category: "upgrade".to_string(),
                                message: "Laravel 8.x is in maintenance mode. Consider upgrading to Laravel 10 or 11".to_string(),
                                file: None,
                                line: None,
                            });
                        }
                    }
                }

                // Vue.js with PHP suggestions
                if let Some(frontend) = project.metadata.extra.get("frontend") {
                    if frontend == "vue" {
                        // Check for Inertia.js (common in Laravel + Vue)
                        if project
                            .dependencies
                            .iter()
                            .any(|d| d.name == "inertiajs/inertia-laravel")
                        {
                            suggestions.push(Suggestion {
                                severity: SeverityLevel::Info,
                                category: "architecture".to_string(),
                                message: "Inertia.js detected. Consider using shared data for common props across pages".to_string(),
                                file: None,
                                line: None,
                            });
                        }
                    }
                }

                // Security: Check for common security packages
                let has_security_package = project.dependencies.iter().any(|d| {
                    d.name == "paragonie/random_compat" || d.name == "defuse/php-encryption"
                });
                if !has_security_package && project.dependencies.len() > 5 {
                    suggestions.push(Suggestion {
                        severity: SeverityLevel::Info,
                        category: "security".to_string(),
                        message: "Consider adding security packages like paragonie/random_compat for cryptographic operations".to_string(),
                        file: None,
                        line: None,
                    });
                }
            }
            _ => {}
        }

        // Generic suggestion based on file count
        if project.files.len() > 100 {
            suggestions.push(Suggestion {
                severity: SeverityLevel::Info,
                category: "architecture".to_string(),
                message: format!(
                    "Large project with {} files. Consider modular organization.",
                    project.files.len()
                ),
                file: None,
                line: None,
            });
        }

        // Framework-specific pattern availability
        if let Some(ref manager) = self.training_manager {
            let patterns = manager.search_by_framework_and_category(framework, "");
            if patterns.is_empty() {
                suggestions.push(Suggestion {
                    severity: SeverityLevel::Info,
                    category: "patterns".to_string(),
                    message: format!(
                        "No patterns found for framework '{}'. Consider adding patterns with train-pattern.",
                        framework
                    ),
                    file: None,
                    line: None,
                });
            }
        }

        suggestions
    }

    /// Build a formatted context string for generic projects
    pub fn build_generic_context_string(&self, analysis: &AnalysisResult) -> String {
        let mut context = String::new();
        let project = &analysis.project;

        context.push_str(&format!(
            "# {} Project Analysis\n\n",
            project.project_type.as_str().to_uppercase()
        ));
        context.push_str(&format!("**Project:** {}\n", project.name));
        if let Some(ref version) = project.version {
            context.push_str(&format!("**Version:** {}\n", version));
        }
        context.push_str(&format!("**Type:** {}\n", project.project_type.as_str()));

        // Metadata
        if let Some(ref tf) = project.metadata.target_framework {
            context.push_str(&format!("**Target Framework:** {}\n", tf));
        }
        if let Some(ref edition) = project.metadata.rust_edition {
            context.push_str(&format!("**Rust Edition:** {}\n", edition));
        }
        if let Some(ref entry) = project.metadata.entry_point {
            context.push_str(&format!("**Entry Point:** {}\n", entry));
        }
        context.push('\n');

        // Dependencies
        if !project.dependencies.is_empty() {
            context.push_str("## Dependencies\n\n");

            let prod_deps: Vec<_> = project
                .dependencies
                .iter()
                .filter(|d| !d.dev_only)
                .collect();
            let dev_deps: Vec<_> = project.dependencies.iter().filter(|d| d.dev_only).collect();

            if !prod_deps.is_empty() {
                context.push_str("### Production\n");
                for dep in prod_deps.iter().take(20) {
                    context.push_str(&format!("- {} ({})\n", dep.name, dep.version));
                }
                if prod_deps.len() > 20 {
                    context.push_str(&format!("- ... and {} more\n", prod_deps.len() - 20));
                }
                context.push('\n');
            }

            if !dev_deps.is_empty() {
                context.push_str("### Development\n");
                for dep in dev_deps.iter().take(10) {
                    context.push_str(&format!("- {} ({})\n", dep.name, dep.version));
                }
                if dev_deps.len() > 10 {
                    context.push_str(&format!("- ... and {} more\n", dev_deps.len() - 10));
                }
                context.push('\n');
            }
        }

        // Statistics
        context.push_str("## Project Statistics\n\n");
        context.push_str(&format!(
            "- Total Files: {}\n",
            analysis.statistics.total_files
        ));
        context.push_str(&format!(
            "- Dependencies: {}\n",
            analysis.statistics.package_count
        ));
        context.push('\n');

        // File breakdown by extension
        let mut ext_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for file in &project.files {
            *ext_counts.entry(&file.language).or_insert(0) += 1;
        }
        if !ext_counts.is_empty() {
            context.push_str("### Files by Type\n");
            for (ext, count) in ext_counts {
                context.push_str(&format!("- .{}: {} files\n", ext, count));
            }
            context.push('\n');
        }

        // Relevant Patterns
        if !analysis.patterns.is_empty() {
            context.push_str("## Relevant Patterns\n\n");
            for pattern in analysis.patterns.iter().take(5) {
                context.push_str(&format!("### {}\n", pattern.title));
                context.push_str(&format!(
                    "**Category:** {} | **Framework:** {}\n",
                    pattern.category, pattern.framework
                ));
                context.push_str(&format!("{}\n\n", pattern.description));
                context.push_str("```\n");
                context.push_str(&pattern.code);
                context.push_str("\n```\n\n");
            }
        }

        // Suggestions
        if !analysis.suggestions.is_empty() {
            context.push_str("## Suggestions\n\n");
            for suggestion in &analysis.suggestions {
                let icon = match suggestion.severity {
                    SeverityLevel::Error => "❌",
                    SeverityLevel::Warning => "⚠️",
                    SeverityLevel::Info => "ℹ️",
                };
                context.push_str(&format!(
                    "{} **{}**: {}\n",
                    icon, suggestion.category, suggestion.message
                ));
            }
        }

        context
    }

    /// Build a compact single-line context string for Endless Mode.
    ///
    /// Targets ~100-200 characters to achieve ~95% token reduction
    /// compared to `build_generic_context_string`.
    pub fn build_compact_context_string(&self, analysis: &AnalysisResult) -> String {
        let project = &analysis.project;

        let version_str = project.version.as_deref().unwrap_or("?");
        let ident = format!(
            "[{}:{} v{}]",
            project.project_type.as_str().to_uppercase(),
            project.name,
            version_str
        );

        let prod_deps = project.dependencies.iter().filter(|d| !d.dev_only).count();
        let dev_deps = project.dependencies.iter().filter(|d| d.dev_only).count();

        let edition = project
            .metadata
            .rust_edition
            .as_deref()
            .or(project.metadata.target_framework.as_deref())
            .or(project.metadata.node_version.as_deref())
            .unwrap_or("?");

        let entry = project.metadata.entry_point.as_deref().unwrap_or("?");

        let pattern_summary = if analysis.patterns.is_empty() {
            "none".to_string()
        } else {
            analysis
                .patterns
                .iter()
                .take(3)
                .map(|p| p.title.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };

        let err_count = analysis
            .suggestions
            .iter()
            .filter(|s| matches!(s.severity, SeverityLevel::Error))
            .count();
        let warn_count = analysis
            .suggestions
            .iter()
            .filter(|s| matches!(s.severity, SeverityLevel::Warning))
            .count();

        format!(
            "{} files:{} deps:{}({}dev) edition:{} entry:{} | patterns:[{}] suggestions:{}({}err,{}warn)",
            ident,
            analysis.statistics.total_files,
            prod_deps,
            dev_deps,
            edition,
            entry,
            pattern_summary,
            analysis.suggestions.len(),
            err_count,
            warn_count
        )
    }

    // ========================================================================
    // Legacy .NET-specific methods (kept for compatibility)
    // ========================================================================

    /// Build complete analysis with patterns and suggestions
    #[allow(dead_code)]
    pub async fn build_analysis(
        &self,
        project: DotNetProject,
    ) -> Result<crate::types::DotNetAnalysisResult> {
        // Detect framework type from packages
        let framework_type = self.detect_framework(&project);

        // Get relevant patterns
        let patterns = if let Some(ref manager) = self.training_manager {
            self.get_relevant_patterns(manager, &framework_type, &project)?
        } else {
            vec![]
        };

        // Generate suggestions
        let suggestions = self.generate_suggestions(&project, &framework_type);

        // Collect statistics
        let statistics = Statistics {
            total_files: project.files.len(),
            total_classes: project.files.iter().map(|f| f.classes.len()).sum(),
            total_methods: project
                .files
                .iter()
                .flat_map(|f| &f.classes)
                .map(|c| c.methods.len())
                .sum(),
            total_lines: 0,
            framework_version: project.target_framework.clone(),
            package_count: project.packages.len(),
        };

        Ok(crate::types::DotNetAnalysisResult {
            project,
            patterns,
            suggestions,
            statistics,
        })
    }

    /// Detect framework type from project packages
    #[allow(dead_code)]
    fn detect_framework(&self, project: &DotNetProject) -> String {
        // Check for Blazor Server
        if project
            .packages
            .iter()
            .any(|p| p.name.contains("AspNetCore.Components"))
        {
            return "blazor-server".to_string();
        }

        // Check for ASP.NET Core
        if project
            .packages
            .iter()
            .any(|p| p.name.contains("AspNetCore"))
        {
            return "aspnet-core".to_string();
        }

        // Check for Entity Framework
        if project
            .packages
            .iter()
            .any(|p| p.name.contains("EntityFrameworkCore"))
        {
            return "entity-framework".to_string();
        }

        // Default
        "dotnet".to_string()
    }

    /// Get relevant patterns based on project analysis
    #[allow(dead_code)]
    fn get_relevant_patterns(
        &self,
        manager: &TrainingManager,
        framework: &str,
        project: &DotNetProject,
    ) -> Result<Vec<CodePattern>> {
        let mut all_patterns = Vec::new();

        // Search for framework-specific patterns
        let criteria = SearchCriteria {
            query: None,
            category: None,
            framework: Some(framework.to_string()),
            tags: vec![],
            min_score: 0.7, // Only high-quality patterns
        };

        let scored_patterns = manager.search_patterns(&criteria);

        // Detect specific needs based on project structure
        let mut additional_categories = Vec::new();

        // Check for lifecycle methods usage
        for file in &project.files {
            for class in &file.classes {
                for method in &class.methods {
                    if method.name.contains("OnInitialized") {
                        additional_categories.push("lifecycle");
                    }
                    if method.is_async {
                        additional_categories.push("async-patterns");
                    }
                }
            }
        }

        // Get patterns for detected categories
        for category in additional_categories {
            let results = manager.search_by_framework_and_category(framework, category);
            for pattern in results {
                if !all_patterns
                    .iter()
                    .any(|p: &CodePattern| p.id == pattern.id)
                {
                    all_patterns.push(pattern.clone());
                }
            }
        }

        // Add scored patterns
        for (pattern, _score) in scored_patterns.into_iter().take(5) {
            if !all_patterns.iter().any(|p| p.id == pattern.id) {
                all_patterns.push(pattern.clone());
            }
        }

        Ok(all_patterns)
    }

    /// Generate suggestions based on project analysis
    #[allow(dead_code)]
    fn generate_suggestions(&self, project: &DotNetProject, framework: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Check for Blazor-specific issues
        if framework == "blazor-server" {
            suggestions.extend(self.check_blazor_patterns(project));
        }

        // Check for async/await patterns
        suggestions.extend(self.check_async_patterns(project));

        // Check for dependency injection usage
        suggestions.extend(self.check_di_patterns(project));

        suggestions
    }

    #[allow(dead_code)]
    fn check_blazor_patterns(&self, project: &DotNetProject) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        for file in &project.files {
            for class in &file.classes {
                // Check if inherits from ComponentBase
                let is_component = class
                    .base_class
                    .as_ref()
                    .map(|b| b.contains("ComponentBase"))
                    .unwrap_or(false);

                if is_component {
                    // Check for synchronous OnInitialized
                    let has_sync_init = class
                        .methods
                        .iter()
                        .any(|m| m.name == "OnInitialized" && !m.is_async);

                    if has_sync_init {
                        suggestions.push(Suggestion {
                            severity: SeverityLevel::Warning,
                            category: "blazor-lifecycle".to_string(),
                            message: format!(
                                "Component '{}' uses synchronous OnInitialized(). Consider using OnInitializedAsync() for better performance.",
                                class.name
                            ),
                            file: Some(file.path.clone()),
                            line: None,
                        });
                    }
                }
            }
        }

        suggestions
    }

    #[allow(dead_code)]
    fn check_async_patterns(&self, project: &DotNetProject) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        for file in &project.files {
            for class in &file.classes {
                for method in &class.methods {
                    // Check for async void (should be async Task)
                    if method.is_async && method.return_type == "void" {
                        suggestions.push(Suggestion {
                            severity: SeverityLevel::Warning,
                            category: "async-patterns".to_string(),
                            message: format!(
                                "Method '{}' in class '{}' is async void. Use async Task instead for proper exception handling.",
                                method.name, class.name
                            ),
                            file: Some(file.path.clone()),
                            line: None,
                        });
                    }
                }
            }
        }

        suggestions
    }

    #[allow(dead_code)]
    fn check_di_patterns(&self, _project: &DotNetProject) -> Vec<Suggestion> {
        // Could check for:
        // - Services that should be injected
        // - Missing DI registration
        // - Incorrect service lifetime

        // For now, just a placeholder
        vec![Suggestion {
            severity: SeverityLevel::Info,
            category: "dependency-injection".to_string(),
            message: "Consider using dependency injection for data access and external services."
                .to_string(),
            file: None,
            line: None,
        }]
    }

    /// Build a formatted context string for AI consumption (legacy .NET version)
    #[allow(dead_code)]
    pub fn build_context_string(&self, analysis: &crate::types::DotNetAnalysisResult) -> String {
        let mut context = String::new();

        context.push_str("# .NET Project Analysis\n\n");
        context.push_str(&format!("**Project:** {}\n", analysis.project.name));
        context.push_str(&format!(
            "**Framework:** {}\n",
            analysis.project.target_framework
        ));
        context.push_str(&format!(
            "**Language:** C# {}\n\n",
            analysis.project.language_version
        ));

        // Dependencies
        context.push_str("## Dependencies\n\n");
        for package in &analysis.project.packages {
            context.push_str(&format!("- {} ({})\n", package.name, package.version));
        }
        context.push('\n');

        // Statistics
        context.push_str("## Project Statistics\n\n");
        context.push_str(&format!(
            "- Total Files: {}\n",
            analysis.statistics.total_files
        ));
        context.push_str(&format!(
            "- Total Classes: {}\n",
            analysis.statistics.total_classes
        ));
        context.push_str(&format!(
            "- Total Methods: {}\n",
            analysis.statistics.total_methods
        ));
        context.push('\n');

        // Relevant Patterns
        if !analysis.patterns.is_empty() {
            context.push_str("## Relevant Patterns\n\n");
            for pattern in &analysis.patterns {
                context.push_str(&format!("### {}\n", pattern.title));
                context.push_str(&format!("**Category:** {}\n", pattern.category));
                context.push_str(&format!("{}\n\n", pattern.description));
                context.push_str("```csharp\n");
                context.push_str(&pattern.code);
                context.push_str("\n```\n\n");
            }
        }

        // Suggestions
        if !analysis.suggestions.is_empty() {
            context.push_str("## Suggestions\n\n");
            for suggestion in &analysis.suggestions {
                let icon = match suggestion.severity {
                    SeverityLevel::Error => "❌",
                    SeverityLevel::Warning => "⚠️",
                    SeverityLevel::Info => "ℹ️",
                };
                context.push_str(&format!(
                    "{} **{}**: {}\n",
                    icon, suggestion.category, suggestion.message
                ));
            }
        }

        context
    }
}
