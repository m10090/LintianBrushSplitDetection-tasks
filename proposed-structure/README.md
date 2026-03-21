# lintian-brush detector/action split (proposal preview)

This directory is a preview implementation for a proposed lintian-brush refactor:

- split detection from fixing,
- represent issues as structured data,
- attach fix actions to issues,

It is intentionally limited in scope and currently focused on `debian/control`.

## Project context

Current lintian-brush fixers do detection and fixing in one pass.

The proposal is to move to:

1. `Detector` phase: find issues and return structured diagnostics
2. `Action/Fixer` phase: apply selected fixes for chosen issues

This model is needed for LSP workflows, where users see diagnostics and choose quick-fixes.

Confirmed mentor: Jelmer Vernooij
Co-mentor: Otto Kekalainen

## What this preview implements

- Trait-based split between detection and fixing:
  - `Detector`
  - `Fixer`
- Structured issue model:
  - `DetectedIssue { tag, description, package, package_type, line, field }`
- Registration by macro + `inventory`:
  - `declare_detector!`
  - `declare_fixer!`
- Tag to fixer mapping (`TAG_TO_FIXERS`) for routing issues to fixers
- Mutable editing with `debian-analyzer::TemplatedControlEditor`
- Simple CLI supporting detect-only and fix flows
- Fixture-based tests for detect/fix behavior

Implemented rules in this preview:

- `debian-control-has-empty-field`
- `installable-field-mirrors-source`
- `cute-field`
- `debian-control-has-unusual-field-spacing`

## Not implemented yet

- Topological sorting / dependency-aware ordering of detectors/fixers
- Full LSP diagnostic/code-action protocol output
- Diagnostic/Fix of of Files other than debian/control
- Lintain Overrides


If you only have a few minutes, review in this order:

1. `src/lib.rs`
   - core data structures
   - detector/fixer traits
   - detect/fix orchestration
2. `src/macros.rs`
   - registration model for detectors/fixers
3. `src/detectors/`
   - detection-only logic
4. `src/fixers/`
   - issue-driven fix application
5. `src/cli.rs`
   - detect-only vs fix mode behavior
6. `test/hello-debian/debian/control`
   - end-to-end fixture with known issues

## CLI

Entrypoint: `src/cli.rs` (`fn main`).

Default mode is detect-only.

When using `--fix`, default behavior is still dry-run. Nothing is written unless `--apply` is provided.

Flags:

- `--fix` run fixers after detection
- `--apply` write changes (default is dry-run)
- `--fixer <name>` restrict to specific fixer(s)
- `--path <dir>` target package path (default `.`)

Examples:

```bash
# detect only
cargo run -- --path test/hello-debian

# dry-run fix
cargo run -- --fix --path test/hello-debian

# dry-run a specific fixer only
cargo run -- --fix --fixer field-name-typo-in-control --path test/hello-debian

# apply fixes
cargo run -- --fix --apply --path test/hello-debian

# apply a specific fixer only
cargo run -- --fix --apply --fixer field-name-typo-in-control --path test/hello-debian
```

## Testing

Smoke-check CLI:

```bash
cargo run -- --help
cargo run -- --path test/hello-debian
cargo run -- --fix --path test/hello-debian
```
