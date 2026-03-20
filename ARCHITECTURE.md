# Architecture

## Overview

`v8-test-runner` is a Rust CLI for orchestrating local 1C platform operations. The current codebase is organized into eight main layers:

1. `cli` parses arguments, maps them into transport-neutral requests, and owns command-level text/json rendering.
2. `config` loads and validates YAML configuration.
3. `domain` defines structured result types for commands plus shared execution step structs.
4. `use_cases` owns transport-neutral requests, `ExecutionContext`, structured failures, and business orchestration.
5. `mcp` is the new MCP-facing service boundary: it maps raw tool inputs into use-case requests and returns MCP-specific DTOs plus structured business/internal failures.
6. `platform` contains process execution, utility discovery, connection argument building, and low-level 1C adapters.
7. `output` contains CLI presentation primitives such as `Presenter` and `Envelope`.
8. `change_detection`, `parsers`, and `support` provide shared subsystems and utilities.

## Current Platform Layer

The platform layer is intentionally split so responsibilities do not bleed into use cases:

- `platform::process` defines `ProcessRunner`, `ProcessExecutor`, `ProcessRequest`, `ProcessResult`, and `SpawnResult`.
- `platform::locator` resolves concrete executables (`1cv8`, `1cv8c`, `ibcmd`, `1cedtcli`) and caches results per `Locator` instance.
- `platform::connection` builds reusable V8 connection/auth arguments from the raw config connection string.
- `platform::utilities` is the current facade used by use cases. It owns the stateful `Locator` and exposes the standard execution path.
- `platform::designer` is the low-level batch DSL for `1cv8 DESIGNER`, returning `PlatformCommandResult` so `/Out` logs stay separate from runner-captured stdio.
- `platform::ibcmd` is the low-level DSL for `ibcmd`, returning `PlatformCommandResult` with stdout/stderr diagnostics (no `/Out` log).

This boundary is designed so Wave 2 can add an EDT-specific interactive runner without replacing the locator API or the standard execution path.

## Command Boundary

The CLI/runtime boundary is now split explicitly:

- `app.rs` owns bootstrap concerns only: config loading, logging setup, log cleanup, and top-level error envelopes for pre-command failures.
- `cli::execute` converts `clap` args into transport-neutral request structs and renders command success/failure output.
- `use_cases::{request,context,result}` define the transport-neutral contract that both CLI and future MCP adapters can consume.
- `use_cases/*.rs` no longer depend on `clap`, `Presenter`, or `Envelope`.

This keeps current CLI behavior intact while reserving a stable internal API for MCP stdio/HTTP adapters.

## Configuration Surface

The typed config model already reserves the MCP knobs needed by the upcoming transports:

- `mcp.http` defines listener/session defaults (`bind_address`, `path`, `stateful_sessions`, `max_sessions`, `idle_ttl_secs`).
- `mcp.execution` defines runtime guardrails (`max_concurrent_calls`, `shutdown_grace_period_secs`).
- `tools.edt_cli` now also carries `startup_timeout_ms` and `command_timeout_ms`, which are validated even before the interactive EDT executor exists.

These settings are intentionally inert for now: Stage 1 only loads, validates, and documents them so later MCP stdio/HTTP work can wire them in without reshaping `AppConfig`.

## MCP Boundary

The future MCP adapter no longer needs to talk to `cli::execute` or to reuse domain serialization directly.

- `mcp::request` defines raw tool-facing request DTOs.
- `mcp::service::McpService` maps those requests into `use_cases::request::*` and attaches per-call MCP transport metadata.
- `mcp::response` defines MCP-specific response DTOs, including nested step/test/issue structs that are decoupled from domain serialization details.
- `mcp::error` splits failures into `McpBusinessFailure<T>` for structured tool responses and `McpInternalError` for adapter/runtime misuse that must not be surfaced as business payloads.
- MCP normalization is finalized in the service layer: dump-mode defaulting, launch alias mapping, `allExtensions` tri-state inference, and MCP-only pre-validation for syntax flag dependencies all live there instead of leaking into transport-neutral use cases.

## Backend Dispatch

`build` and `dump` use cases dispatch by `builder`:

- `builder=DESIGNER` uses the existing `DesignerDsl`.
- `builder=IBCMD` uses `IbcmdDsl` with `config import/apply` for build and `config export` for dump.

Constraints to keep in mind:

- IBCMD requires file-based infobase connections.
- `builder=DESIGNER` supports object-level partial dump via `/DumpConfigToFiles -partial -listFile`.
- `builder=IBCMD` does not support object-scoped partial dump directly; `PARTIAL` degrades to
  incremental export for the resolved target and returns a warning while preserving the requested
  mode in the result payload.

## Output Flow

Use cases now return transport-neutral payloads or structured failures.

- `cli::execute` converts successful command payloads into `Envelope<T>` for JSON mode.
- `cli::execute` preserves command-specific text formatting for build, test, dump, syntax, and launch.
- Failure payload emission is also decided at the adapter boundary, which keeps `launch --output json` failure semantics unchanged while allowing other commands to keep structured JSON failures.
- `mcp::service` returns MCP-specific DTOs and never reuses CLI `Envelope` or presenter logic.

## Working Directories

`workPath` is the root for runtime artifacts:

- `workPath/logs/platform/` stores platform log files.
- `workPath/temp/partial-lists/` stores partial load and partial dump list files.
- `workPath/temp/yaxunit/` stores temporary YaXUnit config files.
- `workPath/hash-storages/` remains reserved for change detection state.
- `workPath/<sourceSetName>/` is reserved for the future EDT export flow and is not created yet.
