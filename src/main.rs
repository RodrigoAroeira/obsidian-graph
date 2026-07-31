#![allow(dead_code, unused_variables, unused_mut)]
use clap::Parser;

use crate::renderer::Renderer;

mod cli;
mod graph;
mod link;
mod physics;
mod renderer;
mod renderer2d;
mod renderer3d;

fn main() {
    let args = cli::Args::parse();

    if !args.vault_path.exists() {
        panic!("{} doesn't exist", args.vault_path.display())
    }
    let mut g = graph::build_graph(&args.vault_path);

    let mut renderer: Box<dyn Renderer> = match args.renderer {
        cli::RendererType::TwoD => Box::new(renderer2d::Renderer2D::new()) as Box<_>,
        cli::RendererType::ThreeD => Box::new(renderer3d::Renderer3D::new()),
    };
    renderer.run(&mut g);
}
