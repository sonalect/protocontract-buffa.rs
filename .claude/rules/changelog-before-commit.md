# Changelog before commit

Before creating a git commit, make `CHANGELOG.md` **actual** for that
commit. Do not commit generator, runtime, plugin-parameter, or
user-facing docs changes while `## [Unreleased]` still omits them.

## What to write

Keep a Changelog groups: `Added`, `Changed`, `Fixed`, `Removed`. English.
The user-facing *why* / effect, not a file list. New notes go under
`## [Unreleased]`. Do **not** bump the crate version here:
that is a release (`.claude/skills/versioning/SKILL.md`).

If Unreleased already states the change, leave it. If it is stale or
wrong, edit it.

```text
# BAD — the async trait changed, Unreleased empty
## [Unreleased]

# GOOD
## [Unreleased]
### Changed
- Async trait methods take the request by value. A caller that keeps the
  request clones it before the call.
```

## Skip (nothing notable)

- The commit is **only** `CHANGELOG.md`.
- rustfmt / comment-only / agent rules (`.claude/**`, `.cursor/**`) with
  no generator, runtime, or `DESIGN.md` change.
- Clippy / rustc lint-only: same generated code, same runtime behaviour.
- The owner explicitly waived the changelog for this commit.

Do not skip for "small" changes to generated code, plugin parameters,
error codes, or bridge behaviour. A lint fix that changes generated
output or what the caller sees is not lint-only.
