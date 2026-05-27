use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::github::Template;
use crate::paths::AppPaths;

#[derive(Clone, Debug)]
pub(crate) struct CacheStore {
    cache_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct ManifestCache {
    fetched_at: u64,
    templates: Vec<Template>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TemplateCache {
    fetched_at: u64,
    content: String,
}

impl CacheStore {
    pub(crate) fn new(paths: &AppPaths) -> Self {
        Self {
            cache_dir: paths.cache_dir.clone(),
        }
    }

    pub(crate) fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .with_context(|| format!("failed to remove {}", self.cache_dir.display()))?;
        }
        Ok(())
    }

    pub(crate) fn load_manifest(&self, ttl: Duration) -> Result<Option<Vec<Template>>> {
        let Some(cache) = self.read_json::<ManifestCache>(&self.manifest_path())? else {
            return Ok(None);
        };

        Ok(is_fresh(cache.fetched_at, ttl).then_some(cache.templates))
    }

    pub(crate) fn save_manifest(&self, templates: &[Template]) -> Result<()> {
        let cache = ManifestCache {
            fetched_at: now_unix(),
            templates: templates.to_vec(),
        };
        self.write_json(&self.manifest_path(), &cache)
    }

    pub(crate) fn load_template(&self, file_name: &str, ttl: Duration) -> Result<Option<String>> {
        let Some(cache) = self.read_json::<TemplateCache>(&self.template_path(file_name))? else {
            return Ok(None);
        };

        Ok(is_fresh(cache.fetched_at, ttl).then_some(cache.content))
    }

    pub(crate) fn save_template(&self, file_name: &str, content: &str) -> Result<()> {
        let cache = TemplateCache {
            fetched_at: now_unix(),
            content: content.to_string(),
        };
        self.write_json(&self.template_path(file_name), &cache)
    }

    fn manifest_path(&self) -> PathBuf {
        self.cache_dir.join("manifest.json")
    }

    fn template_path(&self, file_name: &str) -> PathBuf {
        self.cache_dir.join("templates").join(file_name)
    }

    fn read_json<T>(&self, path: &PathBuf) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        if !path.exists() {
            return Ok(None);
        }

        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read cache file {}", path.display()))?;
        let value = serde_json::from_str(&source)
            .with_context(|| format!("failed to decode cache file {}", path.display()))?;
        Ok(Some(value))
    }

    fn write_json<T>(&self, path: &PathBuf, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create cache directory {}", parent.display())
            })?;
        }

        let source = serde_json::to_string_pretty(value).context("failed to encode cache file")?;
        fs::write(path, source)
            .with_context(|| format!("failed to write cache file {}", path.display()))
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn is_fresh(fetched_at: u64, ttl: Duration) -> bool {
    now_unix().saturating_sub(fetched_at) <= ttl.as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn returns_fresh_manifest_cache() {
        let temp = TempDir::new().unwrap();
        let paths = AppPaths::new(temp.path().join("cache"), temp.path().join("config.toml"));
        let store = CacheStore::new(&paths);
        let templates = vec![Template {
            name: "Node".to_string(),
            file_name: "Node.gitignore".to_string(),
        }];

        store.save_manifest(&templates).unwrap();

        assert_eq!(
            store.load_manifest(Duration::from_secs(600)).unwrap(),
            Some(templates)
        );
    }

    #[test]
    fn ignores_expired_manifest_cache() {
        let temp = TempDir::new().unwrap();
        let paths = AppPaths::new(temp.path().join("cache"), temp.path().join("config.toml"));
        let store = CacheStore::new(&paths);
        let path = store.manifest_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            serde_json::to_string(&ManifestCache {
                fetched_at: 1,
                templates: vec![Template {
                    name: "Node".to_string(),
                    file_name: "Node.gitignore".to_string(),
                }],
            })
            .unwrap(),
        )
        .unwrap();

        assert_eq!(store.load_manifest(Duration::from_secs(1)).unwrap(), None);
    }
}
