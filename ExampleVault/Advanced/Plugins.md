# How it is built

`obsidian-graph` is a pipeline of small, independent pieces:

- **Scan** — `Vault::scan` walks the folder, skipping hidden entries and non-Markdown files (`src/vault.rs`).
- **Parse** — each note is read and every wikilink is extracted from the rendered text, ignoring fenced code blocks (`src/graph.rs`).
- **Build** — every note and every linked name becomes a `Node`; each link becomes an `Edge` (`src/graph.rs`).
- **Simulate** — a force-directed layout separates nodes and pulls linked ones together (`src/physics.rs`).
- **Render** — the `Renderer` trait lets a 2D or 3D raylib view draw the same graph (`src/renderer.rs`).

Existing notes render blue; links that lead nowhere render as missing reddish nodes.

Start again from [[Start here]].
