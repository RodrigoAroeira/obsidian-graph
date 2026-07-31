use anyhow::Result;
use clap::Parser;

use crate::renderer::Renderer;

mod cli;
mod graph;
mod link;
mod physics;
mod renderer;
mod renderer2d;
mod renderer3d;
mod vault;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let vault = vault::Vault::scan(&args.vault_path)?;
    let mut g = graph::build_graph(&vault)?;

    let mut renderer: Box<dyn Renderer> = match args.renderer {
        cli::RendererType::TwoD => Box::new(renderer2d::Renderer2D::new()) as Box<_>,
        cli::RendererType::ThreeD => Box::new(renderer3d::Renderer3D::new()),
    };
    renderer.run(&mut g);
    Ok(())
}
