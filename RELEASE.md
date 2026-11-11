# Release process

This document describes how to release a new version of `zig-lean` to [crates.io](https://crates.io).

## Prerequisites

- Install [`just`](https://github.com/casey/just) if you haven't already.
- Ensure you have a crates.io API token configured (e.g., via `~/.cargo/credentials.toml`).
- Be on the `main` branch with a clean working tree.

## Release steps

1. Update `CHANGELOG.md` with the new version and a summary of changes.


2. Run the pre-release checks:
   ```bash
   just check
   ```
   This runs `fmt`, `clippy`, and `test`.


3. Preview the release:
   ```bash
   just release-dry-run
   ```
   To preview a specific release level:
   ```bash
   just release-dry-run minor
   ```

4. Execute the release:
   ```bash
   just release
   ```
   To release a specific level:
   ```bash
   just release minor
   ```

## What `cargo-release` does

The `just release` command invokes `cargo release --execute`, which automatically:

1. Bumps the version in `Cargo.toml`
2. Runs pre-release checks (`cargo package --locked`)
3. Commits the version bump with the message `dist: ship version <version>`
4. Creates and pushes a `v<version>` tag

Configuration is stored in `release.toml` (e.g., allowed branches, tag format, commit messages).
