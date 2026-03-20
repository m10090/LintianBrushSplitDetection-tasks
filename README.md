# Lintian Brush Split/Parse Detection

This workspace is a small, practical setup to:

- Parse a Debian `debian/control` file and print binary package names (Rust).
- Build and run `lintian-brush` from the local `debian-codemods/` workspace.
- Profile `lintian-brush` to see how often the same packaging files get opened/parsed.
- Spot fixer overlap (multiple fixers touching the same files like `debian/control`).

## Repo Layout

- `debian-control-parser/`: tiny Rust CLI that prints binary package names from a `debian/control`.
- `debian-codemods/`: upstream workspace containing `lintian-brush`.
- `hello-debian/`, `openssl/`: sample Debian packaging repos for testing.
- `output.trace`: example `strace` output.
- `filtered-output.trace`: extracted paths from `output.trace` (focused on Debian packaging files).

## Run and Build This Project

From the workspace root:

This workspace expects the full repository tree (including nested projects) to be present.
If you clone it with git, make sure submodules are initialized:

```sh
git clone --recurse-submodules <repo-url>
```

If you already cloned without submodules:

```sh
git submodule update --init --recursive
```

```sh
# Build both Rust components used in this workspace
cargo build --manifest-path debian-control-parser/Cargo.toml
cargo build --release --manifest-path debian-codemods/Cargo.toml -p lintian-brush
```

Run the parser:

```sh
cargo run --manifest-path debian-control-parser/Cargo.toml -- hello-debian/debian/control
```

Run `lintian-brush` (dry-run) against a sample package tree:

```sh
debian-codemods/target/release/lintian-brush --dry-run --directory openssl
```

## Debian Control Parser

Build/run:

```sh
cargo run --manifest-path debian-control-parser/Cargo.toml [-r] -- hello-debian/debian/control
```

Behavior:

- Accepts an optional path argument.
- Defaults to `debian/control` if no argument is provided.
- Prints one binary package name per line.

Example with a repo that has multiple binaries:

```sh
cargo run --manifest-path debian-control-parser/Cargo.toml [-r] -- openssl/debian/control
```

## Build lintian-brush (from this workspace)

Build the `lintian-brush` binary:

```sh
cargo build --release --manifest-path debian-codemods/Cargo.toml -p lintian-brush
```

Binary path after a successful build:

- `debian-codemods/target/release/lintian-brush`

Run it against a package checkout:

```sh
debian-codemods/target/release/lintian-brush --dry-run --directory openssl
```

Notes:

- `--directory` points at the Debian package tree.
- `--dry-run` currently creates a temporary clone of the repository (so file activity will show up under a temp dir like `/tmp/.tmpXXXXXX`).

## Profile File Re-Reads With strace

Capture file opens while running `lintian-brush`:

```sh
strace -f -e trace=open,openat -o output.trace \
  debian-codemods/target/release/lintian-brush --dry-run --directory openssl
```

Extract opened paths that look like Debian packaging files (simple, best-effort):

```sh
rg -o --replace '$1' '\bopen(?:at)?\([^\"]*"([^\"]*/debian/[^\"]*)"' output.trace \
  > filtered-output.trace
```

Count the most frequently opened paths:

```sh
printf 'total_lines %s\n' "$(wc -l < filtered-output.trace)"
printf 'unique_paths %s\n' "$(sort -u filtered-output.trace | wc -l)"
printf '\nTop 20:\n'
sort filtered-output.trace | uniq -c | sort -nr | head -n 20
```

In the sample trace checked into this repo (`filtered-output.trace`), the hottest files were:

- `debian/changelog` (207 opens)
- `debian/control` (96 opens)
- `debian/rules` (37 opens)
- `debian/copyright` (40 opens)

## Fixers That Touch The Same Files

Many fixers read and/or edit the same packaging files (especially `debian/control`). To find candidates:

```sh
rg -n 'debian/control|TemplatedControlEditor::open\(' debian-codemods/lintian-brush/src/fixers
```

Example fixers that operate on `debian/control`:

- `debian-codemods/lintian-brush/src/fixers/debian_control_has_empty_field.rs`
- `debian-codemods/lintian-brush/src/fixers/debhelper_but_no_misc_depends.rs`
- `debian-codemods/lintian-brush/src/fixers/built_using_for_golang.rs`
- `debian-codemods/lintian-brush/src/fixers/circular_installation_prerequisite.rs`

Example fixers that read `debian/changelog`:

- `debian-codemods/lintian-brush/src/fixers/unnecessary_team_upload.rs`
- `debian-codemods/lintian-brush/src/fixers/debian_changelog_line_too_long.rs`
- `debian-codemods/lintian-brush/src/fixers/out_of_date_standards_version.rs`
