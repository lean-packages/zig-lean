# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-12

### Added
- Initial release of `zig-lean`.
- `lint` command: Inspect and validate `build.zig.zon` `.paths` entries, detecting missing files, root escapes, duplicate
entries, redundant child entries, broad root inclusions, and non-release development artifacts.
- `inspect` command: Detailed report of effective package contents, file counts, and sizes with human-readable and JSON
output formats.
- `update` command: Safely derive and apply a clean, minimal `.paths` allowlist to `build.zig.zon`, preserving original
comments, whitespace, and unrelated manifest fields.
