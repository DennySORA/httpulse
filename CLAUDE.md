# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**httpulse** is a real-time HTTP latency and network quality monitoring tool in Rust. It probes multiple target URLs using configurable HTTP profiles, collects timing/network metrics via libcurl, and displays results in an interactive TUI (Ratatui).

## Build & Development Commands

Use **sequential thinking** for planning.
After every change, **create a commit**.
Use **context7** to look up the **latest** information for any relevant or required packages.

### Formatter (Required)

- `cargo fmt --all -- --check`

### Linter (Required: Clippy)

- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

### Type Checking (Required)

- `cargo check --workspace --all-targets --all-features`

### Tests (Required)

- `cargo test --workspace --all-features`

### Additional Strict Gates (Recommended)

- `cargo deny check` — Licenses, advisories, bans, sources
- `cargo audit` — Security advisories (requires `cargo install cargo-audit`)
- `cargo +nightly udeps --workspace --all-targets` — Unused dependencies (requires nightly)

## Architecture

```
main.rs          CLI parsing → GlobalConfig → AppState → UI loop
    ↓
app.rs           AppState manages targets, profiles, metrics stores, UI state
    ↓
runtime.rs       Spawns per-profile worker threads via crossbeam channels
    ↓
probe_engine.rs  ProbeClient executes HTTP probes using libcurl, extracts timing/TCP info
    ↓
metrics_aggregate.rs  MetricsStore holds samples, computes windowed stats (HDR histogram)
    ↓
ui.rs            Ratatui-based TUI rendering and keyboard input handling
```

**Key data types:**
- `config.rs`: GlobalConfig, TargetConfig, ProfileConfig (HTTP version, TLS, connection reuse)
- `probe.rs`: ProbeSample, ProbeError, TcpInfoSnapshot
- `metrics.rs`: MetricKind (18 variants), StatsSummary

**Threading model:** Main thread owns AppState and runs the UI loop. Each target/profile combination spawns a worker thread that sends `ProbeSample` results back via unbounded crossbeam channel. Control messages (UpdateTarget, UpdateProfile, Pause, Stop) flow from main to workers.

**Platform note:** TCP info extraction (`libc::TCP_INFO` socket option) is Linux-only; returns `None` on macOS/Windows.

## Code Patterns

- Use message-passing via channels; avoid shared mutable state
- New metrics: add variant to `MetricKind` in `metrics.rs`, implement in `sample_metric()` in `metrics_aggregate.rs`
- New probe features: extend `ProfileConfig`, modify `ProbeClient::probe()`, add control message if runtime config changes needed
- Errors are typed via `ProbeErrorKind` (16 variants); map curl errors to domain errors

# Development Guidelines

Follow the rules below. Decompose the system into **components**, keep each responsibility **simple and focused**, and ensure **every feature is implemented end-to-end**.

For the requirements below, **verify each item one by one**. You must **understand, validate, analyze, design, plan, implement**, and **fix all defects**. Use **sequential thinking** for planning.

## Design Standards (Mandatory)

### SOLID (Required)

- **S (SRP) — Single Responsibility:** Each module/class/function should do one thing. It should have only one reason to change.
- **O (OCP) — Open/Closed:** Add new behavior via extension (interfaces/strategies/dependency injection) and avoid modifying core logic that could introduce regressions.
- **L (LSP) — Liskov Substitution:** Subtypes must be substitutable for base types without changing the contract semantics (inputs/outputs/exceptions).
- **I (ISP) — Interface Segregation:** Prefer small, focused interfaces. Avoid "fat interfaces" that force consumers to depend on methods they do not use.
- **D (DIP) — Dependency Inversion:** High-level policy depends on abstractions. Inject IO/external systems via interfaces to enable testing and replacement.

### Clean Code (Required)

- Use specific, readable, searchable names; avoid abbreviations and vague terms (e.g., `data`, `info`, `tmp`).
- Keep functions short and single-purpose; avoid deep nesting (refactor if nesting exceeds ~2 levels).
- Design APIs around intent; call sites should read like natural language.
- Avoid duplication (DRY) but also avoid premature/over-abstraction; abstractions must reduce future change cost.
- Comments should explain **why**, not repeat **what**. If a comment explains what the code does, the code should be made clearer.

### Architecture & Code Structure

- Clear layering: **Domain (business logic) must not directly depend on Infrastructure (DB/HTTP/Queue)**. Use interfaces to isolate dependencies.
- **No business logic in Controllers/Handlers:** Handlers only handle input validation/authentication/authorization/transformation and invoke use-cases.
- Clear module boundaries: cross-module access must go through public APIs; do not rely on internal implementation details.

### Error Handling & Observability

- All errors must be **traceable**: include specific error codes/messages and required context (request id / user id / correlation id).
- Separate errors by layer: **Domain errors vs. Infrastructure errors** must not be mixed.

### Testing (Required)

- Every new/changed behavior must include tests covering at least:
  - Primary success path
  - Critical failure paths (insufficient permissions, invalid input, external dependency failures)
  - Edge cases (null/empty values, max length, time boundaries, concurrency)
- Unit tests must not depend on real external systems (DB/HTTP). Use stubs/mocks/test doubles.
- Bug fixes must include a **failing test first**, then the fix (to prevent regressions).

## Maintainability & Consistency (Required)

### Formatting & Static Analysis

- Must enable: formatter, linter, and type checking (use them wherever applicable).

## Security Standards

### Sensitive Data & Credentials

- Credentials/keys/tokens **must not be committed to source code or the repository**.

# Git Commit Guidelines

All new commits to this repository must strictly adhere to the following rules. This ensures a clean history and supports automated changelog generation.

## 1. Core Format (Conventional Commits)

The commit message header must follow this format:

```text
<type>(<scope>): <imperative summary>
```

### Allowed Types

You must use one of the following types:

* **feat**: A new feature (user-facing capability, new module/endpoint).
* **fix**: A bug fix.
* **docs**: Documentation only changes.
* **refactor**: A code change that neither fixes a bug nor adds a feature.
* **perf**: A code change that improves performance.
* **test**: Adding missing tests or correcting existing tests.
* **build**: Changes that affect the build system or external dependencies.
* **ci**: Changes to CI configuration files and scripts.
* **chore**: Other changes that do not modify src or test files.
* **style**: Changes that do not affect the meaning of the code (white-space, formatting, etc.).
* **revert**: Reverting a previous commit.

### Scope

* Specify the top-level folder or subsystem changed (e.g., `api`, `ui`, `auth`, `deps`, `config`).
* If the change affects the entire system, the scope can be omitted, but it is preferred to include it.

### Title Guidelines

* **Length**: Preferably 50 characters, max 72 characters.
* **Mood**: Use the **imperative mood** (e.g., "add" not "added", "fix" not "fixed").
* **Punctuation**: Do not end the title with a period.
* **Language**: Must be in **English**.

---

## 2. Message Body

A detailed body is **mandatory** for all non-trivial commits.

### Content Requirements

1. **Based on Diffs**: The description must reflect the actual file changes. Do not guess.
2. **Bulleted List**: Use hyphens (`-`) or asterisks (`*`) for formatting.
3. **Structure**:
    * **What**: detailed list of changes (files, functions, configs).
    * **Why**: context or reason for the change (if inferable).

4. **Tests**: You must include a `Tests:` line indicating how the change was verified.
5. **Dependencies**: If dependencies are changed, state the package name and the action (bump/pin/remove).

6. Remove "Co-Authored-By: Claude <noreply@anthropic.com>"

---

## 3. Breaking Changes

If the commit introduces a breaking change (incompatible API change):

1. **Header**: Add an exclamation mark `!` after the type/scope (e.g., `feat(api)!: ...`).
2. **Footer**: Include a `BREAKING CHANGE:` section at the bottom of the body describing the change and migration path.

---

## 4. Author Identity

* **Name**: Your `user.name` config for this repository must be set to **DennySORA**.
* **Email**: Use your actual email address.

---

## 5. Examples

### Feature (feat)

```text
feat(auth): implement JWT token refresh mechanism

- What: Added new endpoint /api/v1/refresh to handle token renewal.
- What: Updated AuthMiddleware to check for expiration before rejecting requests.
- Why: To improve user experience by preventing forced logouts when access tokens expire.
- Dependency: Bumped jsonwebtoken from 8.5.1 to 9.0.0.

Tests: Verified with Postman and ran unit tests (npm test:auth).
```

### Bug Fix (fix)

```text
fix(ui): resolve button misalignment on mobile devices

- What: Adjusted CSS flexbox properties in ButtonComponent.vue.
- What: Removed fixed width constraints causing overflow on screens < 375px.
- Why: Users on iPhone SE were unable to click the "Submit" button.

Tests: Tested on Chrome DevTools mobile view.
```

### Breaking Change

```text
refactor(database)!: drop legacy user columns

- What: Removed 'age' and 'gender' columns from the Users table.
- Why: These fields are no longer collected due to new privacy policy compliance.

BREAKING CHANGE: The 'age' and 'gender' fields are removed from the User schema. Any code relying on these fields will fail.

Tests: Ran db:migrate and existing user tests.
```

---

## 6. Pre-commit Checklist

Before running `git commit`, verify:

* [ ] The working tree is clean (no unintended files).
* [ ] The message is written in **English**.
* [ ] The title follows `type(scope): imperative summary`.
* [ ] The body includes specific "What" and "Why" bullets.
* [ ] The body includes a "Tests" line.
* [ ] If breaking, `!` and `BREAKING CHANGE:` are used.
