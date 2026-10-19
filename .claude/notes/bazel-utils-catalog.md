# bazel_utils catalog entry for the plugin

## State

- `../bazel_utils` 0.4.2 (19 October 2026) catalogs the plugin as
  `protoc-gen-protocontract-buffa`, versions v0.3.1, v0.4.0, and v0.4.1
  (`protoc/plugins/protoc-gen-protocontract-buffa/registry.bzl`), with
  v0.4.1 as the fallback pin; 0.4.1 of bazel_utils added v0.4.0. Scheda
  pins bazel_utils 0.4.2 and the plugin at v0.4.1.
- Knowqore (`../knowqore`) still depends on `sonalect/proto-contract.rs`
  and the old plugin name.

## How to add a release

In `../bazel_utils`, when the owner asks: add the version to
`protoc/plugins/protoc-gen-protocontract-buffa/registry.bzl` with the
sha256 values from that Release's `checksums-sha256.txt`, as its
`CLAUDE.md` says ("Adding a protoc plugin"), and release bazel_utils.

## A bug found there: `buf_generate` scratch paths

`buf/generate.bzl` names its scratch directories after the target alone
(`prefix = ctx.label.name`: `<execroot>/<name>.plugin_bin`, `.work`,
`.home`, `.buf-cache`). On Windows, where actions run unsandboxed, two
`buf_generate` targets of one name in different packages that run together
collide: `cp: can't create …/generated.plugin_bin/protoc-gen-buffa.exe:
File exists`. Seen on 18 October 2026 in this repository's release
workflow on both Windows runners (`//examples/glossary:generated` and
`//examples/refs:generated`); worked around here by naming the second
target `refs_generated`. The fix belongs in bazel_utils: take the package
into the prefix (or declare the directories as action outputs).

**Why:** a consumer pins the plugin by its catalog name, and the binary
names in a Release are the catalog's download URLs. The scratch-path bug
hits any repository with two `buf_generate` targets of one name.
