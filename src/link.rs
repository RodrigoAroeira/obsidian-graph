use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Link {
    pub page: String,
    pub src: PathBuf,
    pub alias: Option<String>,
    pub heading: Option<String>,
    pub embed: bool,
}
