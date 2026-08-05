use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result};
use rand::prelude::*;
use raylib::prelude::Vector3;

use crate::link;
use crate::vault::Vault;

#[derive(Debug, Clone)]
pub struct Edge {
    pub target: Rc<RefCell<Node>>,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub position: Vector3,
    pub velocity: Vector3,
    pub exists: bool,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub nodes: Vec<Rc<RefCell<Node>>>,
}

fn file_stem<P: AsRef<Path>>(path: P) -> String {
    path.as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .map(String::from)
        .unwrap_or_default()
}

pub fn build_graph(vault: &Vault) -> Result<Graph> {
    let mut node_map: HashMap<String, bool> = HashMap::new();
    let mut wiki_edges: Vec<(String, String)> = Vec::new();

    for md_path in vault.files() {
        let source = file_stem(md_path);
        node_map.entry(source.clone()).or_insert(true);

        let content = std::fs::read_to_string(md_path)
            .with_context(|| format!("Read {}", md_path.display()))?;

        for link in link::parse_links(&content, md_path) {
            let target = link.page.clone();
            node_map
                .entry(target.clone())
                .or_insert(vault.exists(&link.page));
            wiki_edges.push((source.clone(), target));
        }
    }

    let mut rng = rand::rng();
    let mut by_name: HashMap<String, Rc<RefCell<Node>>> = HashMap::new();

    for (name, &exists) in &node_map {
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::build_graph;
    use crate::vault::Vault;

    #[test]
    fn subdir_links_resolve() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let root = dir.path();
        fs::create_dir(root.join("sub"))?;
        fs::write(root.join("index.md"), "[[Note]]\n[[Missing]]")?;
        fs::write(root.join("sub/Note.md"), "")?;

        let vault = Vault::scan(root)?;
        let graph = build_graph(&vault)?;

        let note = graph
            .nodes
            .iter()
            .find(|n| n.borrow().name == "Note")
            .unwrap();
        assert!(note.borrow().exists);

        let missing = graph
            .nodes
            .iter()
            .find(|n| n.borrow().name == "Missing")
            .unwrap();
        assert!(!missing.borrow().exists);

        let index = graph
            .nodes
            .iter()
            .find(|n| n.borrow().name == "index")
            .unwrap();
        let targets: Vec<String> = index
            .borrow()
            .edges
            .iter()
            .map(|e| e.target.borrow().name.clone())
            .collect();
        assert!(targets.contains(&"Note".to_string()));
        Ok(())
    }
}
