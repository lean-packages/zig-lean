# zig-lean

Validate and manage `build.zig.zon` package path allowlists to keep your published Zig packages lean.

`zig-lean` is a Zig counterpart to [cargo-lean](https://github.com/lean-packages/cargo-lean). It audits and optimizes the `.paths` field in `build.zig.zon` manifests so repository-only files don't unintentionally end up in published package archives.

## Why?

When you publish a Zig package, the Zig package manager packages up files according to the explicit `.paths` allowlist in `build.zig.zon`. If package authors specify broad root entries (like `""` or `"."`) or include directories without clean boundaries, published archives can unintentionally include development artefacts like test suites, build caches (`zig-cache/`, `zig-out/`), IDE settings, documentation drafts, and CI configuration. These files bloat the archive download size, slow down dependency fetching, and expose internal project structure to downstream consumers.

`zig-lean` makes sure your `build.zig.zon` `.paths` list is clean and minimal so only the source code and metadata your users need end up in the published package archive.

Your working repository remains intact: `zig-lean` configures what is included in the published package archive; it does not delete those files from your project.

## Installation

### From crates.io

```bash
cargo install zig-lean
```

### From source

```bash
git clone git@github.com:lean-packages/zig-lean.git && cd zig-lean
cargo install --path .
```

This places the `zig-lean` binary in `~/.cargo/bin`, which should be on your PATH.

## Usage

```
zig-lean <COMMAND>
```

### Commands

| Command   | Description                                                                    |
|-----------|--------------------------------------------------------------------------------|
| `lint`    | Check `build.zig.zon` `.paths` for missing files, root escapes, and artefacts. |
| `inspect` | Inspect effective package contents, file counts, and archive sizes.            |
| `update`  | Propose and safely update `.paths` in `build.zig.zon`.                         |

### `lint` — Check manifest path completeness and validity

Validates that all paths listed in `build.zig.zon` `.paths` exist, stay within the package root, and do not contain redundant entries or development artefacts.

```bash
# Check the current directory
zig-lean lint

# Check a specific manifest
zig-lean lint --manifest-path /path/to/build.zig.zon

# Treat warnings as errors
zig-lean lint --strict

# JSON output for scripting
zig-lean lint --format json
```

### `inspect` — Inspect package archive contents

Analyzes and summarizes effective package contents, total file count, and archive size based on the `.paths` allowlist.

```bash
# Human-readable report
zig-lean inspect

# Inspect a specific manifest
zig-lean inspect --manifest-path /path/to/build.zig.zon

# JSON output
zig-lean inspect --format json
```

### `update` — Optimize `.paths` allowlist

Analyzes repository files and proposes a minimal, optimal `.paths` allowlist containing only essential package files (e.g. `build.zig`, `build.zig.zon`, `src`, and package metadata).

`update` runs in dry-run mode by default to preview proposed changes with a unified diff. Pass `--write` (`-w`) to write changes atomically to `build.zig.zon`, preserving original comments, indentation, and formatting.

```bash
# Dry run: see what would change
zig-lean update

# Apply changes to build.zig.zon
zig-lean update --write

# JSON output
zig-lean update --format json
```

## Options and flags

All commands support these common options and flags:

| Option / Flag                | Description                                        |
|------------------------------|----------------------------------------------------|
| `-m, --manifest-path <PATH>` | Path to `build.zig.zon` (default: `build.zig.zon`)  |
| `--format <FORMAT>`          | Output format: `text` or `json` (default: `text`)   |
| `--strict`                   | Treat warnings as errors (`lint`)                  |
| `--zig-exe <PATH>`           | Custom Zig executable path for format checks (`lint`) |
| `-w, --write`                | Write changes to manifest (`update`)               |

Note: When using `--manifest-path`, the package root directory is derived from the parent directory of the provided `build.zig.zon`.

## CI integration

Add `zig-lean lint` to your CI pipeline to enforce that package path allowlists stay clean and valid:

```yaml
# Example GitHub Actions step
- name: Check build.zig.zon package paths
  run: zig-lean lint --strict
```

## JSON output

All commands support `--format json` for scripting and automation. Example `lint` output:

```json
{
  "manifest_path": "build.zig.zon",
  "valid": true,
  "error_count": 0,
  "warning_count": 0,
  "notice_count": 0,
  "diagnostics": []
}
```

## AI disclaimer

This package is developed with the assistance of Artificial Intelligence (AI). AI tools are used throughout the development
process, including code generation, documentation, testing, and maintenance.

While reasonable care is taken to review and validate AI-generated contributions, AI-generated code is not inherently correct,
secure, or free of defects. All contributions remain subject to human review and project quality standards.

As with any software, use this package at your own risk. No guarantee is made regarding its fitness for a particular purpose,
and the maintainers assume no liability for damages resulting from its use.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for development instructions.

## License

This project is licensed under the MIT license. Please see [LICENSE.md](LICENSE.md) for more details.

## Related projects

- [php-lean](https://github.com/lean-packages/php-lean) — The `PHP` counterpart.
- [py-lean](https://github.com/lean-packages/py-lean) — The `Python` equivalent.
- [cargo-lean](https://github.com/lean-packages/cargo-lean) — The `Rust` equivalent.
- [js-lean](https://github.com/lean-packages/js-lean) — The `JavaScript` equivalent.
