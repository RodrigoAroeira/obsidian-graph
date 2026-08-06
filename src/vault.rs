use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use anyhow::{Context, Result};
use rand::prelude::*;
use raylib::prelude::Vector3;

use crate::graph::{Edge, Graph, Node};
use crate::link;

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

fn file_stem<P: AsRef<Path>>(path: P) -> String {
    path.as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .map(String::from)
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct Vault {
    root: PathBuf,
    files: Vec<PathBuf>,
    stems: HashSet<String>,
    created: HashMap<PathBuf, Option<SystemTime>>,
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
        let mut created = HashMap::new();
        for path in &files {
            let meta =
                std::fs::metadata(path).with_context(|| format!("Metadata {}", path.display()))?;
            created.insert(
                path.clone(),
                meta.created().or_else(|_| meta.modified()).ok(),
            );
        }
        Ok(Self {
            root,
            files,
            stems,
            created,
        })
    }

    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub fn exists(&self, page: &str) -> bool {
        self.stems.contains(page.trim())
    }

    pub fn date_created(&self, path: &Path) -> Option<SystemTime> {
        self.created.get(path).copied().flatten()
    }

    pub fn build_graph(&self) -> Result<Graph> {
        let mut node_map: HashMap<String, bool> = HashMap::new();
        let mut wiki_edges: Vec<(String, String)> = Vec::new();

        for md_path in self.files() {
            let source = file_stem(md_path);
            node_map.entry(source.clone()).or_insert(true);

            let content = std::fs::read_to_string(md_path)
                .with_context(|| format!("Read {}", md_path.display()))?;

            for link in link::parse_links(&content, md_path) {
                let target = link.page.clone();
                node_map
                    .entry(target.clone())
                    .or_insert(self.exists(&link.page));
                wiki_edges.push((source.clone(), target));
            }
        }

        let mut by_stem: HashMap<String, &PathBuf> = HashMap::new();
        for path in self.files() {
            by_stem.entry(file_stem(path)).or_insert(path);
        }

        let mut rng = rand::rng();
        let mut by_name: HashMap<String, Rc<RefCell<Node>>> = HashMap::new();

        for (name, &exists) in &node_map {
            let date_created = by_stem.get(name).and_then(|p| self.date_created(p));
            by_name.insert(
                name.clone(),
                Rc::new(RefCell::new(Node {
                    name: name.clone(),
                    position: Vector3::new(
                        rng.random_range(-500.0..500.0),
                        rng.random_range(-500.0..500.0),
                        rng.random_range(-500.0..500.0),
                    ),
                    velocity: Vector3::new(0.0, 0.0, 0.0),
                    exists,
                    edges: Vec::new(),
                    date_created,
                    appeared: false,
                })),
            );
        }

        for (source, target) in wiki_edges {
            let src_rc = by_name.get(&source).context("target node missing")?;
            let tgt_rc = by_name.get(&target).context("target node missing")?;
            src_rc.borrow_mut().edges.push(Edge {
                target: tgt_rc.clone(),
            });
        }

        Ok(Graph {
            nodes: by_name.into_values().collect(),
        })
    }

    pub fn rebuild_graph(&self) -> Result<Graph> {
        Self::scan(&self.root)?.build_graph()
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

    #[test]
    fn date_created_populated_for_existing_file() {
        let dir = tempdir().unwrap();
        let vault = scan_with(dir.path(), &["Note.md"]);
        assert!(vault.date_created(&dir.path().join("Note.md")).is_some());
    }

    #[test]
    fn date_created_none_for_missing_file() {
        let dir = tempdir().unwrap();
        let vault = scan_with(dir.path(), &["Note.md"]);
        assert!(vault.date_created(&dir.path().join("Missing.md")).is_none());
    }
}
