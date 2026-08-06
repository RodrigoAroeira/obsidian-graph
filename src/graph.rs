use std::cell::RefCell;
use std::rc::Rc;
use std::time::SystemTime;

use raylib::prelude::Vector3;

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
    pub date_created: Option<SystemTime>,
    pub appeared: bool,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub nodes: Vec<Rc<RefCell<Node>>>,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::vault::Vault;

    #[test]
    fn subdir_links_resolve() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let root = dir.path();
        fs::create_dir(root.join("sub"))?;
        fs::write(root.join("index.md"), "[[Note]]\n[[Missing]]")?;
        fs::write(root.join("sub/Note.md"), "")?;

        let vault = Vault::scan(root)?;
        let graph = vault.build_graph()?;

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

    #[test]
    fn rebuild_reflects_file_changes() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let root = dir.path();
        fs::write(root.join("a.md"), "[[b]]")?;
        fs::write(root.join("b.md"), "")?;
        let vault = Vault::scan(root)?;

        fs::write(root.join("a.md"), "[[c]]")?;
        fs::write(root.join("c.md"), "")?;
        fs::remove_file(root.join("b.md"))?;

        let graph = vault.rebuild_graph()?;

        assert!(graph.nodes.iter().any(|n| n.borrow().name == "c"));
        assert!(!graph.nodes.iter().any(|n| n.borrow().name == "b"));

        let a = graph
            .nodes
            .iter()
            .find(|n| n.borrow().name == "a")
            .unwrap();
        let a_links: Vec<String> = a
            .borrow()
            .edges
            .iter()
            .map(|e| e.target.borrow().name.clone())
            .collect();
        assert!(a_links.contains(&"c".to_string()));
        Ok(())
    }
}
