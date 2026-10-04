# Contributing

Start with [the development guide](docs/development-guide.md), the authority
for architecture boundaries, style, testing, compatibility and release flow.

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
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test          # unit tests + trybuild compile matrix
cargo fmt --check
cargo doc --no-deps
```

`trybuild` is used for the compile-test matrix: `tests/pass/*.rs` must build
and run their assertions, `tests/fail/*.rs` must fail with a message matching
the adjacent `.stderr` baseline. For an intentional diagnostic change, use
`TRYBUILD=overwrite cargo test --test ui`, inspect the diff and commit the
reviewed baseline. `cargo test` also runs the README and API doctests.

## Design

See `docs/design.md` for the semantics, the two-track (shared / All) decision
procedure, the type-rewriting rules and the edge-case matrix. Keep it in sync
when changing behavior.
