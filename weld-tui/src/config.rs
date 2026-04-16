//! User-facing configuration loaded from a TOML file.
//!
//! Resolution order for the config path:
//! 1. `$XDG_CONFIG_HOME/weld/config.toml` if the variable is set and non-empty
//! 2. Platform-native config dir (`dirs::config_dir`) joined with `weld/config.toml`
//!
//! A missing file is not an error — defaults are used. A malformed file
//! fails loudly.

use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Top-level config. New fields should add `#[serde(default = "...")]` with a
/// named default fn so individual defaults stay colocated with the field.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub show_minimap: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { show_minimap: true }
    }
}

impl Config {
    /// Load config from the default path. Missing file → defaults.
    pub fn load() -> Result<Self, ConfigError> {
        match default_path() {
            Some(path) => Self::load_from(&path),
            None => Ok(Self::default()),
        }
    }

    /// Load config from an explicit path. Missing file → defaults.
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).map_err(|source| ConfigError::Parse {
                path: path.to_path_buf(),
                source,
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            }),
        }
    }
}

/// Resolve the default config path, honoring `XDG_CONFIG_HOME` first.
fn default_path() -> Option<PathBuf> {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg).join("weld/config.toml"));
    }
    dirs::config_dir().map(|d| d.join("weld/config.toml"))
}

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Read { path, source } => {
                write!(f, "failed to read config at {}: {source}", path.display())
            }
            ConfigError::Parse { path, source } => {
                write!(f, "failed to parse config at {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::Parse { source, .. } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(contents: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().expect("tempfile");
        f.write_all(contents.as_bytes()).expect("write");
        f
    }

    #[test]
    fn defaults_when_file_missing() {
        let missing = PathBuf::from("/nonexistent/weld-test/config.toml");
        let cfg = Config::load_from(&missing).expect("missing file is ok");
        assert!(cfg.show_minimap);
    }

    #[test]
    fn empty_file_uses_defaults() {
        let f = write_tmp("");
        let cfg = Config::load_from(f.path()).expect("empty is valid toml");
        assert!(cfg.show_minimap);
    }

    #[test]
    fn overrides_show_minimap() {
        let f = write_tmp("show_minimap = false\n");
        let cfg = Config::load_from(f.path()).expect("valid");
        assert!(!cfg.show_minimap);
    }

    #[test]
    fn malformed_toml_fails_loudly() {
        let f = write_tmp("show_minimap = not_a_bool\n");
        let err = Config::load_from(f.path()).expect_err("should fail");
        assert!(matches!(err, ConfigError::Parse { .. }));
    }

    #[test]
    fn unknown_field_fails_loudly() {
        let f = write_tmp("not_a_real_setting = true\n");
        let err = Config::load_from(f.path()).expect_err("should fail on unknown key");
        assert!(matches!(err, ConfigError::Parse { .. }));
    }
}
