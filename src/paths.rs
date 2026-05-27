use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::BaseDirs;

#[derive(Clone, Debug)]
pub(crate) struct AppPaths {
    pub(crate) cache_dir: PathBuf,
    pub(crate) config_file: PathBuf,
}

impl AppPaths {
    pub(crate) fn from_env() -> Result<Self> {
        let home = BaseDirs::new()
            .context("could not find the current user's home directory")?
            .home_dir()
            .to_path_buf();

        let cache_base = env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".cache"));
        let config_base = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));

        Ok(Self {
            cache_dir: cache_base.join("git-ignore"),
            config_file: config_base.join("git-ignore").join("config.toml"),
        })
    }

    #[cfg(test)]
    pub(crate) fn new(cache_dir: PathBuf, config_file: PathBuf) -> Self {
        Self {
            cache_dir,
            config_file,
        }
    }
}
