//! User-facing configuration loaded from a TOML file.
//!
//! Resolution order for the config path:
//! 1. `$XDG_CONFIG_HOME/weld/config.toml` if the variable is set and non-empty
//! 2. Platform-native config dir (`dirs::config_dir`) joined with `weld/config.toml`
//!
//! `XDG_CONFIG_HOME` is honored on **all** platforms (macOS/Windows included),
//! not just Linux. This is deliberate: users who prefer dotfile-style config
//! layouts can set it once and get consistent behavior everywhere.
//!
//! On first launch, if no file exists at the resolved path, we drop a
//! commented template with all defaults so users have something to edit.
//! This happens only when the file is absent; existing files are never
//! touched. New settings added in later releases will *not* appear in
//! existing users' files — release notes should itemize new keys.
//!
//! A missing file is not an error — defaults are used. A malformed file, an
//! unresolvable config directory, or any non-`NotFound` IO error fails loudly.

use std::env;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Commented TOML template written to disk on first launch. Keep in sync with
/// `Config::default()` — the test `default_template_matches_default_config`
/// asserts they agree.
const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("default_config.toml");

/// Top-level config.
///
/// Uses container-level `#[serde(default)]`: the whole struct is built from
/// `Config::default()` first, then fields present in the TOML overwrite.
/// When adding a new field, update `Default` to carry its default value
/// *and* add a corresponding line to `default_config.toml`.
///
/// `deny_unknown_fields` catches typos and stale keys after refactors.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub show_minimap: bool,
    /// Maximum number of undoable operations retained in the undo stack.
    pub undo_capacity: usize,
    /// Columns per tab stop when expanding `\t` for display.
    pub tab_width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            show_minimap: true,
            undo_capacity: 100,
            tab_width: 4,
        }
    }
}

impl Config {
    /// Load config from the default path.
    ///
    /// On first launch (no file present) this writes a commented template at
    /// the resolved path, then loads it. Fails loudly if the config directory
    /// cannot be resolved. Failure to write the template is logged to stderr
    /// but not fatal — defaults still load.
    pub fn load() -> Result<Self, ConfigError> {
        let path = default_path().ok_or(ConfigError::NoConfigDir)?;

        // Always try to create — `write_default_template` is atomic and a
        // no-op if the file already exists, so no check-then-write race.
        if let Err(e) = write_default_template(&path) {
            eprintln!(
                "weld: could not create default config at {}: {e}",
                path.display(),
            );
        }

        Self::load_from(&path)
    }

    /// Load config from an explicit path. Missing file → defaults.
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(contents) => {
                let cfg: Config =
                    toml::from_str(&contents).map_err(|source| ConfigError::Parse {
                        path: path.to_path_buf(),
                        source,
                    })?;
                cfg.validate(path)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Reject semantically-invalid combinations that serde cannot catch
    /// (e.g., `tab_width = 0` would panic inside `expand_tabs`).
    fn validate(self, path: &Path) -> Result<Self, ConfigError> {
        if self.tab_width == 0 {
            return Err(ConfigError::Invalid {
                path: path.to_path_buf(),
                message: "tab_width must be greater than 0".to_string(),
            });
        }
        Ok(self)
    }
}

/// Resolve the default config path, honoring `XDG_CONFIG_HOME` first.
///
/// Returns `None` only if `XDG_CONFIG_HOME` is unset/empty **and**
/// `dirs::config_dir()` cannot resolve a path (no `$HOME`, stripped-down
/// container, etc.). Callers should treat `None` as an error, not as a cue
/// to silently use defaults.
fn default_path() -> Option<PathBuf> {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg).join("weld/config.toml"));
    }
    dirs::config_dir().map(|d| d.join("weld/config.toml"))
}

/// Atomically write the commented default template at `path`, creating parent
/// dirs as needed. A no-op if the file already exists — safe to call on every
/// launch without a pre-check.
fn write_default_template(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => file.write_all(DEFAULT_CONFIG_TEMPLATE.as_bytes()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
}

#[derive(Debug)]
pub enum ConfigError {
    /// Neither `XDG_CONFIG_HOME` nor the platform-native config dir could be
    /// resolved — we have nowhere to look for a config file.
    NoConfigDir,
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    /// A semantically-invalid value survived deserialization — e.g., a type
    /// that serde accepts but that we refuse to run with.
    Invalid { path: PathBuf, message: String },
}

impl fmt::Display for ConfigError {
    // Top-level message is context only; the underlying cause is exposed via
    // `source()` and rendered by the caller's error reporter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NoConfigDir => {
                f.write_str("could not resolve config directory: set $XDG_CONFIG_HOME or $HOME")
            }
            ConfigError::Read { path, .. } => {
                write!(f, "failed to read config at {}", path.display())
            }
            ConfigError::Parse { path, .. } => {
                write!(f, "failed to parse config at {}", path.display())
            }
            ConfigError::Invalid { path, message } => {
                write!(f, "invalid config at {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::NoConfigDir | ConfigError::Invalid { .. } => None,
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
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("does-not-exist.toml");
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

    #[test]
    fn default_template_matches_default_config() {
        let from_template: Config =
            toml::from_str(DEFAULT_CONFIG_TEMPLATE).expect("template must parse");
        assert_eq!(from_template, Config::default());
    }

    #[test]
    fn write_default_template_creates_parent_dirs_and_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested/weld/config.toml");
        assert!(!path.exists());

        write_default_template(&path).expect("write");

        let written = fs::read_to_string(&path).expect("read");
        assert_eq!(written, DEFAULT_CONFIG_TEMPLATE);

        // The written file must parse back into the default Config.
        let cfg = Config::load_from(&path).expect("load");
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn write_default_template_preserves_existing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(&path, "show_minimap = false\n").expect("seed existing file");

        // Second call must be a no-op — the user's edits survive.
        write_default_template(&path).expect("no-op on existing");

        let contents = fs::read_to_string(&path).expect("read");
        assert_eq!(contents, "show_minimap = false\n");
    }

    #[test]
    fn rejects_tab_width_zero() {
        let f = write_tmp("tab_width = 0\n");
        let err = Config::load_from(f.path()).expect_err("zero tab_width must fail");
        assert!(matches!(err, ConfigError::Invalid { .. }));
    }
}
