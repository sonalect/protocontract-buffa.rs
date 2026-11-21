---
name: versioning
description: >-
  Bump the protocontract-buffa crates in lockstep
  (Rust-style 0.x or SemVer after 1.0.0), update CHANGELOG.md, create the
  matching GitHub tag, whose CI workflow tests six platforms and publishes
  the GitHub Release with the plugin binaries, then publish both crates on
  crates.io. Use when releasing, tagging, publishing a GitHub release or a
  crate,
  bumping version, editing the workspace package version, or
  adding a CHANGELOG section.
---

# Versioning

One release version for the whole repository: the runtime crate
`protocontract` and the plugin crate `protoc-gen-protocontract-buffa`. The status line of `DESIGN.md` is **not** this number.

## Scheme

Versions are `MAJOR.MINOR.PATCH`. Cargo `version = "0.0.1"` has **no**
`v`. The git tag **does**: `v0.0.1`.

If **major is 0** (Rust approach, also stated in `CHANGELOG.md`):

- Breaking change → bump **minor** (`0.0.1` → `0.1.0`)
- Bugfix or compatible feature → bump **patch** (`0.0.1` → `0.0.2`)

If **major > 0** (standard SemVer):

- Breaking → **major**
- Compatible features → **minor**
- Bugfixes → **patch**

Do not skip numbers. Do not jump to `1.0.0` unless the owner explicitly
wants a stable public API.

**Breaking** means consumer-visible (`.claude/rules/consumers.md`): a
renamed or removed generated item, a changed generated signature or module
layout, a removed or re-meant plugin parameter or default, a changed
runtime type, or a changed error code. Toolchain and CI pins (mise
tools, rustc) are a patch (0.x) or minor (≥1.0) unless they force a breaking
change. A bump of the pinned buffa or connect-rust that changes the
generated code is breaking.

The versions `mise.toml` pins are **not** this number.

## Lockstep files

Set the same `MAJOR.MINOR.PATCH` everywhere:

- Root `Cargo.toml`: `[workspace.package] version` (members use
  `version.workspace = true`)
- `CHANGELOG.md`: move `## [Unreleased]` notes into `## [MAJOR.MINOR.PATCH] -
  YYYY-MM-DD`; add the version to `## Links` and the reference definitions
  (`[Unreleased]` compare from the new tag, `[MAJOR.MINOR.PATCH]` release URL)

- `README.md`: the `protocontract = { version = … }` examples name the
  compatible range, `MAJOR.MINOR` while major is 0 (`"0.3"`), `MAJOR`
  after; change them only when a release leaves that range.

## Release

Only when the owner asked to release, and only from `main`:

1. Commit the version bump and `CHANGELOG.md` on `main` and push it.
2. Run the workflow by hand on `main` and wait for it: the same six
   platforms and binaries, no Release.

   ```bash
   gh workflow run Release --ref main
   gh run watch "$(gh run list --workflow Release --branch main --limit 1 --json databaseId -q '.[0].databaseId')"
   ```

3. Create the annotated tag with the release notes as its message and push
   it, exactly as `.claude/rules/release-tag.md` says (a file,
   `--cleanup=verbatim`, read back before the push). Do not backfill older
   tags.
4. When the GitHub Release is published, publish the crates from the
   tagged commit, runtime first:

   ```bash
   git checkout vX.Y.Z
   cargo publish --locked --dry-run -p protocontract
   cargo publish --locked -p protocontract
   cargo publish --locked --dry-run -p protoc-gen-protocontract-buffa
   cargo publish --locked -p protoc-gen-protocontract-buffa
   git checkout main
   ```

   The owner runs it: it needs the crates.io token (`cargo login`). A
   published version can only be yanked, never replaced, so a release with
   a problem gets a new patch version. The state of the publish (first
   publish, Trusted Publishing from the workflow) is in
   `.claude/notes/crates-io-publish.md`.

The tag starts `.github/workflows/release.yml`; nothing else does. It:

1. fails at once unless the tag is annotated, points at a commit on
   `main`, has a message body, and is `v` plus the version in `Cargo.toml`
   with `## [MAJOR.MINOR.PATCH]` in `CHANGELOG.md`;
2. runs `mise run gate` on six platforms
   (`linux_amd64`, `linux_arm64`, `darwin_amd64`, `darwin_arm64`,
   `windows_amd64`, `windows_arm64`), and on each builds the plugin's
   static release binary with Cargo (`--profile dist`);
3. when all six pass, publishes the GitHub Release for the tag: the six
   binaries, named `protoc-gen-protocontract-buffa-{tag}-{file}` (`DESIGN.md`
   §9), and `checksums-sha256.txt` for all six; the tag message's subject is the title,
   its body the notes. Not a draft, not a prerelease.

Do not run `gh release create` by hand. A tag whose workflow failed has no
Release: fix the cause on `main`, delete the tag locally and on `origin`,
and tag the fixed commit.

Watch the run with `gh run watch` and return the Release URL when it
succeeds. The `registry.bzl` entry in `bazel_utils` (URL template and the
sha256 of each platform, from `checksums-sha256.txt`) is a change in that
repository: prepare it only when the owner asks.

Examples: `v0.0.1`, `v0.0.2`, `v0.1.0`.
