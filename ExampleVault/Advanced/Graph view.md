# Graph view

The graph view turns your vault into a live map.

- Every note is a **node**.
- Every wikilink is an **edge**.
- Notes that exist on disk render normally; links that lead nowhere render as **missing** nodes.

Run `cargo run --release` in the repo root to see this very vault as a graph.

Back in [[Start here]] there is a dangling link to [[Nothing here]] — that is the missing node.
