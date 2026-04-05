use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Detected line ending style of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

impl LineEnding {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
        }
    }
}

/// A file's content, split into lines with metadata.
#[derive(Debug, Clone)]
pub struct FileContent {
    /// Path used to load this file content.
    pub path: PathBuf,
    /// File content split into lines, without line terminators.
    pub lines: Vec<String>,
    /// Detected line ending style, preserved on save.
    pub line_ending: LineEnding,
}

impl FileContent {
    /// Load a file from disk as UTF-8 text.
    /// Detects line ending style (LF vs CRLF) and normalizes internally.
    /// Fails loudly if the file doesn't exist or isn't readable.
    pub fn load(path: &Path) -> io::Result<Self> {
        let raw = fs::read_to_string(path).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("failed to read {}: {err}", path.display()),
            )
        })?;

        let line_ending = if raw.contains("\r\n") {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        };

        let normalized = raw.replace("\r\n", "\n");
        let lines: Vec<String> = normalized.split('\n').map(String::from).collect();

        // Remove trailing empty string from final newline
        let lines = if lines.last().is_some_and(|l| l.is_empty()) {
            lines[..lines.len() - 1].to_vec()
        } else {
            lines
        };

        Ok(FileContent {
            path: path.to_path_buf(),
            lines,
            line_ending,
        })
    }

    /// Save lines back to disk using the original line ending style.
    pub fn save(&self) -> io::Result<()> {
        let ending = self.line_ending.as_str();
        let content = self.lines.join(ending) + ending;
        fs::write(&self.path, content)
    }

    /// Reconstruct the full text content (LF-normalized) for diffing.
    pub fn text(&self) -> String {
        self.lines.join("\n") + "\n"
    }
}

impl fmt::Display for FileContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_lf_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2\nline3\n").unwrap();

        let content = FileContent::load(&path).unwrap();

        assert_eq!(content.lines, vec!["line1", "line2", "line3"]);
        assert_eq!(content.line_ending, LineEnding::Lf);
    }

    #[test]
    fn load_crlf_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\r\nline2\r\nline3\r\n").unwrap();

        let content = FileContent::load(&path).unwrap();

        assert_eq!(content.lines, vec!["line1", "line2", "line3"]);
        assert_eq!(content.line_ending, LineEnding::CrLf);
    }

    #[test]
    fn save_preserves_lf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2\n").unwrap();

        let content = FileContent::load(&path).unwrap();
        content.save().unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, "line1\nline2\n");
    }

    #[test]
    fn save_preserves_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\r\nline2\r\n").unwrap();

        let content = FileContent::load(&path).unwrap();
        content.save().unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, "line1\r\nline2\r\n");
    }

    #[test]
    fn text_returns_lf_normalized() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "a\r\nb\r\n").unwrap();

        let content = FileContent::load(&path).unwrap();
        assert_eq!(content.text(), "a\nb\n");
    }

    #[test]
    fn load_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        fs::write(&path, "").unwrap();

        let content = FileContent::load(&path).unwrap();
        assert!(content.lines.is_empty());
    }

    #[test]
    fn load_missing_file_fails() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.txt");
        let result = FileContent::load(&missing);
        assert!(result.is_err());
    }

    #[test]
    fn load_error_includes_path() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.txt");
        let err = FileContent::load(&missing).unwrap_err();
        assert!(
            err.to_string().contains("nope.txt"),
            "error should include filename: {err}"
        );
    }

    #[test]
    fn round_trip_identical_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("round.txt");
        let original = "func main() {\n\tfmt.Println(\"hello\")\n}\n";
        fs::write(&path, original).unwrap();

        let content = FileContent::load(&path).unwrap();
        content.save().unwrap();

        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(original, after);
    }
}
