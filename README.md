# simple-term

[![CI](https://github.com/ttaatoo/simple-term/actions/workflows/ci.yml/badge.svg)](https://github.com/ttaatoo/simple-term/actions/workflows/ci.yml)
[![Release](https://github.com/ttaatoo/simple-term/actions/workflows/release.yml/badge.svg)](https://github.com/ttaatoo/simple-term/actions/workflows/release.yml)
[![GitHub release](https://img.shields.io/github/v/release/ttaatoo/simple-term)](https://github.com/ttaatoo/simple-term/releases)

A standalone desktop terminal built with Rust, [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui), and `alacritty_terminal`.

## Why this project

`simple-term` separates terminal core logic from app UI glue so behavior can be tested, evolved, and released with clear boundaries.

- `apps/simple-term`: desktop app entrypoint and UI orchestration
- `crates/simple-term`: reusable terminal core, mappings, settings, PTY glue

## Current platform support

- Official release artifacts: **macOS only** (currently `macos-arm64`)
- Published formats: `.dmg`, `.tar.gz`, and `SHA256SUMS.txt`
- CI verification: runs on macOS
- Linux/Windows source builds are not part of the current release contract

## Quick start

### Install from GitHub Releases

1. Download the latest release asset from [Releases](https://github.com/ttaatoo/simple-term/releases).
2. Preferred: open the `.dmg` and drag `SimpleTerm.app` to `Applications`.
3. Alternative: extract the `.tar.gz` and run the `simple-term` binary.

Example:

```bash
open https://github.com/ttaatoo/simple-term/releases/latest
```

### Build and run from source

Requirements:

- Rust stable toolchain
- macOS development environment

Commands:

```bash
cargo check --workspace
cargo run -p simple-term-app
```

## Project layout

Active workspace members are defined in root `Cargo.toml`:

```text
apps/simple-term
crates/simple-term
```

## Configuration

Settings are loaded from JSON via `TerminalSettings::load(...)`.

- Config directory: `ProjectDirs("com", "simple-term", "SimpleTerm")`
- Config file: `settings.json` in that directory

On missing/invalid config, sane defaults are used.

### Example `settings.json`

```json
{
  "shell": { "type": "system" },
  "font_size": 14,
  "font_family": "Menlo",
  "line_height": { "type": "comfortable" },
  "default_width": 960,
  "default_height": 600,
  "max_scroll_history_lines": 10000,
  "scroll_multiplier": 3.0,
  "option_as_meta": false,
  "copy_on_select": false,
  "keep_selection_on_copy": true,
  "path_hyperlink_timeout_ms": 500
}
```

## Architecture at a glance

1. UI captures keyboard/mouse input in `TerminalView`.
2. Input is mapped to protocol bytes or local UI actions.
3. Core terminal backend processes PTY I/O and emits events.
4. View consumes events and repaints from lock-free snapshots.

Detailed invariants and flow are documented in:

- [`docs/architecture-invariants.md`](docs/architecture-invariants.md)
- [`docs/evolution/INDEX.md`](docs/evolution/INDEX.md)

## Development workflow

Before opening a PR:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Mandatory evolution docs policy

For code or process changes (app/core/workspace/workflows), update `docs/evolution/` with rationale and safe-change guidance.

- Policy source: [`AGENTS.md`](AGENTS.md)
- Enforced by CI script: [`.github/scripts/require-evolution-docs.sh`](.github/scripts/require-evolution-docs.sh)

## Release process

- Version source: `[workspace.package].version` in root `Cargo.toml`
- Tag format: `vX.Y.Z`
- Release workflow: [`.github/workflows/release.yml`](.github/workflows/release.yml)

Manual release path:

1. Update workspace version.
2. Run GitHub Actions `Release` workflow with `version` and `ref`.
3. Verify generated assets (`.dmg`, `.tar.gz`) and `SHA256SUMS.txt`.

## Troubleshooting

See [`docs/troubleshooting.md`](docs/troubleshooting.md) for known issues and diagnostics guidance.

## License

MIT
