//! Config schema, XDG paths, Excalith JSON importer.
//!
//! Filesystem and XDG resolution live here rather than in `sp-core`, which must stay
//! wasm32-clean (no `std::fs`, no host directory lookups).

use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use sp_core::{SearchEngine, Section, Shortcut, Theme};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptConfig {
    pub placeholder: String,
    #[serde(alias = "promptSymbol")]
    pub symbol: String,
    #[serde(alias = "userColor")]
    pub user_color: String,
    #[serde(alias = "hostColor")]
    pub host_color: String,
    #[serde(alias = "promptColor")]
    pub prompt_color: String,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            placeholder: "command...".to_string(),
            symbol: "❯".to_string(),
            user_color: "green".to_string(),
            host_color: "magenta".to_string(),
            prompt_color: "magenta".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub username: String,
    pub title: String,
    pub theme: Theme,
    pub prompt: PromptConfig,
    pub sections: Vec<Section>,
    /// Default search engine for bare text with no filter match.
    pub default_engine: SearchEngine,
    /// Configured shortcut prefixes, e.g. `s some bug` -> StackOverflow search.
    pub shortcuts: Vec<Shortcut>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            username: "excalith".to_string(),
            title: "Excalith Start Page".to_string(),
            theme: Theme::default(),
            prompt: PromptConfig::default(),
            sections: sp_core::default_sections(),
            default_engine: SearchEngine::default(),
            shortcuts: sp_core::default_shortcuts(),
        }
    }
}

impl From<Config> for sp_core::AppState {
    fn from(c: Config) -> Self {
        sp_core::AppState::new(
            c.title,
            c.username,
            c.prompt.symbol,
            c.theme,
            c.sections,
            c.default_engine,
            c.shortcuts,
        )
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
    #[error("failed to parse YAML config file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("failed to serialize YAML config: {source}")]
    Serialize {
        #[source]
        source: serde_yaml::Error,
    },
    #[error("failed to parse Excalith JSON: {source}")]
    ParseJson {
        #[source]
        source: serde_json::Error,
    },
}

/// Resolved path to `config.yaml`, honouring `XDG_CONFIG_HOME` on Linux and the platform
/// equivalent elsewhere (`directories::ProjectDirs`).
pub fn config_path() -> Result<PathBuf, ConfigError> {
    let dirs = ProjectDirs::from("", "", "start-page").ok_or(ConfigError::NoConfigDir)?;
    Ok(dirs.config_dir().join("config.yaml"))
}

/// Load the config from the resolved XDG path. A missing file yields `Config::default()`,
/// not an error.
pub fn load() -> Result<Config, ConfigError> {
    load_from(&config_path()?)
}

pub fn load_from(path: &std::path::Path) -> Result<Config, ConfigError> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_yaml::from_str(&contents).map_err(|source| ConfigError::Parse {
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

pub fn save_to(config: &Config, path: &std::path::Path) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let yaml = serde_yaml::to_string(config).map_err(|source| ConfigError::Serialize { source })?;
    fs::write(path, yaml).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

// --- Excalith JSON parsing structures ---

#[derive(Deserialize)]
struct ExcalithRaw {
    username: Option<String>,
    title: Option<String>,
    theme: Option<Theme>,
    prompt: Option<PromptConfig>,
    search: Option<ExcalithSearchRaw>,
    sections: Option<ExcalithSectionsRaw>,
}

#[derive(Deserialize)]
struct ExcalithSearchRaw {
    default: Option<String>,
    #[serde(default)]
    shortcuts: Vec<ExcalithShortcutRaw>,
}

#[derive(Deserialize)]
struct ExcalithShortcutRaw {
    alias: String,
    url: String,
}

#[derive(Deserialize)]
struct ExcalithSectionsRaw {
    #[serde(default)]
    list: Vec<Section>,
}

impl Config {
    /// Parses an Excalith `settings.json` content string into `Config`.
    pub fn from_excalith_json(json_str: &str) -> Result<Self, ConfigError> {
        let raw: ExcalithRaw =
            serde_json::from_str(json_str).map_err(|source| ConfigError::ParseJson { source })?;
        let def = Config::default();

        let default_engine = raw
            .search
            .as_ref()
            .and_then(|s| s.default.as_deref())
            .map(|u| SearchEngine {
                url_template: u.replace("{}", "{query}"),
            })
            .unwrap_or(def.default_engine);

        let shortcuts = raw
            .search
            .map(|s| {
                s.shortcuts
                    .into_iter()
                    .map(|item| Shortcut {
                        prefix: item.alias,
                        engine: SearchEngine {
                            url_template: item.url.replace("{}", "{query}"),
                        },
                    })
                    .collect()
            })
            .unwrap_or(def.shortcuts);

        Ok(Config {
            username: raw.username.unwrap_or(def.username),
            title: raw.title.unwrap_or(def.title),
            theme: raw.theme.unwrap_or(def.theme),
            prompt: raw.prompt.unwrap_or(def.prompt),
            sections: raw.sections.map(|s| s.list).unwrap_or(def.sections),
            default_engine,
            shortcuts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Serializes tests that mutate the process-wide XDG_CONFIG_HOME env var.
    #[cfg(target_os = "linux")]
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn config_round_trips_through_yaml() {
        let config = Config {
            username: "voznik".to_string(),
            title: "start-page".to_string(),
            ..Config::default()
        };
        let yaml = serde_yaml::to_string(&config).unwrap();
        let back: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config, back);
    }

    #[test]
    fn missing_config_file_yields_defaults() {
        let dir = std::env::temp_dir().join("sp-config-test-missing");
        let path = dir.join("does-not-exist.yaml");
        assert_eq!(load_from(&path).unwrap(), Config::default());
    }

    #[test]
    fn parses_excalith_settings_json() {
        let json = include_str!("../../../data/settings.json");
        let config = Config::from_excalith_json(json).expect("should parse settings.json");
        assert_eq!(config.username, "Excalith");
        assert_eq!(config.title, "Excalith Start Page");
        assert_eq!(config.prompt.symbol, "❯");
        assert_eq!(config.sections.len(), 6);
        assert_eq!(config.sections[0].title, "General");
        assert_eq!(config.sections[0].links.len(), 4);
        assert_eq!(config.sections[0].links[0].name, "Portfolio");
        assert_eq!(config.sections[0].links[0].url, "https://cancellek.com");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn config_path_respects_xdg_config_home() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp = std::env::temp_dir().join("sp-config-test-xdg");
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &temp);
        }
        let resolved = config_path().unwrap();
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        assert_eq!(resolved, temp.join("start-page").join("config.yaml"));
    }
}
