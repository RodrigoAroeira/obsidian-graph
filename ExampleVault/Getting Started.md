# Getting Started

Building and running the tool is quick:

```bash
cargo run --release             # this vault (ExampleVault) in 2D
cargo run --release -- -r 3d    # same vault, 3D renderer
cargo run --release -- ~/some/vault -r 2d
```

The three ideas that matter:

1. Point it at a vault folder — it defaults to `./ExampleVault`.
2. It scans every `.md` file and every wikilink inside them.
3. Links become the edges of the graph.

In 2D: scroll to zoom, drag a node, drag empty space to pan.
In 3D: scroll to zoom, right-drag to orbit, left-drag to pan or drag a node.

For the link details, see [[Link notes#Wikilinks]].

Here is the same welcome text embedded:

![[What is Obsidian]]

Ready to add something? Go to [[Create a note]].
