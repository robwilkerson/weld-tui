use std::fs;
use std::path::{Path, PathBuf};

/// A file's content, split into lines.
#[derive(Debug)]
pub struct FileContent {
    pub path: PathBuf,
    pub lines: Vec<String>,
}

impl FileContent {
    /// Load a file from disk, splitting into lines.
    /// Fails loudly if the file doesn't exist or isn't readable.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let raw = fs::read_to_string(path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!("failed to read {}: {err}", path.display()),
            )
        })?;
        let lines: Vec<String> = raw.lines().map(String::from).collect();
        Ok(FileContent {
            path: path.to_path_buf(),
            lines,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_reads_lines() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "line one").unwrap();
        writeln!(tmp, "line two").unwrap();
        writeln!(tmp, "line three").unwrap();

        let content = FileContent::load(tmp.path()).unwrap();
        assert_eq!(content.lines, vec!["line one", "line two", "line three"]);
    }

    #[test]
    fn load_empty_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let content = FileContent::load(tmp.path()).unwrap();
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
}
