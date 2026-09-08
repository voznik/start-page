//! Config schema, XDG paths, Excalith JSON importer.
//!
//! Filesystem and XDG resolution live here rather than in `sp-core`, which must stay
//! wasm32-clean (no `std::fs`, no host directory lookups).

use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use sp_core::{SearchEngine, Shortcut, Theme};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub username: String,
    pub title: String,
    pub theme: Theme,
    /// Default search engine for bare text with no filter match (T1.5).
    pub default_engine: SearchEngine,
    /// Configured shortcut prefixes, e.g. `s some bug` -> StackOverflow search (T1.5).
    pub shortcuts: Vec<Shortcut>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            username: String::new(),
            title: String::new(),
            theme: Theme::default(),
            default_engine: SearchEngine::default(),
            shortcuts: sp_core::default_shortcuts(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not determine a config directory for this platform")]
    NoConfigDir,
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

/// Resolved path to `config.toml`, honouring `XDG_CONFIG_HOME` on Linux and the platform
/// equivalent elsewhere (`directories::ProjectDirs`).
pub fn config_path() -> Result<PathBuf, ConfigError> {
    let dirs = ProjectDirs::from("", "", "start-page").ok_or(ConfigError::NoConfigDir)?;
    Ok(dirs.config_dir().join("config.toml"))
}

/// Load the config from the resolved XDG path. A missing file yields `Config::default()`,
/// not an error.
pub fn load() -> Result<Config, ConfigError> {
    load_from(&config_path()?)
}

fn load_from(path: &std::path::Path) -> Result<Config, ConfigError> {
    match fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        }),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(source) => Err(ConfigError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serializes tests that mutate the process-wide XDG_CONFIG_HOME env var.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn config_round_trips_through_toml() {
        let config = Config {
            username: "voznik".to_string(),
            title: "start-page".to_string(),
            ..Config::default()
        };
        let toml = toml::to_string(&config).unwrap();
        let back: Config = toml::from_str(&toml).unwrap();
        assert_eq!(config, back);
    }

    #[test]
    fn missing_config_file_yields_defaults() {
        let dir = std::env::temp_dir().join("sp-config-test-missing");
        let path = dir.join("does-not-exist.toml");
        assert_eq!(load_from(&path).unwrap(), Config::default());
    }

    // XDG_CONFIG_HOME is a Linux/XDG-spec convention; `directories` ignores it on macOS and
    // Windows in favour of the platform-native directories.
    #[cfg(target_os = "linux")]
    #[test]
    fn config_path_respects_xdg_config_home() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp = std::env::temp_dir().join("sp-config-test-xdg");
        // Safety: guarded by ENV_LOCK, no other test in this crate reads/writes this var
        // concurrently.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &temp);
        }
        let resolved = config_path().unwrap();
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        assert_eq!(resolved, temp.join("start-page").join("config.toml"));
    }
}
