# Run commands from this repository

Every `mise`, `cargo`, and `buf` command for protocontract-buffa **must**
run in this clone's root: the directory with `mise.toml` and the workspace
`Cargo.toml`. The Bash tool's working directory persists between calls and
may have drifted (another repository may be the session's primary
directory), so `cd` to the root by absolute path in the same command when
unsure. Never run them from a sibling project, a scratch or tmp directory,
or a subdirectory.

## Generate

The test protos under `proto/` and the examples' protos are compiled by
the plugin built from this tree into golden files and the examples'
`api/src/generated`. After changing a proto or the plugin's output, run
`mise run generate` before tests, and read the diff: a changed golden file
is a consumer-visible change (`consumers.md`). Never edit a generated file
by hand.

## Fast loop and gate

- `cargo test --workspace` and
  `cargo clippy --workspace --all-targets`: the fast loop.
- `mise run gate`: the gate before a commit (`commit-after-tests.md`).
  It runs `check:generate` (the checked-in generated trees against the
  plugins' output), `lint:proto` (buf lint and buf format), `lint:rust`
  (clippy in each feature set), `fmt:rust` (`cargo fmt --check`),
  `deny:rust` (cargo-deny, `deny.toml`), `lint:markdown` (markdownlint
  over every `*.md`, `.claude/` included), and `test`. `mise tasks`
  lists them; each runs alone as `mise run <task>`. `mise run format`
  fixes Rust and proto formatting. The proto tasks first run `protos`,
  which stages protovalidate's protos under `.cache/proto` (git-ignored);
  run it once before opening the protos in an editor with Buf LSP.

## Tools

`mise.toml` pins every tool outside Cargo; `mise.lock` holds its URL and
sha256 per platform. After changing a version in `mise.toml`, run
`mise lock --platform linux-x64,linux-arm64,macos-x64,macos-arm64,windows-x64,windows-arm64`
so the release workflow on all six platforms finds it, and commit both
files (and `.mise/locks/`, the lock of an npm tool's packages).

## No sandbox

Do not run `mise`, `buf`, or `cargo` inside a command sandbox. mise and
Cargo need the network for tools, crates, and the RustSec database, the
caches live under the home directory, and generate writes into the source
tree. If Bash sandboxing is on, run these with
`dangerouslyDisableSandbox: true`.
