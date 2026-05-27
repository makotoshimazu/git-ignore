use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

pub(crate) fn append_to_gitignore(cwd: &Path, content: &str) -> Result<()> {
    let path = cwd.join(".gitignore");
    let needs_leading_newline = match fs::read(&path) {
        Ok(existing) => !existing.is_empty() && !existing.ends_with(b"\n"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;

    if needs_leading_newline {
        file.write_all(b"\n")
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    file.write_all(content.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    if !content.ends_with('\n') {
        file.write_all(b"\n")
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn creates_gitignore_when_missing() {
        let temp = TempDir::new().unwrap();

        append_to_gitignore(temp.path(), "node_modules/\n").unwrap();

        assert_eq!(
            fs::read_to_string(temp.path().join(".gitignore")).unwrap(),
            "node_modules/\n"
        );
    }

    #[test]
    fn appends_without_deduplicating_existing_entries() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join(".gitignore"), "node_modules/\n").unwrap();

        append_to_gitignore(temp.path(), "node_modules/\n").unwrap();

        assert_eq!(
            fs::read_to_string(temp.path().join(".gitignore")).unwrap(),
            "node_modules/\nnode_modules/\n"
        );
    }

    #[test]
    fn inserts_newline_before_append_when_file_has_no_trailing_newline() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join(".gitignore"), "target").unwrap();

        append_to_gitignore(temp.path(), "node_modules/").unwrap();

        assert_eq!(
            fs::read_to_string(temp.path().join(".gitignore")).unwrap(),
            "target\nnode_modules/\n"
        );
    }
}
