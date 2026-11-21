# Dependency list order

When adding, removing, or bumping a dependency, put it in its sorted place.
Do not append at the bottom.

## Cargo.toml

- Every dependency, external or path, is declared once, in the root
  `[workspace.dependencies]`, with its version and default features. A
  crate takes it with `{ workspace = true }`, adding only `features`,
  `optional`, or (edition 2024) `default-features`. No version in a member
  manifest (owner, 21 November 2026). A crate only an example uses gets a
  comment there saying so.
- An unused dependency fails the build: `[workspace.lints.cargo]`
  `unused_dependencies = "deny"` (Cargo 1.100). Remove it, do not allow it.
- `[workspace.members]`: alphabetical by path.
- `[workspace.dependencies]`, `[dependencies]`, `[dev-dependencies]`,
  `[build-dependencies]`, per-target tables: two zones, decided by the
  owner (7 October 2026):
  1. this repository's path crates (`glossary-*`, `protocontract`),
     alphabetical;
  2. third-party crates, alphabetical by package name.

  Optional crates stay in the same sort, not in a block at the end.

```toml
# BAD — protocontract sorted in among the third-party crates, and
# connectrpc dumped at the bottom
buffa = { workspace = true }
protocontract = { workspace = true }
tokio = { workspace = true, optional = true }
connectrpc = { workspace = true, optional = true }

# GOOD
protocontract = { workspace = true }
buffa = { workspace = true }
connectrpc = { workspace = true, optional = true }
tokio = { workspace = true, optional = true }
```

## mise.toml

`[tools]` and `[tool_alias]`: grouped by purpose under a comment (Protocol
Buffers, Rust, Markdown), the entries of a group alphabetical.

Does not replace `dependency-quarantine.md`, which applies to every tool
`mise.toml` pins.
