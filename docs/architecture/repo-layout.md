# Repository Layout

This repository has two production applications and a small set of internal support materials.

## Top-level boundaries

- `src/`: React frontend for the desktop UI.
- `src-tauri/`: Rust backend for the desktop app.
- `verify-vps/`: optional SMTP verification service.
- `docs/`: human-facing documentation, grouped by intent.
- `scripts/`: internal helper scripts that are not part of the shipped app.

## Documentation folders

- `docs/architecture/`: structure notes, codebase boundaries, and repo conventions.
- `docs/operations/`: workflows such as verification review/evaluation.
- `docs/plans/`: implementation plans and project planning notes.
- `docs/prompts/`: prompt/spec artifacts that informed implementation.

## Script folders

- `scripts/eval/`: reusable utility scripts referenced by docs.
- `scripts/experiments/`: one-off simulations and exploratory scripts.
- `scripts/patches/`: internal patch/rewrite helpers used during development.

## Root folder rules

- Keep runtime entrypoints and package manifests at the root.
- Do not leave one-off patch scripts or planning documents at the root.
- Avoid backup files like `file 2.rs` in tracked source directories.
- Keep deploy keys and other secrets out of git history and out of normal source folders.
