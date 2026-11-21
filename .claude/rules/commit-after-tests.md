# Test before commit

Never create a git commit until `mise run gate` has exited 0 on **this
working tree** in this session. `cargo test` alone is not enough: CI runs
the gate (the generated-file check, clippy in each feature set, rustfmt,
cargo-deny, buf lint and format, markdownlint, the tests).

Follow `run-commands.md`: repo root (`mise.toml`), unsandboxed. After a
proto edit, `mise run generate` first.

```bash
mise run gate
```

If it fails, fix and re-run. Do not commit a red tree.

```text
# BAD — commit after a local cargo test, skip the gate
# (markdownlint, the generated-file check, and buf format only run there)

# GOOD — gate green, then git add / commit / push
```

Do not skip for "format-only", "comments", `mise.toml`, workflow files, or
Markdown: markdownlint is part of the gate. The owner must explicitly
waive this rule to commit without that test.
