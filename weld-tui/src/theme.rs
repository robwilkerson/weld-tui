use ratatui::style::{Color, Style};

/// All colors and styles used by the TUI, in one place.
/// Add new themes by creating additional constructors (e.g., `Theme::latte()`).
#[allow(dead_code)]
pub struct Theme {
    /// Background for the entire app
    pub bg: Color,
    /// Default foreground text
    pub fg: Color,
    /// Header bar background
    pub header_bg: Color,
    /// Header file path text
    pub header_fg: Color,
    /// Dirty indicator dot in header
    pub dirty_indicator: Color,
    /// Status bar background
    pub status_bar_bg: Color,
    /// Status bar text
    pub status_bar_fg: Color,
    /// Line number foreground
    pub line_number_fg: Color,
    /// Gutter background
    pub gutter_bg: Color,
    /// Active diff dot in gutter
    pub gutter_dot: Color,
    /// Scrollbar track
    pub scrollbar_track: Color,
    /// Scrollbar thumb
    pub scrollbar_thumb: Color,
    /// Background for deleted lines
    pub diff_delete_bg: Color,
    /// Emphasis for changed characters within deleted lines
    pub diff_delete_emphasis_bg: Color,
    /// Background for inserted lines
    pub diff_insert_bg: Color,
    /// Emphasis for changed characters within inserted lines
    pub diff_insert_emphasis_bg: Color,
    /// Style for the currently active diff block indicator
    pub active_block_style: Style,
    /// Overlay background (help, prompts)
    pub overlay_bg: Color,
    /// Overlay text
    pub overlay_fg: Color,
}

/// Catppuccin Macchiato palette.
impl Default for Theme {
    fn default() -> Self {
        Theme {
            bg: Color::Rgb(36, 39, 58),                                         // Base
            fg: Color::Rgb(202, 211, 245),                                      // Text
            header_bg: Color::Rgb(54, 58, 79),                                  // Surface0
            header_fg: Color::Rgb(138, 173, 244),                               // Blue
            dirty_indicator: Color::Rgb(245, 169, 127),                         // Peach
            status_bar_bg: Color::Rgb(54, 58, 79),                              // Surface0
            status_bar_fg: Color::Rgb(165, 173, 203),                           // Subtext0
            line_number_fg: Color::Rgb(165, 173, 203),                          // Subtext0
            gutter_bg: Color::Rgb(73, 77, 100),                                 // Surface1
            gutter_dot: Color::Rgb(245, 169, 127),                              // Peach
            scrollbar_track: Color::Rgb(110, 115, 141),                         // Overlay0
            scrollbar_thumb: Color::Rgb(147, 154, 183),                         // Overlay2
            diff_delete_bg: Color::Rgb(76, 50, 62),                             // Red blended into Base
            diff_delete_emphasis_bg: Color::Rgb(120, 60, 75),                   // Stronger red for char-level
            diff_insert_bg: Color::Rgb(48, 64, 58),                             // Green blended into Base
            diff_insert_emphasis_bg: Color::Rgb(55, 90, 75),                    // Stronger green for char-level
            active_block_style: Style::default().fg(Color::Rgb(245, 169, 127)), // Peach
            overlay_bg: Color::Rgb(30, 32, 48),                                 // Mantle
            overlay_fg: Color::Rgb(202, 211, 245),                              // Text
        }
    }
}
