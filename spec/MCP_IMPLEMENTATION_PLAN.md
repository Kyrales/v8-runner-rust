# Поэтапный план реализации MCP

## Summary

- Добавить в текущий бинарь отдельные режимы `v8-test-runner mcp serve stdio` и `v8-test-runner mcp serve http`.
- Реализовать MCP поверх `rmcp`, сохранив 8 tool-методов и входную семантику из `kotlin-example/.../McpServer.kt`.
- Не переиспользовать CLI `Envelope` и `clap`-типы в MCP. Ввести transport-neutral request/service слой и отдельные MCP result structs.
- Включить в MCP-этап недостающий функциональный gap: `dump_config` в режиме `PARTIAL`.
- Для MCP path добавить shared EDT interactive session как bounded actor; CLI path оставить на текущем one-shot поведении.

## Stage 1. Foundation And Contract Layer

- [x] 2026-03-20: Ввести transport-neutral request/result слой и `ExecutionContext`, чтобы use case-ы не зависели от `clap`, `Presenter` и `Envelope`.
  - Добавлены transport-neutral request DTO, `ExecutionContext`, shared use-case failure contract и CLI adapter boundary.
  - CLI rendering сохранен в `cli::execute`, а bootstrap error rendering оставлен в `app.rs`.
- [x] 2026-03-20: Добавить MCP-facing service layer с явными structured business failures.
  - Добавлен внутренний модуль `src/mcp` с request/response DTO, `McpCallContext`, `McpUseCasePort` и `McpService`.
  - MCP boundary теперь возвращает либо typed success payload, либо `McpBusinessFailure<T>` с machine-readable error code, либо отдельный `McpInternalError`.
  - Raw MCP defaults и alias normalization изолированы в service-layer mapper-ах с явными `TODO(mcp-normalization-stage)`.
  - MCP response structs отвязаны от domain nested DTO, чтобы будущий transport adapter не зависел от внутренних сериализационных деталей.
- [x] 2026-03-20: Расширить config для MCP transport defaults и EDT timeout knobs.
  - Добавлены typed config-секции `mcp.http` и `mcp.execution` с defaults `127.0.0.1:3000`, `/mcp`, `stateful_sessions=true`, `max_sessions=64`, `idle_ttl_secs=900`, `max_concurrent_calls=1`, `shutdown_grace_period_secs=30`.
  - `tools.edt_cli` теперь поддерживает `startup_timeout_ms` и `command_timeout_ms` с default `300_000 ms`; сохранена совместимость с `startup-timeout-ms` и `command-timeout-ms`.
  - Добавлена pre-runtime validation для bind address, MCP HTTP path, positive session/concurrency/grace/timeout limits и обновлены example/docs.
- [x] 2026-03-20: Реализовать contract normalization в MCP service layer.
  - Зафиксирован tri-state для `allExtensions`: blank `extension` трактуется как отсутствие значения, default зависит от наличия `extension`, явный `allExtensions=true` сохраняется как Kotlin-compatible behavior.
  - Добавлен pre-validation для `checkUseSynchronousCalls` и `checkUseModality`: при `extendedModulesCheck=false` MCP boundary возвращает `InvalidArgument` до вызова use case.
  - Зафиксирован alias set для `launch_app`, включая Kotlin aliases и ранее принятые `thin` / `thick`; trim + lowercase normalization сохранены.
  - Осознанное product-расхождение теперь задокументировано и покрыто тестами: `dump_config(mode=null|blank)` в MCP трактуется как `INCREMENTAL`.
- [x] 2026-03-20: Закрыть functional gap: `dump_config(PARTIAL)` для `DESIGNER`; для `IBCMD` сделать degraded fallback с warning и сохранением requested mode `PARTIAL`.
  - `DESIGNER` partial dump теперь использует `DumpConfigToFiles -partial -listFile ... -updateConfigDumpInfo` для configuration и extension targets.
  - `IBCMD` partial dump теперь валидирует тот же object list contract, затем деградирует в incremental export (`--sync`) по resolved target и возвращает warning, сохраняя `mode=PARTIAL` в use-case/MCP payload.
  - Добавлены regression tests на object normalization, temp-list cleanup, missing-target creation и `IBCMD` success/failure fallback semantics.

## Stage 2. MCP stdio MVP

- Добавить `v8-test-runner mcp serve stdio`.
- Поднять `rmcp` tool server только с tools-capability, без resources и prompts.
- Опубликовать все 8 tools через MCP adapter поверх нового service layer.
- Зафиксировать правило `stdout reserved for MCP`:
  - никакого tracing/stdout логирования
  - panic hook в stderr
  - subprocess stdout/stderr только captured или null
- Добавить bounded execution через semaphore и per-call timeout/cancel semantics для MCP path.
- На этом этапе EDT tools могут работать через текущий one-shot path, но уже через новый MCP adapter.

## Stage 3. Shared EDT Session For MCP

- Реализовать `InteractiveProcessExecutor` для `1cedtcli`.
- Добавить `EdtSessionManager` как single shared actor для MCP mode:
  - single-flight startup
  - FIFO queue
  - bounded admission
  - per-command deadline
  - restart-on-timeout или hang
  - explicit error model для queued и cancelled requests
- Перед каждой EDT-командой делать baseline/reset check, чтобы не было межсессионной утечки интерактивного состояния.
- Во время shutdown или restart queued jobs отменять сразу единым business error.
- MCP path переключить на shared EDT actor; CLI path оставить без изменений.

## Stage 4. HTTP Transport

- Добавить `v8-test-runner mcp serve http`.
- Поднять `axum` + `rmcp` streamable HTTP transport.
- Явно зафиксировать HTTP defaults:
  - `bind_address=127.0.0.1:3000`
  - `path=/mcp`
  - `stateful_sessions=true`
  - `max_sessions=64`
  - `idle_ttl_secs=900`
- Проверить session semantics:
  - reuse `Mcp-Session-Id`
  - sticky shared app context
  - deterministic behavior for missing или expired session
- Shared EDT actor используется и для HTTP, без создания отдельных EDT processes per MCP session.

## Stage 5. Hardening And Docs

- Добавить stress и regression suite:
  - contract tests на `list_tools`
  - integration tests на все 8 tools
  - stdio cleanliness tests
  - HTTP session lifecycle tests
  - EDT timeout/restart/isolation tests
  - `dump PARTIAL` matrix for `DESIGNER` and `IBCMD`
- Добавить runtime metrics и tracing:
  - semaphore wait time
  - EDT queue depth
  - restart count
  - shutdown drain stats
- Оформить migration note для расхождения `dump_config(mode=null)` с текущим Kotlin code path.
- Сохранять этот документ как основной staged plan для MCP-работ.

## Public Changes

- Новый CLI surface:
  - `v8-test-runner mcp serve stdio`
  - `v8-test-runner mcp serve http`
- Новые config keys в `mcp.*` и `tools.edt_cli.*`.
- Новый внутренний transport-neutral API между CLI/MCP и use case-слоем.
- Новый shared EDT actor только для MCP path.

## Test Plan

- Stage 1: unit tests на use-case boundary, CLI request mapping, bootstrap error parity, launch JSON failure parity, normalization, validation, alias matrix, partial dump semantics.
- Stage 2: MCP stdio integration tests, protocol-clean stdout tests, business-vs-transport error boundary tests.
- Stage 3: EDT actor tests, timeout/restart tests, queue cancellation tests, A->B isolation tests.
- Stage 4: HTTP session tests, TTL/eviction tests, stateful behavior tests.
- Stage 5: full regression and stress suite.

## Assumptions

- Сохраняется Kotlin tool surface, но не byte-for-byte DTO parity.
- `dump_config(mode=null -> INCREMENTAL)` остается осознанным продуктовым решением, а не случайной несовместимостью.
- HTTP transport локально-ориентирован; remote auth layer в этот этап не входит.
- Если baseline test suite остается частично красным из-за уже существующего unrelated дефекта, MCP acceptance не блокируется на нем.
