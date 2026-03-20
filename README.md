# v8-test-runner

Rust CLI for local 1C development workflows.

## Build

Current `build` support is limited to `builder=DESIGNER` and `format=DESIGNER`.

- `v8-test-runner build` runs change detection and loads only affected `source-set` entries.
- `v8-test-runner build --full-rebuild` bypasses change detection and forces full load for every Designer `source-set`.
- Execution order is always the main `CONFIGURATION` first, then extensions in config order.
- Build is intentionally non-atomic across `source-set`: if a later step fails, earlier successful steps remain applied.

Optional YAML settings:

```yaml
build:
  partialLoadThreshold: 20
```

- `partialLoadThreshold` controls when partial load falls back to full load.
- `Configuration.xml` changes and deletions always force a full load.

## Dump

Current `dump` support is limited to `format=DESIGNER` with either `builder=DESIGNER` or
`builder=IBCMD`.

- `v8-test-runner dump --mode full` performs a full export.
- `v8-test-runner dump --mode incremental` exports only changes according to backend semantics.
- `v8-test-runner dump --mode partial --object <OBJECT>` performs object-scoped partial dump for
  `builder=DESIGNER`.
- `v8-test-runner dump --mode partial --object <OBJECT>` on `builder=IBCMD` degrades to
  incremental export for the resolved configuration/extension target and returns a warning while
  keeping the requested mode as `PARTIAL`.
- `partial` requires at least one `--object`; blank values and control characters are rejected.

## Tests

Current `test` support is limited to `builder=DESIGNER` and `format=DESIGNER`.

- `v8-test-runner test all` always runs `build` first, then launches YaXUnit via `1cv8c`.
- `v8-test-runner test module <MODULE_NAME>` does the same, but writes `filter.modules = ["<MODULE_NAME>"]` into the temporary YaXUnit config.
- `v8-test-runner test --full ...` keeps passed test cases and full stack traces.
- Compact mode hides passed cases and truncates stack traces.
- If the run fails or the JUnit report cannot be parsed, sanitized retained artifacts stay under `workPath/temp/yaxunit/runs/<run-id>/`.
- YaXUnit must already be installed and callable from the target infobase.

Optional YAML settings:

```yaml
tests:
  execution_timeout_seconds: 300
```

- `execution_timeout_seconds` controls the hard timeout for the Enterprise test run.

## Internal Boundary

The CLI now uses a transport-neutral use-case contract internally:

- `cli::execute` maps parsed CLI args into request DTOs and renders the final text/json output.
- `use_cases` no longer depend on `clap`, `Presenter`, or `Envelope`.
- `mcp` now adds a separate internal service boundary with MCP-specific request/response DTOs and explicit business-vs-internal failure split.
- MCP request normalization is isolated in the service layer instead of leaking into use cases.
- MCP now fixes these transport-level semantics explicitly:
  - `dump_config(mode=null|blank)` resolves to `INCREMENTAL` starting on `2026-03-20`.
  - `launch_app` accepts the Kotlin alias set plus the already supported `thin` / `thick` aliases, with trim + lowercase normalization.
  - `allExtensions` is treated as tri-state in MCP request mapping, with the default inferred from whether `extension` is present.
  - `checkUseSynchronousCalls` and `checkUseModality` are rejected at the MCP boundary when `extendedModulesCheck=false`.
- This remains preparatory work for the upcoming MCP adapters without changing the public CLI surface.

## MCP Configuration Prep

The config model now reserves the MCP transport knobs that upcoming stdio/HTTP stages will consume.

Optional YAML settings:

```yaml
mcp:
  http:
    bind_address: 127.0.0.1:3000
    path: /mcp
    stateful_sessions: true
    max_sessions: 64
    idle_ttl_secs: 900
  execution:
    max_concurrent_calls: 1
    shutdown_grace_period_secs: 30

tools:
  edt_cli:
    startup_timeout_ms: 300000
    command_timeout_ms: 300000
```

- `mcp.http.*` and `mcp.execution.*` are parsed and validated now, but they are not wired into a live transport yet.
- `tools.edt_cli.startup_timeout_ms` and `tools.edt_cli.command_timeout_ms` default to `300000` ms and also accept legacy `edt-cli` / kebab-case aliases for compatibility.
