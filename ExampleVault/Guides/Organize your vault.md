# Repository layout

The repo itself is organized into small modules:

```
obsidian-graph/
├── Cargo.toml
├── src/
│   ├── main.rs         # entry point
│   ├── cli.rs          # arguments: vault path, -r 2d|3d
│   ├── vault.rs        # scans .md files, resolves note names
│   ├── link.rs         # a parsed wikilink
│   ├── graph.rs        # nodes + edges, builds the graph
│   ├── physics.rs      # force-directed layout
│   ├── renderer.rs     # shared colors/properties + Renderer trait
│   ├── renderer2d.rs   # raylib 2D view
│   └── renderer3d.rs   # raylib 3D view
├── ExampleVault/       # this vault
└── obsidian-fake/      # generated 1100-note stress test
```

The graph only follows links, not folders. See [[Graph view]] and [[Advanced/Plugins]].
