use crate::types::ProjectType;
use std::path::Path;

/// Detects the type of project based on configuration files present
pub struct ProjectDetector;

impl ProjectDetector {
    /// Detect project type from a directory path
    pub fn detect(path: &Path) -> ProjectType {
        // Check for various project indicators (in priority order)

        // .NET: .csproj, .fsproj, .sln
        if Self::has_extension(path, "csproj")
            || Self::has_extension(path, "fsproj")
            || Self::has_extension(path, "sln")
        {
            return ProjectType::DotNet;
        }

        // Rust: Cargo.toml
        if path.join("Cargo.toml").exists() {
            return ProjectType::Rust;
        }

        // PHP: composer.json (check before Node because some PHP projects have package.json too)
        if path.join("composer.json").exists() {
            return ProjectType::Php;
        }

        // Node.js: package.json
        if path.join("package.json").exists() {
            return ProjectType::Node;
        }

        // Python: pyproject.toml, setup.py, requirements.txt
        if path.join("pyproject.toml").exists()
            || path.join("setup.py").exists()
            || path.join("requirements.txt").exists()
        {
            return ProjectType::Python;
        }

        // Go: go.mod
        if path.join("go.mod").exists() {
            return ProjectType::Go;
        }

        // Java: pom.xml (Maven) or build.gradle (Gradle)
        if path.join("pom.xml").exists()
            || path.join("build.gradle").exists()
            || path.join("build.gradle.kts").exists()
        {
            return ProjectType::Java;
        }

        ProjectType::Unknown
    }

    /// Check if directory contains a file with the given extension
    fn has_extension(path: &Path, ext: &str) -> bool {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Some(file_ext) = entry.path().extension() {
                    if file_ext == ext {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get the file extensions to analyze for each project type
    pub fn get_source_extensions(project_type: &ProjectType) -> Vec<&'static str> {
        match project_type {
            ProjectType::DotNet => vec!["cs", "fs", "vb", "razor"],
            ProjectType::Rust => vec!["rs"],
            ProjectType::Node => vec!["js", "ts", "jsx", "tsx", "mjs", "cjs", "vue", "svelte"],
            ProjectType::Python => vec!["py", "pyi"],
            ProjectType::Go => vec!["go"],
            ProjectType::Java => vec!["java", "kt", "kts", "scala"],
            ProjectType::Php => vec!["php", "blade.php", "twig", "js", "ts", "vue"],
            ProjectType::Unknown => vec![],
        }
    }

    /// Get the primary config file for each project type
    #[allow(dead_code)]
    pub fn get_config_file(project_type: &ProjectType) -> Option<&'static str> {
        match project_type {
            ProjectType::DotNet => None, // .csproj varies by project name
            ProjectType::Rust => Some("Cargo.toml"),
            ProjectType::Node => Some("package.json"),
            ProjectType::Python => Some("pyproject.toml"),
            ProjectType::Go => Some("go.mod"),
            ProjectType::Java => Some("pom.xml"),
            ProjectType::Php => Some("composer.json"),
            ProjectType::Unknown => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_detect_rust_project() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        assert_eq!(ProjectDetector::detect(dir.path()), ProjectType::Rust);
    }

    #[test]
    fn test_detect_node_project() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        assert_eq!(ProjectDetector::detect(dir.path()), ProjectType::Node);
    }

    #[test]
    fn test_detect_unknown() {
        let dir = tempdir().unwrap();

        assert_eq!(ProjectDetector::detect(dir.path()), ProjectType::Unknown);
    }
}
