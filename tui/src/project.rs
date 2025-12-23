use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub path: PathBuf,
    pub name: String,
    #[allow(dead_code)]
    pub is_flutter: bool,
}

#[derive(Debug, Deserialize)]
struct Pubspec {
    name: String,
    #[serde(default)]
    dependencies: Option<PubspecDependencies>,
}

#[derive(Debug, Deserialize)]
struct PubspecDependencies {
    flutter: Option<serde_yaml::Value>,
}

impl ProjectInfo {
    pub fn from_path(path: &Path) -> Result<Self> {
        let pubspec_path = path.join("pubspec.yaml");

        if !pubspec_path.exists() {
            bail!(
                "No pubspec.yaml found at {}. Please run from a Flutter project directory or use --project.",
                path.display()
            );
        }

        let content = std::fs::read_to_string(&pubspec_path)
            .with_context(|| format!("Failed to read {}", pubspec_path.display()))?;

        let pubspec: Pubspec = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", pubspec_path.display()))?;

        let is_flutter = pubspec
            .dependencies
            .as_ref()
            .map(|deps| deps.flutter.is_some())
            .unwrap_or(false);

        if !is_flutter {
            bail!(
                "Project '{}' at {} does not appear to be a Flutter project (no flutter dependency found).",
                pubspec.name,
                path.display()
            );
        }

        Ok(Self {
            path: path.to_path_buf(),
            name: pubspec.name,
            is_flutter,
        })
    }

    pub fn from_path_or_cwd(project_arg: Option<&Path>) -> Result<Self> {
        let path = match project_arg {
            Some(p) => p.to_path_buf(),
            None => std::env::current_dir().context("Failed to get current directory")?,
        };

        let canonical = path
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", path.display()))?;

        Self::from_path(&canonical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_valid_flutter_project() {
        let dir = tempdir().unwrap();
        let pubspec = r#"
name: test_app
dependencies:
  flutter:
    sdk: flutter
"#;
        fs::write(dir.path().join("pubspec.yaml"), pubspec).unwrap();

        let info = ProjectInfo::from_path(dir.path()).unwrap();
        assert_eq!(info.name, "test_app");
        assert!(info.is_flutter);
    }

    #[test]
    fn test_non_flutter_project() {
        let dir = tempdir().unwrap();
        let pubspec = r#"
name: dart_only
dependencies:
  some_package: ^1.0.0
"#;
        fs::write(dir.path().join("pubspec.yaml"), pubspec).unwrap();

        let result = ProjectInfo::from_path(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not appear to be a Flutter"));
    }

    #[test]
    fn test_missing_pubspec() {
        let dir = tempdir().unwrap();
        let result = ProjectInfo::from_path(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No pubspec.yaml"));
    }
}
