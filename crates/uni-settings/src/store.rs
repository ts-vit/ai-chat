use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uni_common::UniError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub is_sensitive: bool,
}

#[async_trait]
pub trait SettingsStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, UniError>;
    async fn set(&self, key: &str, value: &str) -> Result<(), UniError>;
    async fn delete(&self, key: &str) -> Result<(), UniError>;
    async fn list(&self, prefix: &str) -> Result<Vec<Setting>, UniError>;
}

pub struct JsonSettingsStore {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

const SENSITIVE_PATTERNS: &[&str] = &["api_key", "_key", "password", "token", "secret"];

fn is_sensitive_key(key: &str) -> bool {
    SENSITIVE_PATTERNS.iter().any(|p| key.contains(p))
}

fn mask_value(value: &str) -> String {
    if value.len() > 8 {
        let prefix: String = value.chars().take(4).collect();
        format!("{prefix}***")
    } else {
        "***".to_string()
    }
}

impl JsonSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Arc::new(Mutex::new(())),
        }
    }

    fn read_map(&self) -> Result<HashMap<String, String>, UniError> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }
        let data = std::fs::read_to_string(&self.path)?;
        let map: HashMap<String, String> = serde_json::from_str(&data)?;
        Ok(map)
    }

    fn write_map(&self, map: &HashMap<String, String>) -> Result<(), UniError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("tmp");
        let data = serde_json::to_string_pretty(map)?;
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[async_trait]
impl SettingsStore for JsonSettingsStore {
    async fn get(&self, key: &str) -> Result<Option<String>, UniError> {
        let map = self.read_map()?;
        Ok(map.get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), UniError> {
        let _guard = self.lock.lock().await;
        let mut map = self.read_map()?;
        map.insert(key.to_string(), value.to_string());
        self.write_map(&map)
    }

    async fn delete(&self, key: &str) -> Result<(), UniError> {
        let _guard = self.lock.lock().await;
        let mut map = self.read_map()?;
        map.remove(key);
        self.write_map(&map)
    }

    async fn list(&self, prefix: &str) -> Result<Vec<Setting>, UniError> {
        let map = self.read_map()?;
        let mut settings: Vec<Setting> = map
            .into_iter()
            .filter(|(k, _)| prefix.is_empty() || k.starts_with(prefix))
            .map(|(k, v)| {
                let sensitive = is_sensitive_key(&k);
                Setting {
                    key: k,
                    value: if sensitive { mask_value(&v) } else { v },
                    is_sensitive: sensitive,
                }
            })
            .collect();
        settings.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store(dir: &tempfile::TempDir) -> JsonSettingsStore {
        JsonSettingsStore::new(dir.path().join("settings.json"))
    }

    #[tokio::test]
    async fn test_set_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(&dir);
        store.set("foo", "bar").await.unwrap();
        assert_eq!(store.get("foo").await.unwrap(), Some("bar".to_string()));
    }

    #[tokio::test]
    async fn test_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(&dir);
        store.set("foo", "bar").await.unwrap();
        store.delete("foo").await.unwrap();
        assert_eq!(store.get("foo").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_list_all() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(&dir);
        store.set("a", "1").await.unwrap();
        store.set("b", "2").await.unwrap();
        store.set("c", "3").await.unwrap();
        let list = store.list("").await.unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn test_list_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(&dir);
        store.set("llm.model", "gpt-4").await.unwrap();
        store.set("llm.temperature", "0.7").await.unwrap();
        store.set("ui.language", "en").await.unwrap();
        let list = store.list("llm.").await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|s| s.key.starts_with("llm.")));
    }

    #[tokio::test]
    async fn test_sensitive_masking() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(&dir);
        store
            .set("llm.openrouter.api_key", "sk-or-abc123xyz")
            .await
            .unwrap();

        // get() returns the original value
        assert_eq!(
            store.get("llm.openrouter.api_key").await.unwrap(),
            Some("sk-or-abc123xyz".to_string())
        );

        // list() masks sensitive values
        let list = store.list("").await.unwrap();
        let setting = list
            .iter()
            .find(|s| s.key == "llm.openrouter.api_key")
            .unwrap();
        assert!(setting.is_sensitive);
        assert_eq!(setting.value, "sk-o***");
    }

    #[tokio::test]
    async fn test_atomic_write() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(&dir);
        store.set("key", "value").await.unwrap();

        // No .tmp file should remain
        let tmp_path = dir.path().join("settings.json.tmp");
        assert!(!tmp_path.exists());

        // Main file should be valid JSON
        let data = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let parsed: HashMap<String, String> = serde_json::from_str(&data).unwrap();
        assert_eq!(parsed.get("key").unwrap(), "value");
    }

    #[tokio::test]
    async fn test_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = make_store(&dir);
        assert_eq!(store.get("anything").await.unwrap(), None);
    }
}
