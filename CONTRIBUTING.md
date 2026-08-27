# Contributing

## Commits

Lightweight [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>: <subject>
```

- `type` is one of: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`,
  `chore`, `build`.
- `subject` is an English imperative, at most 50 characters.
- No scope for single-crate commits.

## Development

Before committing, run the full quality gate:

```sh
cargo check
cargo clippy --all-targets -- -D warnings
cargo test          # unit tests + trybuild compile matrix
cargo fmt --check
```

`trybuild` is used for the compile-test matrix: `tests/pass/*.rs` must build
and run their assertions, `tests/fail/*.rs` must fail with a message matching
the adjacent `.stderr` baseline. When an intended diagnostic changes, delete
the stale `.stderr`, rerun, and commit the regenerated baseline.

## Design

See `docs/design.md` for the semantics, the two-track (shared / All) decision
procedure, the type-rewriting rules and the edge-case matrix. Keep it in sync
when changing behavior.
