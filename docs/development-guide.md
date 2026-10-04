# Development Guide

This is the authority for development conventions in auto-tuple. Start here
and read [design.md](design.md) before changing behavior. Adapted from the
sibling batch-impl guide; its project-specific pipeline, dependencies,
consumer claims and test layout do not apply here.

## 1. Collaboration and Style

- Communicate with the author in Chinese. Write code comments, Rust API docs
  and commit messages in English; keep the existing design document in Chinese.
- Make contracts visible in types and named functions. Prefer a small enum or
  structured configuration over boolean combinations whose meaning is implicit.
- Prefer inference, iterator/combinator chains and concise idiomatic Rust when
  they improve readability. Avoid unnecessary annotations and intermediates.
- Keep predicates and syntax definitions in one authoritative place. Share
  selection rules between parsing, resolution and tests rather than duplicating
  name tables across modules.
- Use patch/edit tools for literal file edits. Do not use PowerShell text
  replacement or batch rewriting; shell commands are fine for builds and reads.

## 2. Architecture and Diagnostics

- The pipeline is attribute parsing -> member selection -> Shared/All analysis
  -> type rewriting and code generation. Resolve exclusions before inspecting
  selected signatures or checking their supported shapes.
- `docs/design.md` owns semantics. Update it before implementation changes;
  generated helper names used by downstream crates are a compatibility contract.
- User input must produce a result or `syn::Error`, never a macro panic. New
  production code must not use `unwrap`, `expect`, `panic!`, `unreachable!` or
  assertions. Avoid unchecked indexing and fallible `parse_quote!` templates.
  Tests may assert and unwrap. This rule is not a claim that legacy code has
  already been fully audited: keep known gaps explicit and add regressions when
  fixing them.
- Point diagnostics at the offending input token. Unknown member names and
  unknown selector families should identify that name, not the whole trait.
- Selection does not silently discard unsupported items. If selected, they
  must be validated and diagnosed; an empty selection must never become a
  default selection by accident.

## 3. Tests and Quality Gate

Use inline unit tests for parsing, selection, analysis and rewriting.
`tests/pass/*.rs` contains real compile-and-run examples; `tests/fail/*.rs`
and neighboring `.stderr` files lock rejected inputs. Property tests exercise
macro expansion with both trait syntax and attribute configurations.

Before completing a change, run:

```sh
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

`cargo test` includes doctests; use `cargo test --doc` for focused documentation
verification. Build docs with `RUSTDOCFLAGS=-Dwarnings` in CI. Do not add tests
that merely restate the implementation: cover user-visible behavior, invalid
inputs and interactions such as selection x generic track x arity.

Review intentional diagnostic changes before accepting snapshots. Use
`TRYBUILD=overwrite cargo test --test ui` only for those changes. CI currently
runs the full matrix on stable Linux and Windows; the MSRV job omits diagnostic
snapshots. Do not overwrite baselines merely to conceal toolchain drift.

## 4. Dependencies and Toolchain

- Keep the current stable toolchain, edition 2024 and MSRV 1.98 unless a task
  requires changing them. Any MSRV change needs an actual build and matching CI.
- Reuse `syn`, `quote` and `proc_macro2`; use `syn::Result` for macro errors.
  Do not add dependencies or an async runtime without a concrete requirement.
- This crate emits sequential async calls; it does not own an executor.

## 5. Documentation and Compatibility

- Keep README usage, API docs, the design document and examples consistent.
  README Rust examples are compiled as crate doctests; conceptual syntax
  fragments must be marked `text`, not presented as runnable Rust.
- Record changes under `## Unreleased` in CHANGELOG.md as they land. Preserve
  published version entries as historical behavior.
- Explain breaking changes and provide a migration example. For a 0.x crate,
  an incompatible public change requires a new minor release.
- Development after 0.2.0 targets the next release; the 0.2.0 baseline changed
  implicit selection from methods only to methods + associated constants +
  associated types (see CHANGELOG.md). Keep the manifest's published version
  until release preparation; Unreleased is not a claim that the next version
  has shipped.

## 6. Commits and Release

Use `<type>: <subject>` with an English imperative subject of at most 50
characters. Types: feat, fix, refactor, perf, test, docs, chore, build. A
single-crate commit has no scope. Release commits use `chore: release X.Y.Z`.

For a requested release:

1. Check crates.io versions, GitHub refs/releases and CI before choosing a
   version. Do not infer published state solely from Cargo.toml.
2. Run the quality gate; inspect `cargo package --list` and verify the package.
   Temporary probes, editor files and unrelated notes must not ship. Files
   required by doctests or shipped tests must remain in the package together.
3. Bump Cargo.toml, turn Unreleased into the dated release entry and synchronize
   versioned examples. Remove any "unreleased" / "development toward X" banner
   from README and docs — the README is the crate documentation via
   `include_str!`, so a stale banner ships to docs.rs. Update the baseline
   wording in AGENTS.md and §5 of this guide to the new version. Commit and tag
   the exact tested revision.
4. Push to the repository's actual default branch (`master` currently) and
   require its CI to pass before `cargo publish`.
5. Create a GitHub Release with the final release notes. Do not silently move
   published tags or rewrite an existing release to hide a follow-up fix.

Developing a next version does not itself publish a crate, push commits or
create a GitHub Release.
