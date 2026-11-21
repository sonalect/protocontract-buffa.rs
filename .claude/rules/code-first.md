# Code first, documents after

Behaviour ships before prose. On a change, the code and its tests come
first, the gates run, and only then the documents move — **once**.

## Order

1. Code and tests.
2. `cargo test` and `cargo clippy` green.
3. Documents: one pass, in the same commit or the next one. In protocontract-buffa
   the normative text is `DESIGN.md`, README, and CHANGELOG: mark a stage
   of `DESIGN.md` done only when its proof passes under `mise run gate`.

## Budget

Prose is about a tenth of the effort on a change, not most of it. One
pass over the affected files, then stop.

## Do not

- Edit a document before the behaviour it describes exists.
- Open the same document a second and a third time to polish wording.
- Treat status-line updates, cross-reference sweeps, and renumbering as their own activity. Fold
  them into the one pass.
- Ask the owner to approve wording. Ship the code; the document follows.
- Answer "what is the next step?" with a document task while code waits.

## Exception

The owner asked for a plan, a design, a review, or a design document.
Then prose **is** the deliverable and this rule does not apply.
`full-review.md` and `readiness.md` keep priority.
