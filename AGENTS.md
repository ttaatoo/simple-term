# Repository Guidelines

## Project Structure & Module Organization
This repository is a Rust workspace with two members:
- `apps/simple-term`: application entrypoint and UI integration (`src/main.rs`, `src/terminal_view.rs`).
- `crates/simple-term`: reusable terminal library (core terminal logic, mappings, settings, hyperlinks, platform glue).

Top-level `Cargo.toml` defines shared dependencies and workspace settings. Build artifacts go to `target/` and should not be edited manually.

## Build, Test, and Development Commands
Use Cargo from the repository root:
- `cargo check --workspace`: fast type-check across all workspace crates.
- `cargo build -p simple-term-app`: build the desktop terminal binary.
- `cargo run -p simple-term-app`: run the app locally for manual validation.
- `cargo test --workspace`: run unit/integration tests across crates.
- `cargo fmt --all`: apply Rust formatting.
- `cargo clippy --workspace --all-targets -- -D warnings`: lint with warnings treated as errors.

## Coding Style & Naming Conventions
Follow idiomatic Rust and `rustfmt` defaults (4-space indentation, trailing commas where formatter adds them). Keep modules focused and small.

Naming patterns:
- `snake_case`: functions, files, modules (`terminal_settings.rs`).
- `PascalCase`: structs/enums/traits (`TerminalView`, `TerminalBounds`).
- `SCREAMING_SNAKE_CASE`: constants (`DEFAULT_SCROLL_HISTORY_LINES`).

Prefer explicit error propagation (`Result`, `?`) and avoid `unwrap()` in non-test code.

## Testing Guidelines
Place tests close to implementation (`mod tests`) or as integration tests when behavior spans modules. Name tests by expected behavior, e.g., `scroll_report_handles_alt_screen`.

Before opening a PR, run:
1. `cargo check --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

For UI/terminal interaction changes, add manual smoke steps in PR notes (mouse, scroll, title updates, hyperlink open).

## Commit & Pull Request Guidelines
Use Conventional Commits for consistency (e.g., `feat: wire mouse reporting`, `fix: clamp scrollback limit`).

PRs should include:
- concise problem/solution summary,
- linked issue (if available),
- verification evidence (commands run and outcomes),
- screenshots or short recordings for UI-visible changes.

## Security & Configuration Tips
Do not commit local machine paths, shell-specific secrets, or generated artifacts under `target/`. Keep user-configurable defaults in `terminal_settings` paths and validate untrusted input (especially URL/path handling).

## Development History Documentation Policy (LLM-Oriented)
`docs/evolution/` is a required knowledge layer that complements commit history. It must capture architectural rationale, design constraints, safe-change patterns, and known pitfalls.

Rules:
1. Any code or behavior change under `apps/simple-term/`, `crates/simple-term/`, workspace manifests, or CI/release workflows must include an update in `docs/evolution/`.
2. Evolution entries must explain **why** and **how to change safely**, not just list file diffs.
3. Every new evolution entry must be added to `docs/evolution/INDEX.md`.
4. Follow `docs/evolution/README.md` and `docs/evolution/TEMPLATE.md` exactly for structure and writing style.
5. If a change introduces or fixes a recurring mistake pattern, update `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md` (or a newer replacement) with detection and recovery guidance.
