use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
pub struct Vault {
    files: Vec<PathBuf>,
    stems: HashSet<String>,
}

fn get_files<P: AsRef<Path>>(vault_path: P) -> Result<Vec<PathBuf>> {
    let vault_path = vault_path.as_ref();
    let mut files = Vec::new();
    let entries = std::fs::read_dir(vault_path)
        .with_context(|| format!("Read {} directory", vault_path.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path
            .file_name()
            .is_some_and(|s| s.to_string_lossy().starts_with('.'))
        {
            continue;
        }
        if path.is_dir() {
            let extend = get_files(&path)?;
            files.extend(extend);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
    Ok(files)
}

impl Vault {
    pub fn scan<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut files = get_files(&root).context("Get root files")?;
        files.sort();
        let stems: HashSet<String> = files
            .iter()
            .map(|p| -> Result<String> {
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
                    .context("Get file stem")
            })
            .collect::<Result<_>>()?;
        Ok(Self { files, stems })
    }

    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub fn exists(&self, page: &str) -> bool {
        self.stems.contains(page.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn scan_with(root: &Path, files: &[&str]) -> Vault {
        for rel in files {
            let path = root.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, "").unwrap();
        }
        Vault::scan(root).unwrap()
    }

    fn relative_files(vault: &Vault, root: &Path) -> Vec<String> {
        vault
            .files()
            .iter()
            .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn exists_bare_name_in_subdir() {
        let dir = tempdir().unwrap();
        let vault = scan_with(dir.path(), &["sub/Note.md"]);
        assert!(vault.exists("Note"));
    }

    #[test]
    fn exists_root_level() {
        let dir = tempdir().unwrap();
        let vault = scan_with(dir.path(), &["Note.md"]);
        assert!(vault.exists("Note"));
    }

    #[test]
    fn exists_dangling_link() {
        let dir = tempdir().unwrap();
        let vault = scan_with(dir.path(), &["Note.md"]);
        assert!(!vault.exists("Missing"));
    }

    #[test]
    fn exists_trims_whitespace() {
        let dir = tempdir().unwrap();
        let vault = scan_with(dir.path(), &["Note.md"]);
        assert!(vault.exists("  Note  "));
    }

    #[test]
    fn exists_is_case_sensitive() {
        let dir = tempdir().unwrap();
        let vault = scan_with(dir.path(), &["Note.md"]);
        assert!(!vault.exists("note"));
    }

    #[test]
    fn exists_empty_string() {
        let dir = tempdir().unwrap();
        let vault = scan_with(dir.path(), &["Note.md"]);
        assert!(!vault.exists(""));
        assert!(!vault.exists("   "));
    }

    #[test]
    fn scan_skips_hidden_entries() {
        let dir = tempdir().unwrap();
        let vault = scan_with(
            dir.path(),
            &[".obsidian/a.md", ".hidden/b.md", "visible.md", "notes.txt"],
        );
        assert_eq!(relative_files(&vault, dir.path()), ["visible.md"]);
    }

    #[test]
    fn scan_sorts_files() {
        let dir = tempdir().unwrap();
        let vault = scan_with(dir.path(), &["z.md", "a/sub.md", "m.md"]);
        assert_eq!(
            relative_files(&vault, dir.path()),
            ["a/sub.md", "m.md", "z.md"]
        );
    }

    #[test]
    fn scan_missing_root_errors() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert!(Vault::scan(&missing).is_err());
    }
}
