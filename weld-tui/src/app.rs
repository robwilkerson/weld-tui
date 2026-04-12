use std::path::PathBuf;

use weld_core::file::diff_model::DiffModel;
use weld_core::file::io::{Content, shorten_dir};

use crate::theme::Theme;
use crate::viewport::Viewport;

/// Application mode — determines how input is interpreted.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants used as features land (search, help overlay)
pub enum Mode {
    #[default]
    Normal,
    Command,
    Overlay,
}

/// Tracks multi-key input sequences (e.g., `gg`, future counts/search).
#[derive(Default)]
pub struct InputState {
    /// Whether the previous keypress was `g` (waiting for `gg`).
    pub pending_g: bool,
}

/// Top-level application state.
pub struct App {
    pub model: DiffModel,
    pub theme: Theme,
    pub running: bool,
    #[allow(dead_code)] // Used as features land (search, help overlay)
    pub mode: Mode,
    pub left_dir: String,
    pub left_filename: String,
    pub right_dir: String,
    pub right_filename: String,
    pub needs_initial_scroll: bool,
    pub viewport: Viewport,
    pub input: InputState,
    pub minimap_width: u16,
}

impl App {
    pub fn new(left: PathBuf, right: PathBuf) -> Result<Self, std::io::Error> {
        let left_content = Content::load(&left)?;
        let right_content = Content::load(&right)?;

        let left_abs = left.canonicalize().unwrap_or(left);
        let right_abs = right.canonicalize().unwrap_or(right);

        let mut app = Self::from_contents(left_content, right_content);
        app.left_dir = shorten_dir(
            &left_abs
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        );
        app.left_filename = left_abs
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        app.right_dir = shorten_dir(
            &right_abs
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        );
        app.right_filename = right_abs
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        app.needs_initial_scroll = true;

        Ok(app)
    }

    /// Construct an App from pre-loaded file contents (no filesystem access).
    pub fn from_contents(left_content: Content, right_content: Content) -> Self {
        App {
            model: DiffModel::new(left_content, right_content),
            theme: Theme::default(),
            running: true,
            mode: Mode::default(),
            left_dir: String::new(),
            left_filename: String::new(),
            right_dir: String::new(),
            right_filename: String::new(),
            needs_initial_scroll: false,
            viewport: Viewport::default(),
            input: InputState::default(),
            minimap_width: 1,
        }
    }
}
