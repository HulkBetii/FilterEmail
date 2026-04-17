# Module Map

## Frontend

- `src/App.tsx`: composition shell only.
- `src/hooks/use-processing-controller.ts`: app orchestration, event wiring, history updates, notifications, and job execution.
- `src/lib/app-storage.ts`: localStorage boundary with legacy-compatible keys.
- `src/lib/app-state.ts`: shared frontend types, constants, and normalization helpers.
- `src/components/`: presentational UI blocks such as header, dashboard, settings, results, history, and verify cards.

Dependency direction:

- shell -> hook
- hook -> storage + shared state helpers
- shell -> presentational components
- presentational components -> shared types only

## Tauri backend

- `src-tauri/src/main.rs`: command wiring and app bootstrap.
- `src-tauri/src/processor/mod.rs`: public processor facade and top-level orchestration.
- `src-tauri/src/processor/types.rs`: stable payload/status types and internal state structs.
- `src-tauri/src/processor/cache.rs`: SQLite-backed DNS/SMTP cache logic.
- `src-tauri/src/processor/classify.rs`: domain normalization and classification helpers.
- `src-tauri/src/processor/dns.rs`: domain collection, resolver setup, and DNS scan flow.
- `src-tauri/src/processor/pipeline.rs`: second pass routing and SMTP spool/batch execution.
- `src-tauri/src/processor/output.rs`: writers, file naming, CSV output, and flush logic.
- `src-tauri/src/processor/payload.rs`: progress math and payload construction.

Dependency direction:

- `main.rs` -> `processor`
- `processor/mod.rs` -> internal processor submodules
- `dns` / `pipeline` -> `types`, `payload`, `output`, `classify`, `cache`
- `output` and `cache` do not depend on `dns` or `pipeline`

This keeps `process_file_core` as the stable entrypoint while allowing internal modules to evolve independently.
