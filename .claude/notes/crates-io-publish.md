# Publishing to crates.io

## State

- Published on 16 October 2026 by the owner, by hand with a crates.io
  token (`cargo publish` from the tag `v0.3.1`):
  [`protocontract`](https://crates.io/crates/protocontract) 0.3.1 and
  [`protoc-gen-protocontract-buffa`](https://crates.io/crates/protoc-gen-protocontract-buffa)
  0.3.1, owned by the owner's GitHub account. docs.rs built
  `protocontract`.
- What the publish needed is in the tree since 0.3.1: caret ranges in
  `[workspace.dependencies]` (the quarantine lives in `Cargo.lock`,
  `.claude/rules/dependency-quarantine.md`); keywords, categories,
  `LICENSE`, and `exclude = ["BUILD.bazel"]` in both packages; docs.rs
  with all features; a README with absolute links and the crates.io
  dependency; `.bazelignore` for `target/package`.
- 0.4.0 was published on 18 October 2026 from the tag `v0.4.0`, by Claude
  at the owner's request, with the crates.io token configured on the
  owner's machine (`~/.cargo/credentials.toml`): `--dry-run` first, then
  `protocontract`, then the plugin.
- Every release publishes both crates by hand: step 4 of
  `.claude/skills/versioning/SKILL.md`, run by the owner or at his request.
- The README's shields.io badges (version, license) cache a crate's
  answer for one and four hours: a badge loaded before a publish shows
  "not found" until then.

## Still to do

1. **Trusted Publishing**, so the release workflow publishes instead of
   the owner: configure it in each crate's crates.io settings (repository
   `sonalect/protocontract-buffa.rs`, workflow `release.yml`). Then a
   `publish` job after `release`, with `id-token: write` and the crates.io
   auth action pinned by commit SHA (quarantine), runs `cargo publish
   --locked` for `protocontract`, then the plugin; step 4 of the
   versioning skill then becomes "check the job".
2. **Social preview** of the GitHub repository (Settings → General →
   Social preview, 1280×640 PNG), if not uploaded yet: GitHub has no API
   for it. Render `assets/social-preview.svg` to PNG (for example
   `resvg social-preview.svg social-preview.png` in `assets/`).
3. Optional: the organization as an owner
   (`cargo owner --add github:sonalect:<team>`), once the organization
   grants the crates.io application access on GitHub.

**Why:** the next publish starts from this state.
