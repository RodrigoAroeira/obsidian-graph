use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use regex_lite::Regex;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Link {
    pub page: String,
    pub src: PathBuf,
    pub alias: Option<String>,
    pub heading: Option<String>,
    pub embed: bool,
}

pub fn parse_links<P: AsRef<Path>>(content: &str, src: P) -> Vec<Link> {
    let re = Regex::new(
        r"(?P<embed>!)?\[\[(?P<page>[^#\]|]+)(?:#(?P<heading>[^\]|]+))?(?:\|(?P<alias>[^\]]+))?\]\]",
    )
    .unwrap();

    let src = src
        .as_ref()
        .file_name()
        .and_then(|n| n.to_str())
        .map_or_else(PathBuf::new, PathBuf::from);

    let parser = Parser::new(content);
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
            src: src.clone(),
            embed: m.name("embed").is_some(),
            alias: m.name("alias").map(|m| m.as_str().to_string()),
            heading: m.name("heading").map(|m| m.as_str().to_string()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::parse_links;

    fn pages(content: &str) -> Vec<String> {
        parse_links(content, Path::new("note.md"))
            .into_iter()
            .map(|l| l.page)
            .collect()
    }

    #[test]
    fn plain_link() {
        assert_eq!(pages("see [[Target]] here"), vec!["Target"]);
    }

    #[test]
    fn link_with_alias() {
        let links = parse_links("[[Target|my alias]]", Path::new("note.md"));
        assert_eq!(links[0].page, "Target");
        assert_eq!(links[0].alias.as_deref(), Some("my alias"));
    }

    #[test]
    fn link_with_heading() {
        let links = parse_links("[[Target#Section]]", Path::new("note.md"));
        assert_eq!(links[0].page, "Target");
        assert_eq!(links[0].heading.as_deref(), Some("Section"));
    }

    #[test]
    fn embed_flag() {
        let links = parse_links("[[Plain]] ![[Embed]]", Path::new("note.md"));
        assert!(!links[0].embed);
        assert!(links[1].embed);
    }

    #[test]
    fn code_blocks_are_ignored() {
        assert_eq!(
            pages("```rust\n[[NotALink]]\n```\nreal: [[RealLink]]"),
            vec!["RealLink"]
        );
    }
}
