# AGENTS.md

A file for [guiding coding agents](https://agents.md/).

`README.md` covers what commut is, installation, and end-user setup.
This file only covers repository-specific guidance for agents working in this tree.

## Working Agreement

- Start from `nix develop` so `cargo`, `pnpm`, `rustfmt`, and related tools are available.
- Prefer the narrowest validation that matches the change you made.
- Keep `README.md` user-facing. Put developer workflow, validation commands, and repo-specific editing guidance here instead of duplicating them there.
- Do not introduce a new formatting tool unless the user asks for it.
- When working under `frontend/`, also follow `frontend/AGENTS.md`.

## Protocol Ownership

- Keep the backend crate standalone: backend code, backend tests, and `backend/package.nix` must not reference the `frontend/` tree, frontend build outputs, or client static asset roots.
- Compose backend and frontend artifacts only outside the backend boundary, such as in the top-level flake or installer packaging.
- Treat the backend wire spec in `backend/src/contract.rs` as the source of truth for browser protocol values.
- When changing handshake labels, WebSocket message types, close codes, or default dimensions, update that backend spec and its tests first; mirror the change in the frontend after that.
- Do not make backend tests read frontend source files to infer protocol behavior.

## Validation Commands

- **Enter dev shell:** `nix develop`
- **Run dev server:** `nix run .#dev-server`
- **Build (all):** `nix build .#commut .#commut-installer`
- **Build (backend):** `nix develop --command -- cargo build --manifest-path backend/Cargo.toml`
- **Lint (backend):** `nix develop --command -- cargo clippy --manifest-path backend/Cargo.toml -- -D warnings`
- **Test (backend):** `nix develop --command -- cargo test --manifest-path backend/Cargo.toml`
  - Prefer to run targeted tests with `nix develop --command -- cargo test --manifest-path backend/Cargo.toml <test_name>` because some integration tests are slower.
- **Install frontend dependencies:** `nix develop --command -- pnpm --dir frontend install --frozen-lockfile`
- **Test (frontend pure modules, after installing frontend dependencies):** `nix develop --command -- pnpm --dir frontend run test`
- **Test (frontend build scripts, after installing frontend dependencies):** `nix develop --command -- pnpm --dir frontend run build-test`
- **Build (frontend assets, after installing frontend dependencies):** `nix develop --command -- pnpm --dir frontend run build`
- **Typecheck (frontend, after installing frontend dependencies):** `nix develop --command -- pnpm --dir frontend run typecheck`
- **Format (Rust):** `nix develop --command -- cargo fmt --manifest-path backend/Cargo.toml`
- **Format (frontend):** no dedicated formatter is configured; keep TypeScript/CSS style consistent with surrounding files.
- **Check Nix files:** `nix flake check`

## Change-Specific Guidance

- The frontend commands in this file expect `frontend/node_modules` to exist. If it does not, run `nix develop --command -- pnpm --dir frontend install --frozen-lockfile` first.
- Prefer focused backend test runs when changing a narrow area.
- When changing Caddy routing, keep `installer/Caddyfile.template` and `dev-server.nix` in sync.
- When changing the frontend, run the relevant frontend tests from the validation list, run `nix develop --command -- pnpm --dir frontend run typecheck`, and rebuild assets with `nix develop --command -- pnpm --dir frontend run build` after installing frontend dependencies.
- When changing packaging or installer behavior, validate with `nix build` for the affected package.

## Repository Map

- `flake.nix`: shared project entrypoint and package definitions
- `dev-server.nix`: local development server app for `nix run .#dev-server`
- `backend/`: Rust backend service
- `frontend/`: TypeScript web client and static assets
- `installer/`: Nix and shell deployment tooling
- `.github/`: GitHub automation
