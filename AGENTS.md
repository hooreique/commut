# Agent Development Guide

A file for [guiding coding agents](https://agents.md/).

`README.md` covers what commut is, installation, and end-user setup.
This file only covers repository-specific guidance for agents working in this tree.

## Working Agreement

- Start from `nix develop` so `cargo`, `pnpm`, `rustfmt`, and related tools are available.
- Keep `README.md` user-facing. Put developer workflow, validation commands, and repo-specific editing guidance here instead of duplicating them there.
- Prefer the narrowest validation that matches the change you made.
- Do not introduce a new formatting tool unless the user asks for it.

## Repository Map

- `flake.nix`: shared project entrypoint and package definitions
- `backend/`: Rust backend service
- `frontend/`: TypeScript web client and static assets
- `installer/`: Nix and shell deployment tooling
- `.github/`: GitHub automation

## Validation Commands

- **Enter dev shell:** `nix develop`
- **Build (all):** `nix build .#commut-client .#commut .#commut-installer`
- **Build (backend):** `cargo build --manifest-path backend/Cargo.toml`
- **Test (backend):** `cargo test --manifest-path backend/Cargo.toml`
  - Prefer to run targeted tests with `cargo test --manifest-path backend/Cargo.toml <test_name>` because some integration tests are slower.
- **Install frontend dependencies:** `pnpm --dir frontend install`
- **Build (frontend assets, after installing frontend dependencies):** `pnpm --dir frontend run prepare`
- **Typecheck (frontend, after installing frontend dependencies):** `pnpm --dir frontend run typecheck`
- **Format (Rust):** `cargo fmt --manifest-path backend/Cargo.toml`
- **Format (frontend):** no dedicated formatter is configured; keep TypeScript/CSS style consistent with surrounding files.
- **Check Nix files:** `nix flake check`

## Change-Specific Guidance

- The frontend commands in this file expect `frontend/node_modules` to exist. If it does not, run `pnpm --dir frontend install` first.
- Prefer focused backend test runs when changing a narrow area.
- When changing the frontend, run `pnpm --dir frontend run typecheck` and rebuild assets with `pnpm --dir frontend run prepare` after installing frontend dependencies.
- When changing packaging or installer behavior, validate with `nix build` for the affected package.
