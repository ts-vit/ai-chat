use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::types::*;

/// Registry of available Python scripts.
pub struct ScriptRegistry {
    scripts: HashMap<String, RegisteredScript>,
}

impl ScriptRegistry {
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
        }
    }

    /// Scan a directory for scripts (each subdirectory with manifest.toml).
    pub async fn scan_directory(
        &mut self,
        dir: &Path,
        source: ScriptSource,
    ) -> Result<Vec<String>, uni_common::UniError> {
        let mut registered = Vec::new();

        if !dir.exists() {
            return Ok(registered);
        }

        let mut entries = tokio::fs::read_dir(dir)
            .await
            .map_err(uni_common::UniError::Io)?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(uni_common::UniError::Io)?
        {
            if !entry
                .file_type()
                .await
                .map_err(uni_common::UniError::Io)?
                .is_dir()
            {
                continue;
            }

            let script_dir = entry.path();
            let manifest_path = script_dir.join("manifest.toml");

            if !manifest_path.exists() {
                continue;
            }

            match self
                .register_from_manifest(&manifest_path, source.clone())
                .await
            {
                Ok(name) => registered.push(name),
                Err(e) => {
                    log::warn!("Failed to register script from {:?}: {}", manifest_path, e);
                }
            }
        }

        Ok(registered)
    }

    /// Register a script from its manifest.toml path.
    pub async fn register_from_manifest(
        &mut self,
        manifest_path: &Path,
        source: ScriptSource,
    ) -> Result<String, uni_common::UniError> {
        let content = tokio::fs::read_to_string(manifest_path)
            .await
            .map_err(uni_common::UniError::Io)?;

        let manifest: ScriptManifest = toml::from_str(&content).map_err(|e| {
            uni_common::UniError::Generic(format!("Invalid manifest {:?}: {}", manifest_path, e))
        })?;

        let script_dir = manifest_path
            .parent()
            .ok_or_else(|| uni_common::UniError::Generic("No parent dir".to_string()))?
            .to_path_buf();

        let entry_path = script_dir.join(&manifest.script.entry);
        if !entry_path.exists() {
            return Err(uni_common::UniError::Generic(format!(
                "Entry point not found: {:?}",
                entry_path
            )));
        }

        let requirements_path = manifest
            .requirements
            .as_ref()
            .map(|r| script_dir.join(&r.file));

        let name = manifest.script.name.clone();

        self.scripts.insert(
            name.clone(),
            RegisteredScript {
                manifest,
                dir: script_dir,
                entry_path,
                requirements_path,
                source,
            },
        );

        Ok(name)
    }

    /// Get a registered script by name.
    pub fn get(&self, name: &str) -> Option<&RegisteredScript> {
        self.scripts.get(name)
    }

    /// List all registered scripts.
    pub fn list(&self) -> Vec<&RegisteredScript> {
        self.scripts.values().collect()
    }

    /// Remove a script by name. Returns true if it was removed.
    pub fn remove(&mut self, name: &str) -> bool {
        self.scripts.remove(name).is_some()
    }

    /// Check if a script is registered.
    pub fn has(&self, name: &str) -> bool {
        self.scripts.contains_key(name)
    }

    /// Get all requirements.txt paths (for bulk dependency installation).
    pub fn all_requirements_paths(&self) -> Vec<&PathBuf> {
        self.scripts
            .values()
            .filter_map(|s| s.requirements_path.as_ref())
            .collect()
    }
}

impl Default for ScriptRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_script(base: &Path, name: &str) -> PathBuf {
        let dir = base.join(name);
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let manifest = format!(
            r#"
[script]
name = "{name}"
version = "1.0.0"
entry = "main.py"
mode = "one-shot"

[requirements]
file = "requirements.txt"
"#
        );
        tokio::fs::write(dir.join("manifest.toml"), manifest)
            .await
            .unwrap();
        tokio::fs::write(dir.join("main.py"), "# script")
            .await
            .unwrap();
        tokio::fs::write(dir.join("requirements.txt"), "requests>=2.0")
            .await
            .unwrap();
        dir
    }

    #[tokio::test]
    async fn test_register_from_manifest() {
        let base = std::env::temp_dir().join("uni-python-registry-test-1");
        let _ = tokio::fs::remove_dir_all(&base).await;
        let dir = create_test_script(&base, "test-script").await;

        let mut registry = ScriptRegistry::new();
        let name = registry
            .register_from_manifest(&dir.join("manifest.toml"), ScriptSource::Bundled)
            .await
            .unwrap();

        assert_eq!(name, "test-script");
        assert!(registry.has("test-script"));

        let script = registry.get("test-script").unwrap();
        assert_eq!(script.manifest.script.version, "1.0.0");
        assert_eq!(script.source, ScriptSource::Bundled);
        assert!(script.requirements_path.is_some());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_scan_directory() {
        let base = std::env::temp_dir().join("uni-python-registry-test-2");
        let _ = tokio::fs::remove_dir_all(&base).await;
        create_test_script(&base, "script-a").await;
        create_test_script(&base, "script-b").await;

        let mut registry = ScriptRegistry::new();
        let names = registry
            .scan_directory(&base, ScriptSource::Custom)
            .await
            .unwrap();

        assert_eq!(names.len(), 2);
        assert!(registry.has("script-a"));
        assert!(registry.has("script-b"));
        assert_eq!(registry.list().len(), 2);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_remove_script() {
        let base = std::env::temp_dir().join("uni-python-registry-test-3");
        let _ = tokio::fs::remove_dir_all(&base).await;
        create_test_script(&base, "removable").await;

        let mut registry = ScriptRegistry::new();
        registry
            .scan_directory(&base, ScriptSource::Custom)
            .await
            .unwrap();
        assert!(registry.has("removable"));

        assert!(registry.remove("removable"));
        assert!(!registry.has("removable"));
        assert!(!registry.remove("nonexistent"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_scan_empty_directory() {
        let base = std::env::temp_dir().join("uni-python-registry-test-4");
        tokio::fs::create_dir_all(&base).await.unwrap();

        let mut registry = ScriptRegistry::new();
        let names = registry
            .scan_directory(&base, ScriptSource::Bundled)
            .await
            .unwrap();
        assert!(names.is_empty());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_scan_nonexistent_directory() {
        let mut registry = ScriptRegistry::new();
        let names = registry
            .scan_directory(&PathBuf::from("/nonexistent/path"), ScriptSource::Bundled)
            .await
            .unwrap();
        assert!(names.is_empty());
    }

    #[tokio::test]
    async fn test_all_requirements_paths() {
        let base = std::env::temp_dir().join("uni-python-registry-test-5");
        let _ = tokio::fs::remove_dir_all(&base).await;
        create_test_script(&base, "with-reqs").await;

        let mut registry = ScriptRegistry::new();
        registry
            .scan_directory(&base, ScriptSource::Bundled)
            .await
            .unwrap();

        let req_paths = registry.all_requirements_paths();
        assert_eq!(req_paths.len(), 1);

        let _ = tokio::fs::remove_dir_all(&base).await;
    }
}
