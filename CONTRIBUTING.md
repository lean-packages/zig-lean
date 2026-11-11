# Contributing

Thank you for your interest in contributing to `zig-lean`.

## Prerequisites

- Rust 1.85+ — the project uses the 2024 edition.
- [Just](https://just.systems/) — recipe runner for local development commands.

## Getting Started

After cloning the repository, follow these steps to set up the project:

```bash
git clone git@github.com:lean-packages/zig-lean.git && cd zig-lean
```

Install Rust dependencies. Cargo automatically resolves and downloads all dependencies declared in `Cargo.toml`. Simply
run any Cargo command to trigger the download:

```bash
cargo check
```

This compiles just enough to verify that all dependencies are available and the project builds correctly. For a full build:

```bash
cargo build
```

## Development Workflow

We use `just` to streamline common development tasks. Run `just` without arguments to see available recipes:

```bash
just
```

### Running Checks

Before submitting a pull request, ensure all checks pass:

```bash
just check
```

This runs:
1. `cargo fmt -- --check` — Verifies code formatting.
2. `cargo clippy -- -D warnings` — Runs the linter with warnings treated as errors.
3. `cargo test` — Runs the test suite.

### Formatting

To automatically format your code:

```bash
cargo fmt
```

### Testing

Run the test suite with:

```bash
just test
```

Or directly with Cargo:

```bash
cargo test
```

## Pull Request Guidelines

1. **Create a branch**: Never work directly on `main`. Create a feature branch:
   ```bash
   git checkout -b my-feature
   ```

2. **Write tests**: Add tests for new features or bug fixes. Ensure all tests pass.

3. **Follow conventions**: Match the established code style, error handling patterns, and CLI conventions.

4. **Run checks**: Verify that `just check` passes before opening a PR.

5. **Write clear commit messages**: Keep commit messages concise and descriptive.
