use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    #[arg(default_value = default_vault_path())]
    pub vault_path: PathBuf,
    #[arg(short, long, default_value = "3D", ignore_case = true)]
    pub renderer: RendererType,
}

#[derive(clap::ValueEnum, Clone)]
pub enum RendererType {
    #[value(name = "2D", alias = "twod")]
    TwoD,
    #[value(name = "3D", alias = "threed")]
    ThreeD,
}

// Little hack to set a default vault path
// if run with cargo or standalone bin
fn default_vault_path() -> Option<&'static str> {
    std::env::var("CARGO_MANIFEST_DIR")
        .ok()
        .map(|_| "./ExampleVault")
}
