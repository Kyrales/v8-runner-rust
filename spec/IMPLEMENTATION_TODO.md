# Active TODO For `v8-runner`

This file tracks open implementation work only.

## Current Status

- Open tasks as of `2026-05-02`:
  - `T21`: implement local config overlay from [ADR-0021](decisions/0021-lokalnyy-overlay-config.md).

## Open Tasks

### T21: Implement local config overlay

Status: planned

Scope:

- Add automatic `v8project.local.yaml` overlay loading next to the primary `v8project.yaml`.
- Make `basePath` optional at YAML boundary with default equal to the primary config directory.
- Forbid local overlay from changing `source-set`, `format`, or `builder`.
- Keep precedence `project config -> local overlay -> CLI overrides`.

Acceptance:

- `cargo test --locked config` covers overlay merge, forbidden local keys and `basePath` default.
- `docs/CONFIGURATION.md`, examples and architecture invariants are synchronized with ADR-0021.

## Rules

- Keep this file short and active-only.
- Move closed task detail into `spec/archive/`.
- If a task changes a public or architectural contract, update the ADR and active docs layer
  before implementation.
- Promote only immediately executable work here; keep broader ADR reconciliation in
  `ADR_DERIVED_BACKLOG.md`.

## Historical Records

- [spec/archive/IMPLEMENTATION_TODO_2026-04-30.md](archive/IMPLEMENTATION_TODO_2026-04-30.md):
  closed task ledger moved out of the active file.
- [spec/archive/MCP_IMPLEMENTATION_PLAN_2026-03-21.md](archive/MCP_IMPLEMENTATION_PLAN_2026-03-21.md):
  closed MCP rollout history.
- [spec/archive/completed-tasks-t22.md](archive/completed-tasks-t22.md):
  closed universal tool extension preparation task.
