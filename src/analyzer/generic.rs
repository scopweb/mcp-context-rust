use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::detector::ProjectDetector;
#[allow(unused_imports)]
use crate::types::{
    Dependency, Project, ProjectMetadata, ProjectType, SourceFile, Symbol, SymbolKind,
};

/// Generic project analyzer that works with any project type.
///
/// Supports multiple project types including Rust, Node.js, Python, .NET,
/// Go, Java, and PHP projects.
pub struct GenericAnalyzer;

impl GenericAnalyzer {
    /// Analyze a project directory and return a generic Project struct.
    ///
    /// # Arguments
    /// * `path` - Path to the project root directory
    ///
    /// # Returns
    /// A `Project` struct containing detected type, dependencies, and metadata
    ///
    /// # Errors
    /// Returns an error if the project cannot be analyzed or required files are missing
    pub async fn analyze(path: &Path) -> Result<Project> {
        // Detect project type
        let project_type = ProjectDetector::detect(path);

tracing::debug!(project_type = ?project_type, "Detected project type");

        // Get project info based on type
        let (name, version, dependencies, metadata) = match project_type {
            ProjectType::DotNet => Self::parse_dotnet_project(path)?,
            ProjectType::Rust => Self::parse_rust_project(path)?,
            ProjectType::Node => Self::parse_node_project(path)?,
            ProjectType::Python => Self::parse_python_project(path)?,
            ProjectType::Go => Self::parse_go_project(path)?,
            ProjectType::Java => Self::parse_java_project(path)?,
            ProjectType::Php => Self::parse_php_project(path)?,
            ProjectType::Unknown => Self::parse_unknown_project(path)?,
        };

        // Find and analyze source files
        let extensions = ProjectDetector::get_source_extensions(&project_type);
        let files = Self::find_and_analyze_files(path, &extensions)?;

        Ok(Project {
            path: path.to_path_buf(),
            name,
            project_type,
            version,
            dependencies,
            files,
            metadata,
        })
    }

    // ========================================================================
    // Project-specific parsers
    // ========================================================================

    fn parse_dotnet_project(
        path: &Path,
    ) -> Result<(String, Option<String>, Vec<Dependency>, ProjectMetadata)> {
        // Find .csproj file
        let csproj = Self::find_file_with_extension(path, "csproj")?;
        let content = fs::read_to_string(&csproj)?;

        let name = csproj
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Parse target framework
        let target_framework = Self::extract_xml_value(&content, "TargetFramework")
            .or_else(|| Self::extract_xml_value(&content, "TargetFrameworks"));

        // Parse packages
        let dependencies = Self::parse_nuget_packages(&content);

        let metadata = ProjectMetadata {
            target_framework,
            ..Default::default()
        };

        Ok((name, None, dependencies, metadata))
    }

    fn parse_rust_project(
        path: &Path,
    ) -> Result<(String, Option<String>, Vec<Dependency>, ProjectMetadata)> {
        let cargo_toml = path.join("Cargo.toml");
        let content = fs::read_to_string(&cargo_toml).context("Failed to read Cargo.toml")?;

        // Simple TOML parsing for name and version
        let name =
            Self::extract_toml_value(&content, "name").unwrap_or_else(|| "Unknown".to_string());
        let version = Self::extract_toml_value(&content, "version");
        let edition = Self::extract_toml_value(&content, "edition");

        // Parse dependencies
        let dependencies = Self::parse_cargo_dependencies(&content);

        let metadata = ProjectMetadata {
            rust_edition: edition,
            entry_point: Some("src/main.rs".to_string()),
            build_command: Some("cargo build".to_string()),
            ..Default::default()
        };

        Ok((name, version, dependencies, metadata))
    }

    fn parse_node_project(
        path: &Path,
    ) -> Result<(String, Option<String>, Vec<Dependency>, ProjectMetadata)> {
        let package_json = path.join("package.json");
        let content = fs::read_to_string(&package_json).context("Failed to read package.json")?;

        let json: serde_json::Value =
            serde_json::from_str(&content).context("Failed to parse package.json")?;

        let name = json["name"].as_str().unwrap_or("Unknown").to_string();
        let version = json["version"].as_str().map(|s| s.to_string());

        // Parse dependencies
        let mut dependencies = Vec::new();

        if let Some(deps) = json["dependencies"].as_object() {
            for (name, version) in deps {
                dependencies.push(Dependency {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("*").to_string(),
                    dev_only: false,
                });
            }
        }

        if let Some(deps) = json["devDependencies"].as_object() {
            for (name, version) in deps {
                dependencies.push(Dependency {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("*").to_string(),
                    dev_only: true,
                });
            }
        }

        let mut metadata = ProjectMetadata {
            entry_point: json["main"].as_str().map(|s| s.to_string()),
            ..Default::default()
        };

        // Detect framework
        if json["dependencies"].get("react").is_some() {
            metadata
                .extra
                .insert("framework".to_string(), "react".to_string());
        } else if json["dependencies"].get("vue").is_some() {
            metadata
                .extra
                .insert("framework".to_string(), "vue".to_string());
        } else if json["dependencies"].get("next").is_some() {
            metadata
                .extra
                .insert("framework".to_string(), "next".to_string());
        }

        Ok((name, version, dependencies, metadata))
    }

    fn parse_python_project(
        path: &Path,
    ) -> Result<(String, Option<String>, Vec<Dependency>, ProjectMetadata)> {
        let mut name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let mut version = None;
        let mut dependencies = Vec::new();
        let mut metadata = ProjectMetadata::default();

        // Try pyproject.toml first
        let pyproject = path.join("pyproject.toml");
        if pyproject.exists() {
            let content = fs::read_to_string(&pyproject)?;

            if let Some(n) = Self::extract_toml_value(&content, "name") {
                name = n;
            }
            version = Self::extract_toml_value(&content, "version");

            // Parse dependencies from pyproject.toml
            // (simplified - real parsing would need proper TOML parser)
        }

        // Try requirements.txt
        let requirements = path.join("requirements.txt");
        if requirements.exists() {
            let content = fs::read_to_string(&requirements)?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Parse "package==version" or "package>=version" etc.
                let (pkg_name, pkg_version) = if let Some(pos) = line.find("==") {
                    (line[..pos].to_string(), line[pos + 2..].to_string())
                } else if let Some(pos) = line.find(">=") {
                    (line[..pos].to_string(), line[pos + 2..].to_string())
                } else {
                    (line.to_string(), "*".to_string())
                };

                dependencies.push(Dependency {
                    name: pkg_name,
                    version: pkg_version,
                    dev_only: false,
                });
            }
        }

        metadata.entry_point = Some("main.py".to_string());
        metadata.build_command = Some("python main.py".to_string());

        Ok((name, version, dependencies, metadata))
    }

    fn parse_go_project(
        path: &Path,
    ) -> Result<(String, Option<String>, Vec<Dependency>, ProjectMetadata)> {
        let go_mod = path.join("go.mod");
        let content = fs::read_to_string(&go_mod).context("Failed to read go.mod")?;

        // Parse module name from first line
        let name = content
            .lines()
            .find(|l| l.starts_with("module "))
            .map(|l| l.strip_prefix("module ").unwrap_or(l).trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        // Parse Go version
        let go_version = content
            .lines()
            .find(|l| l.starts_with("go "))
            .map(|l| l.strip_prefix("go ").unwrap_or(l).trim().to_string());

        // Parse require block for dependencies
        let mut dependencies = Vec::new();
        let mut in_require = false;

        for line in content.lines() {
            let line = line.trim();

            if line == "require (" {
                in_require = true;
                continue;
            }
            if line == ")" {
                in_require = false;
                continue;
            }

            if in_require && !line.is_empty() && !line.starts_with("//") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    dependencies.push(Dependency {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                        dev_only: false,
                    });
                }
            }
        }

        let mut metadata = ProjectMetadata::default();
        if let Some(v) = go_version {
            metadata.extra.insert("go_version".to_string(), v);
        }
        metadata.entry_point = Some("main.go".to_string());
        metadata.build_command = Some("go build".to_string());

        Ok((name, None, dependencies, metadata))
    }

    fn parse_java_project(
        path: &Path,
    ) -> Result<(String, Option<String>, Vec<Dependency>, ProjectMetadata)> {
        let mut name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let mut version = None;
        let dependencies = Vec::new();
        let mut metadata = ProjectMetadata::default();

        // Try pom.xml (Maven)
        let pom = path.join("pom.xml");
        if pom.exists() {
            let content = fs::read_to_string(&pom)?;

            if let Some(n) = Self::extract_xml_value(&content, "artifactId") {
                name = n;
            }
            version = Self::extract_xml_value(&content, "version");

            metadata.build_command = Some("mvn package".to_string());
        }

        // Try build.gradle
        let gradle = path.join("build.gradle");
        if gradle.exists() {
            metadata.build_command = Some("gradle build".to_string());
        }

        Ok((name, version, dependencies, metadata))
    }

    fn parse_php_project(
        path: &Path,
    ) -> Result<(String, Option<String>, Vec<Dependency>, ProjectMetadata)> {
        let composer_json = path.join("composer.json");
        let content = fs::read_to_string(&composer_json).context("Failed to read composer.json")?;

        let json: serde_json::Value =
            serde_json::from_str(&content).context("Failed to parse composer.json")?;

        // Get project name (format: vendor/package)
        let name = json["name"].as_str().unwrap_or("Unknown").to_string();
        let version = json["version"].as_str().map(|s| s.to_string());

        // Parse dependencies
        let mut dependencies = Vec::new();

        if let Some(deps) = json["require"].as_object() {
            for (pkg_name, pkg_version) in deps {
                // Skip PHP version requirement
                if pkg_name == "php" {
                    continue;
                }
                dependencies.push(Dependency {
                    name: pkg_name.clone(),
                    version: pkg_version.as_str().unwrap_or("*").to_string(),
                    dev_only: false,
                });
            }
        }

        if let Some(deps) = json["require-dev"].as_object() {
            for (pkg_name, pkg_version) in deps {
                dependencies.push(Dependency {
                    name: pkg_name.clone(),
                    version: pkg_version.as_str().unwrap_or("*").to_string(),
                    dev_only: true,
                });
            }
        }

        let mut metadata = ProjectMetadata::default();

        // Detect PHP version requirement
        if let Some(deps) = json["require"].as_object() {
            if let Some(php_ver) = deps.get("php") {
                metadata.extra.insert(
                    "php_version".to_string(),
                    php_ver.as_str().unwrap_or("*").to_string(),
                );
            }
        }

        // Detect framework
        let framework = Self::detect_php_framework(&dependencies, path);
        if let Some(fw) = framework {
            metadata.extra.insert("framework".to_string(), fw);
        }

        // Check for frontend (Vue, etc.)
        let package_json = path.join("package.json");
        if package_json.exists() {
            if let Ok(pkg_content) = fs::read_to_string(&package_json) {
                if let Ok(pkg_json) = serde_json::from_str::<serde_json::Value>(&pkg_content) {
                    // Check for Vue
                    if pkg_json["dependencies"].get("vue").is_some()
                        || pkg_json["devDependencies"].get("vue").is_some()
                    {
                        metadata
                            .extra
                            .insert("frontend".to_string(), "vue".to_string());
                    }
                    // Check for React (Inertia.js often uses React)
                    if pkg_json["dependencies"].get("react").is_some()
                        || pkg_json["devDependencies"].get("react").is_some()
                    {
                        metadata
                            .extra
                            .insert("frontend".to_string(), "react".to_string());
                    }
                    // Check for Vite
                    if pkg_json["devDependencies"].get("vite").is_some() {
                        metadata
                            .extra
                            .insert("bundler".to_string(), "vite".to_string());
                    }
                    // Check for Laravel Mix / Webpack
                    if pkg_json["devDependencies"].get("laravel-mix").is_some() {
                        metadata
                            .extra
                            .insert("bundler".to_string(), "laravel-mix".to_string());
                    }
                }
            }
        }

        // Set entry point based on framework
        if metadata
            .extra
            .get("framework")
            .map(|f| f == "laravel")
            .unwrap_or(false)
        {
            metadata.entry_point = Some("public/index.php".to_string());
            metadata.build_command = Some("php artisan serve".to_string());
        } else if metadata
            .extra
            .get("framework")
            .map(|f| f == "symfony")
            .unwrap_or(false)
        {
            metadata.entry_point = Some("public/index.php".to_string());
            metadata.build_command = Some("symfony server:start".to_string());
        } else {
            metadata.entry_point = Some("index.php".to_string());
            metadata.build_command = Some("php -S localhost:8000".to_string());
        }

        Ok((name, version, dependencies, metadata))
    }

    /// Detect PHP framework from dependencies and directory structure
    fn detect_php_framework(dependencies: &[Dependency], path: &Path) -> Option<String> {
        // Laravel detection
        if dependencies.iter().any(|d| d.name == "laravel/framework") {
            return Some("laravel".to_string());
        }
        if path.join("artisan").exists() {
            return Some("laravel".to_string());
        }

        // Symfony detection
        if dependencies.iter().any(|d| d.name.starts_with("symfony/")) {
            // Check if it's a full Symfony app or just uses components
            if dependencies
                .iter()
                .any(|d| d.name == "symfony/framework-bundle")
            {
                return Some("symfony".to_string());
            }
        }

        // WordPress detection
        if path.join("wp-config.php").exists() || path.join("wp-content").exists() {
            return Some("wordpress".to_string());
        }

        // CodeIgniter detection
        if dependencies
            .iter()
            .any(|d| d.name == "codeigniter4/framework")
        {
            return Some("codeigniter".to_string());
        }

        // Yii detection
        if dependencies.iter().any(|d| d.name.starts_with("yiisoft/")) {
            return Some("yii".to_string());
        }

        // CakePHP detection
        if dependencies.iter().any(|d| d.name == "cakephp/cakephp") {
            return Some("cakephp".to_string());
        }

        // Slim detection
        if dependencies.iter().any(|d| d.name == "slim/slim") {
            return Some("slim".to_string());
        }

        // Drupal detection
        if dependencies.iter().any(|d| d.name == "drupal/core") {
            return Some("drupal".to_string());
        }

        None
    }

    fn parse_unknown_project(
        path: &Path,
    ) -> Result<(String, Option<String>, Vec<Dependency>, ProjectMetadata)> {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        Ok((name, None, Vec::new(), ProjectMetadata::default()))
    }

    // ========================================================================
    // Helper methods
    // ========================================================================

    fn find_file_with_extension(path: &Path, ext: &str) -> Result<PathBuf> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if let Some(file_ext) = entry.path().extension() {
                if file_ext == ext {
                    return Ok(entry.path());
                }
            }
        }
        anyhow::bail!("No .{} file found in {}", ext, path.display())
    }

    fn extract_xml_value(content: &str, tag: &str) -> Option<String> {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);

        if let Some(start) = content.find(&open_tag) {
            let value_start = start + open_tag.len();
            if let Some(end) = content[value_start..].find(&close_tag) {
                return Some(content[value_start..value_start + end].trim().to_string());
            }
        }
        None
    }

    fn extract_toml_value(content: &str, key: &str) -> Option<String> {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with(&format!("{} ", key)) || line.starts_with(&format!("{}=", key)) {
                if let Some(pos) = line.find('=') {
                    let value = line[pos + 1..].trim().trim_matches('"');
                    return Some(value.to_string());
                }
            }
        }
        None
    }

    fn parse_nuget_packages(content: &str) -> Vec<Dependency> {
        let mut deps = Vec::new();

        // Match <PackageReference Include="name" Version="version" />
        for line in content.lines() {
            if line.contains("PackageReference") {
                let include = Self::extract_xml_attr(line, "Include");
                let version = Self::extract_xml_attr(line, "Version");

                if let Some(name) = include {
                    deps.push(Dependency {
                        name,
                        version: version.unwrap_or_else(|| "*".to_string()),
                        dev_only: false,
                    });
                }
            }
        }

        deps
    }

    fn extract_xml_attr(line: &str, attr: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr);
        if let Some(start) = line.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = line[value_start..].find('"') {
                return Some(line[value_start..value_start + end].to_string());
            }
        }
        None
    }

    fn parse_cargo_dependencies(content: &str) -> Vec<Dependency> {
        let mut deps = Vec::new();
        let mut in_deps = false;
        let mut in_dev_deps = false;

        for line in content.lines() {
            let line = line.trim();

            if line == "[dependencies]" {
                in_deps = true;
                in_dev_deps = false;
                continue;
            }
            if line == "[dev-dependencies]" {
                in_deps = false;
                in_dev_deps = true;
                continue;
            }
            if line.starts_with('[') {
                in_deps = false;
                in_dev_deps = false;
                continue;
            }

            if (in_deps || in_dev_deps) && !line.is_empty() && !line.starts_with('#') {
                if let Some((name, version)) = Self::parse_cargo_dep_line(line) {
                    deps.push(Dependency {
                        name,
                        version,
                        dev_only: in_dev_deps,
                    });
                }
            }
        }

        deps
    }

    fn parse_cargo_dep_line(line: &str) -> Option<(String, String)> {
        // Handle: name = "version" or name = { version = "1.0", features = [...] }
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            let name = parts[0].trim().to_string();
            let value = parts[1].trim();

            // Simple version string
            if value.starts_with('"') {
                let version = value.trim_matches('"').to_string();
                return Some((name, version));
            }

            // Object with version key
            if value.starts_with('{') {
                if let Some(ver) = Self::extract_toml_value(value, "version") {
                    return Some((name, ver));
                }
            }
        }
        None
    }

    fn find_and_analyze_files(path: &Path, extensions: &[&str]) -> Result<Vec<SourceFile>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_str().unwrap_or("");
                !name.starts_with('.')
                    && name != "node_modules"
                    && name != "target"
                    && name != "bin"
                    && name != "obj"
                    && name != "__pycache__"
                    && name != ".git"
                    && name != "vendor"
            })
        {
            let entry = entry?;
            let file_path = entry.path();

            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext) {
                    let metadata = fs::metadata(file_path)?;

                    files.push(SourceFile {
                        path: file_path.to_path_buf(),
                        language: ext.to_string(),
                        size_bytes: metadata.len(),
                        symbols: Vec::new(), // TODO: Parse symbols with tree-sitter
                    });
                }
            }
        }

        Ok(files)
    }
}
