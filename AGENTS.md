# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs` is the application entry point; add new Rust modules under `src/` (one module per file, public APIs in `mod.rs` or `lib.rs` if introduced).
- `tests/` is the home for integration tests; unit tests live beside code under `#[cfg(test)] mod tests`.
- `Cargo.toml` defines package metadata, edition (2024), and dependencies.
- `GEMINI.md` contains extended contributor/agent requirements (design, testing, commits).

## Build, Test, and Development Commands
- `cargo build` compiles the project.
- `cargo run` runs the binary locally.
- `cargo check --workspace --all-targets --all-features` performs fast type-checking.
- `cargo fmt --all -- --check` validates formatting (required).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` runs linting (required).
- `cargo test --workspace --all-features` runs the test suite (required).

## Coding Style & Naming Conventions
- Use `cargo fmt`; Rust defaults apply (4-space indentation, trailing commas, etc.).
- Naming: `snake_case` for modules/functions/vars, `CamelCase` for types/traits, `SCREAMING_SNAKE_CASE` for constants.
- Keep functions small and single-purpose; avoid deep nesting and unclear abbreviations.

## Testing Guidelines
- Use Rust’s built-in test framework (`cargo test`).
- Add unit tests in-module and integration tests under `tests/` (e.g., `tests/network_monitor.rs`).
- New or changed behavior should include success, failure, and edge-case coverage.

## Commit & Pull Request Guidelines
- No Git history exists yet; follow Conventional Commits in `GEMINI.md`.
- Header format: `type(scope): imperative summary` (English, ≤72 chars, no period).
- Non-trivial commits require a body with `- What:` / `- Why:` bullets and a `Tests:` line.
- Configure Git authoring: `user.name` must be `DennySORA` and use a real email.
- PRs should describe changes, list tests run, and link relevant issues; include screenshots for UI changes (if any).

## Security & Configuration
- Do not commit secrets; prefer environment variables or local config files excluded by `.gitignore`.
- If you add new configuration, document it in `README.md` or a dedicated `docs/` note.

## Agent Notes
- Automation should follow `GEMINI.md` for required checks and commit rules.
