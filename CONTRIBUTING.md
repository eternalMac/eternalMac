# Contributing

Thanks for contributing to `eternalMac`.

This project is still in an early MVP phase, so the main priority is keeping the implementation aligned with the actual product goal: a simple, reliable Mac Mini devserver setup for a laptop.

## Ground Rules

- Keep the project macOS-only unless there is an explicit decision to expand scope.
- Keep the MVP Homebrew-first. Do not add alternate install paths unless they are intentionally accepted into scope.
- Prefer small, reviewable changes over large speculative refactors.
- Keep documentation lean and current. Do not add broad future-facing docs just to fill space.

## Development Setup

Prerequisites:

- Rust toolchain
- macOS

Clone the repo and build:

```bash
cargo build
```

## Before Opening a PR

Run the full test suite:

```bash
cargo test
```

Run the smoke script:

```bash
bash scripts/smoke/bootstrap.sh
```

## Change Expectations

If you change behavior:

- add or update tests
- keep command help, status, and doctor output accurate
- update the relevant docs when user-facing behavior changes

If you change setup, daemon, session, or sync behavior:

- prefer integration-style tests in `tests/`
- cover failure paths, not just happy paths
- avoid inventing state that the underlying tools did not actually provide

## Commit Style

Prefer concise conventional-style commit messages such as:

- `fix: ...`
- `feat: ...`
- `docs: ...`
- `test: ...`

## Project Layout

- `src/commands/` contains CLI command handlers
- `src/setup/` contains server/client setup flows
- `src/daemon/` contains periodic background reconciliation
- `src/tooling/` contains wrappers and parsers for external tools
- `tests/` contains integration-focused behavior tests
- `scripts/smoke/` contains lightweight smoke checks

## Scope Discipline

Good contributions for this stage:

- correctness fixes
- setup reliability improvements
- better health reporting
- tighter tests around ET, `tmux`, Mutagen, Tailscale, and `launchd` interactions
- lean documentation updates that match the codebase

Changes that should usually start with discussion first:

- non-macOS support
- non-Homebrew installation paths
- major architecture rewrites
- broad UX redesigns
- large post-MVP feature additions
