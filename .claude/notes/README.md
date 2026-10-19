# Project notes

Project state, deferred work, measurements, and references that must
outlive one machine (see `.claude/rules/project-notes.md`). Read this index
before planning; open a note when its topic comes up.

| Note | What it holds |
| - | - |
| [bazel-utils-catalog.md](bazel-utils-catalog.md) | The plugin's bazel_utils catalog entry (v0.3.1, v0.4.0, v0.4.1 in bazel_utils 0.4.2), and the `buf_generate` scratch-path bug found there |
| [crates-io-publish.md](crates-io-publish.md) | crates.io: both crates published (0.4.1, by hand); Trusted Publishing and the social preview still to do |

## References

- Design and decisions: `DESIGN.md` at the repository root.
- Anchor consumer: Knowqore, `github.com/sonalect/knowqore`, usually
  cloned as `../knowqore` (`.claude/rules/consumers.md`).
- Plugin catalog for Bazel: `github.com/sonalect/bazel_utils`,
  `protoc/plugins/` (one `registry.bzl` per plugin).
