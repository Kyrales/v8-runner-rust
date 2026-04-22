# Активный TODO реализации `v8-runner`

Этот файл является коротким рабочим source of truth для следующих задач.

Исторический TODO до очистки сохранен в [spec/archive/IMPLEMENTATION_TODO_2026-04-21.md](archive/IMPLEMENTATION_TODO_2026-04-21.md).
Подробная декомпозиция ADR-задач находится в [spec/ADR_DERIVED_BACKLOG.md](ADR_DERIVED_BACKLOG.md).
Закрытый staged record MCP rollout остается в [spec/MCP_IMPLEMENTATION_PLAN.md](MCP_IMPLEMENTATION_PLAN.md) и не используется как активный backlog без явного запроса.

## Правила ведения

- Держать здесь только открытые задачи или короткие ссылки на активные детализации.
- После закрытия задачи отмечать ее `[x]` только на время текущего delivery loop, затем переносить детали в профильный archive/spec/history-документ.
- Если задача меняет архитектурный контракт, сначала обновлять или добавлять ADR, затем синхронизировать `docs/architecture/invariants.md`, arc42 и публичную документацию.
- Для реализации брать следующий конкретный пункт сверху вниз, если пользователь не указал другой приоритет.

## P0

- [x] `ADR-TASK-011`: Согласовать `ADR-0010` с backlog и целевой CLI output policy. Выполнено `2026-04-22`: зафиксирована единая output policy без отдельной audience/profile-оси; единственная public ось — `--output text|json`, JSON не меняется.
- [x] `ADR-TASK-002`: Закрыть EDT two-state build pipeline по `ADR-0002` и `ADR-0012`. Выполнено `2026-04-22`: EDT build разделен на независимые export/load стадии; EDT stage коммитит только `edt-*` snapshot после successful export, Designer stage анализирует `designer-<sourceSetName>` после successful/skipped EDT stage, делает partial/full load только по generated diff и коммитит `designer-*` snapshot только после successful load/apply; добавлено regression coverage для Designer и IBCMD EDT flow.
- [x] `ADR-TASK-008`: Реализовать новый `infobase` config contract по `ADR-0018` и server provisioning path по `ADR-0019`. Выполнено `2026-04-22`: `connection`/`credentials` перенесены в `infobase`, добавлен `infobase.dbms`, legacy top-level keys жёстко отклоняются на parse/validation boundary, `IBCMD` поддерживает file/server mapping для `build`/`dump`/`extensions`, `init` для `builder=IBCMD` + server connection выполняет `ibcmd infobase create --create-database` как ensure-step и нормализует benign `already exists` как non-fatal outcome; добавлено regression coverage для loader, validation, config init round-trip, server init и server IBCMD use-case flows.
- [x] `ADR-TASK-003`: Ввести единую transport-neutral policy таймаутов и отмены по `ADR-0013` и `ADR-0014`. Выполнено `2026-04-22`: добавлен общий public `execution_timeout` в конфиг и transport-neutral `ExecutionContext`/`McpCallContext` с `deadline` и cancellation token; все public CLI/MCP entrypoint-ы инициализируют execution context с единым budget и наследуют его во вложенные сценарии. Добавлены `InterruptionSafetyClass` и общий `ProcessExecutionPolicy`; `ProcessExecutor` и interactive `1cedtcli` execution теперь ждут terminal state при running cancellation/timeout, различают `Interruptible`/`GracefulThenKill`/`CriticalNonAbortable` semantics и сохраняют deferred interruption metadata для successful critical phase. `load`, `dump`, `build`, `artifacts`, `init`, `extensions`, one-shot/interactive EDT flows и enterprise test runner переведены на context-aware process policy с safe-point checks и deferred warnings в result/execution diagnostics. На MCP server boundary queued timeout/cancel остаются transport-level admission errors, а running timeout/cancel для shared EDT syntax теперь дожидаются terminal state и возвращаются как structured business failure; перенос shared EDT actor/manager из MCP boundary в общий execution слой остаётся отдельной архитектурной задачей `ADR-TASK-004`.
- [x] `ADR-TASK-004`: Свести CLI EDT interactive execution к shared interactive режиму по `ADR-0007`. Выполнено `2026-04-22`: shared EDT actor/manager перенесён в `src/platform/edt_session.rs` с явными host options и observer boundary; MCP оставлен thin boundary над общим component, а CLI `init`, EDT export в `build` и CLI `syntax edt` при `tools.edt_cli.interactive_mode=true` используют тот же shared actor/manager с lazy startup на command-scope. Прямые production-вызовы `EdtDsl::new_interactive` устранены, добавлено покрытие на one-shot path при `interactive_mode=false`, lazy CLI semantics при `auto_start=true`, reuse shared session в `build` и сохранение MCP timeout/cancel boundary semantics.

## P1

- [ ] `ADR-TASK-005`: Закрыть follow-up gaps атомарной публикации по `ADR-0015`: neutral/caller-specific backup prefix, metadata sidecar на cleanup unit для staging directory внешних артефактов, `CriticalNonAbortable` publication phase после общей execution policy, cleanup warning в едином CLI output contract.

  Частично выполнено `2026-04-22`: в `dump` и `artifacts` добавлены safe-point проверки перед publish; warning/degraded plumbing уже есть для `dump` и single-file artifacts publication; есть частичное regression coverage для orphan cleanup (`dump`: stale/malformed, `artifacts`: stale directory scan).

  Остаются открытыми: neutral/caller-specific backup prefix в `replace_dir_atomically`, metadata sidecar на cleanup unit для `external artifacts` staging directory, фактический `CriticalNonAbortable` contract для publication phase, проброс cleanup warning для external directory publication и недостающее regression coverage для `foreign`/`recent` cleanup cases.
- [ ] `ADR-TASK-006`: Довести `ExecutionOutcome<T>` и step contract до целевого состояния по `ADR-0016`: outcome-driven serialized status/errors/metrics/artifacts, `ExecutionStatus::Cancelled` для фактической terminal cancellation, command-level interruption diagnostics, richer `ExecutionStep` или расширенный `StepResult`.
- [ ] `ADR-TASK-007`: Проработать CLI output по `ADR-0010` как единый high-signal contract для человека и AI-агента: применить критерии корректного output из `spec/ADR_DERIVED_BACKLOG.md`, убрать лишний шум из clean success path, явно показывать warnings/degraded/artifacts/diagnostics и покрыть rendering tests, не меняя JSON contract без отдельного решения.

## P2

- [ ] `ADR-TASK-009`: Усилить regression coverage platform locator по `ADR-0004`: exact/mask selection `8.3`, `8.3.20`, `8.3.27.1789` для всех platform utilities, особенно `ibcmd`; отдельно зафиксировать `tools.platform.path` как root/hint и не терять уже существующее покрытие для `1cv8` и `1cv8c`.
- [ ] Добавить CI workflow wiring из `spec/REAL_ENV_TEST_PLAN.md`: установка 1С на GitHub-hosted runner'ах, bootstrap файловой ИБ через `ibsrv`, trusted/fork gating и upload deploy-ready артефактов.

## P3

- [ ] `ADR-TASK-010`: Добавить архитектурные guardrails для ADR-инвариантов (`ADR-0005`, `ADR-0006`, `ADR-0008`, `ADR-0009`, `ADR-0011`, `ADR-0017`, `ADR-0018`): границы зависимостей use case, platform DSL boundary, workspace lock boundary, validation/docs для config contract, checklist изменения MCP surface.
