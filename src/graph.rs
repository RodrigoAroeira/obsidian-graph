use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use rand::prelude::*;
use raylib::prelude::Vector3;
use regex_lite::Regex;

use crate::link::Link;

#[derive(Debug, Clone)]
pub struct Edge {
    pub target: Rc<RefCell<Node>>,
    #[allow(dead_code)]
    pub link: Link,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub position: Vector3,
    pub velocity: Vector3,
    pub is_file: bool,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub nodes: Vec<Rc<RefCell<Node>>>,
}

fn get_files(vault_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = match fs::read_dir(vault_path) {
        Ok(e) => e,
        Err(_) => return files,
    };

    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == ".obsidian") {
                continue;
            }
            files.extend(get_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
    files
}

fn extract_relations(md_path: &Path) -> Vec<Link> {
    let content = match fs::read_to_string(md_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let re = Regex::new(
        r"(?P<embed>!)?\[\[(?P<page>[^#\]|]+)(?:#(?P<heading>[^\]|]+))?(?:\|(?P<alias>[^\]]+))?\]\]",
    )
    .unwrap();

    let file_name = md_path
        .file_name()
        .and_then(|n| n.to_str())
        .map_or_else(PathBuf::new, PathBuf::from);

    let parser = Parser::new(&content);
    let mut in_code = false;
    let mut text_buf = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code = true,
            Event::End(TagEnd::CodeBlock) => in_code = false,
            Event::Text(text) if !in_code => text_buf.push_str(&text),
            _ => {}
        }
    }

    re.captures_iter(&text_buf)
        .map(|m| Link {
            page: m.name("page").map_or("", |m| m.as_str()).to_string(),
            src: file_name.clone(),
            embed: m.name("embed").is_some(),
            alias: m.name("alias").map(|m| m.as_str().to_string()),
            heading: m.name("heading").map(|m| m.as_str().to_string()),
        })
        .collect()
}

pub fn build_graph(vault_path: &Path) -> Graph {
    let mut node_map: HashMap<String, bool> = HashMap::new();
    let mut wiki_edges: Vec<(String, Link)> = Vec::new();

    for md_path in get_files(vault_path) {
        let source = md_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        node_map.entry(source.clone()).or_insert(true);

        for link in extract_relations(&md_path) {
            let target = link.page.clone();
            node_map
                .entry(target.clone())
                .or_insert(vault_path.join(format!("{}.md", link.page)).exists());
            wiki_edges.push((source.clone(), link));
        }
    }

    let mut rng = rand::rng();
    let mut by_name: HashMap<String, Rc<RefCell<Node>>> = HashMap::new();

    for (name, is_file) in &node_map {
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
                is_file: *is_file,
                edges: Vec::new(),
            })),
        );
    }

    for (source, link) in wiki_edges {
        let src_rc = by_name.get(&source).unwrap();
        let tgt_rc = by_name.get(&link.page).unwrap();
        src_rc.borrow_mut().edges.push(Edge {
            target: tgt_rc.clone(),
            link,
        });
    }

    Graph {
        nodes: by_name.into_values().collect(),
    }
}
