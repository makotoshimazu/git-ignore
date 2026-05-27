use std::fs;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::paths::AppPaths;

const DEFAULT_CACHE_ENABLED: bool = true;
const DEFAULT_TTL_SECONDS: u64 = 600;

#[derive(Clone, Debug)]
pub(crate) struct AppConfig {
    pub(crate) cache: CacheConfig,
}

#[derive(Clone, Debug)]
pub(crate) struct CacheConfig {
    pub(crate) enabled: bool,
    pub(crate) ttl: Duration,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    cache: Option<CacheConfigFile>,
}

#[derive(Debug, Deserialize)]
struct CacheConfigFile {
    enabled: Option<bool>,
    ttl_seconds: Option<u64>,
}

impl AppConfig {
    pub(crate) fn load(paths: &AppPaths) -> Result<Self> {
        if !paths.config_file.exists() {
            return Ok(Self::default());
        }

        let source = fs::read_to_string(&paths.config_file)
            .with_context(|| format!("failed to read {}", paths.config_file.display()))?;
        let parsed: ConfigFile = toml::from_str(&source)
            .with_context(|| format!("failed to parse {}", paths.config_file.display()))?;
        Ok(Self::from_config_file(parsed))
    }

    fn from_config_file(file: ConfigFile) -> Self {
        let cache = file.cache.unwrap_or(CacheConfigFile {
            enabled: None,
            ttl_seconds: None,
        });

        Self {
            cache: CacheConfig {
                enabled: cache.enabled.unwrap_or(DEFAULT_CACHE_ENABLED),
                ttl: Duration::from_secs(cache.ttl_seconds.unwrap_or(DEFAULT_TTL_SECONDS)),
            },
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            cache: CacheConfig {
                enabled: DEFAULT_CACHE_ENABLED,
                ttl: Duration::from_secs(DEFAULT_TTL_SECONDS),
            },
        }
    }
}
