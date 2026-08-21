---
name: anpai-code
description: Maintain and extend the anpai Rust workspace, including the FEEL interpreter, DMN parser/evaluator, XML utilities, CLI, dependencies, and CI. Use this skill for project code changes, bug fixes, tests, or Rust workflow maintenance.
---

# anpai Code

## Project Shape

`anpai` is a Rust workspace for BPMN/DMN tooling. Its main pieces are:

- `crates/feel`: FEEL scanner, parser, AST, evaluator, values, and built-in prelude functions.
- `crates/dmn`: DMN XML parsing and decision-table evaluation built on the FEEL engine.
- `crates/anpaiutils`: shared XML parsing and XPath helpers.
- `crates/anpai`: the `anpai` CLI for evaluating FEEL expressions and DMN files.

The normal data path is scanner -> parser/AST -> `feel::eval::Engine`; DMN evaluation parses XML, evaluates FEEL input/output entries, and returns a FEEL `Value`.

## Toolchain and Dependencies

- Use the repository-pinned stable toolchain from `rust-toolchain.toml`; do not reintroduce nightly-only features or commands.
- Workspace-level dependency versions belong in the root `Cargo.toml`; crate manifests should use `*.workspace = true` for shared dependencies.
- Update `Cargo.lock` with Cargo when dependency versions change. Do not edit the lockfile by hand.
- Preserve the Rust 2021 edition and workspace resolver unless the task explicitly requires a migration.

## Implementation Guidance

- Preserve FEEL semantics when refactoring: context scope push/pop, three-valued/null behavior, range boundary handling, temporal parsing, and evaluation error positions are observable behavior.
- Reuse the existing `Value`, `Numeric`, `Context`, `RangeT`, `Engine`, and prelude helpers instead of introducing parallel representations.
- Keep parser and evaluator errors in their existing types so callers and the CLI retain useful source positions and line pointers.
- For DMN or XML changes, preserve the existing conversion to `DmnError`/`XmlError` and keep XPath node ownership and lifetimes correct.
- Treat public compatibility as important. Prefer additive API changes or compatibility aliases when renaming public helpers; update call sites and tests together.
- Keep Clippy suppressions local and justified. Do not disable broad lint groups to hide warnings, especially for intentional boxed AST ownership or partial ordering behavior.
- Add focused tests next to the affected module. Prefer semantic assertions over snapshots of implementation details.

## DMN Decision Tables

`crates/dmn` currently evaluates these hit policies through separate policy methods:

- `FIRST` (the parser default): return the first matching rule in table order.
- `UNIQUE`: allow at most one matching rule; multiple matches are a `DmnError::HitPolicy`.
- `COLLECT`: evaluate every matching rule and collect each output field into an array; no matches produce empty arrays.
- `ANY`: allow multiple matching rules only when their rendered output contexts are equal.
- `PRIORITY`: evaluate every matching rule and select the output with the highest priority from the output clause's allowed values.

Output allowed values are read from `<allowedValues><text>...</text></allowedValues>` and retained in declaration order on `Output::allowed_values`; the first value has the highest priority. `PRIORITY` requires allowed values and rejects output values not listed there. Other DMN policies are not implemented and must return an explicit hit-policy error rather than silently using `FIRST`.

When modifying hit-policy behavior, keep matching, output rendering, and policy selection separable. Add tests for multiple matches, no matches, policy violations, and output parsing. Condition parse/evaluation failures must remain `DmnError::FEELEval` with the relevant DMN path and source text.

## CLI Behavior

The `anpai` binary exposes two subcommands:

- `anpai feel`: evaluate code from `--code` or files/stdin, optionally loading `--vars`/`--varsfile`, selecting a parse `--top`, or dumping the AST with `--ast` and `--json`.
- `anpai dmn`: evaluate a DMN file with optional context variables and `--start-decision-id`.

When changing CLI behavior, preserve the existing command names, flags, output format, and human-readable error reporting unless the request explicitly changes them.

## Validation

Run the smallest relevant checks while iterating, then run the full workspace checks before handoff:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --locked --offline
```

Use `cargo build` for CLI or compilation-only changes. If dependency resolution requires network access, state that clearly and use the repository lockfile when possible.

CI is defined in `.github/workflows/rust.yml` and should continue to run stable Rust, formatting, strict Clippy, build, and tests. Keep workflow changes aligned with the local commands above.

## Change Boundaries

Inspect the relevant crate and its tests before editing. Keep changes scoped to the requested behavior, avoid unrelated refactors, and check `git diff --check` before finishing. Summarize changed crates and verification results in the final handoff.
