# Экспорт JUnit-отчёта YAxUnit — план реализации

> **Для agentic workers:** ОБЯЗАТЕЛЬНЫЙ SUB-SKILL: использовать `superpowers:subagent-driven-development` (рекомендуется) или `superpowers:executing-plans` и выполнять план последовательно по checkbox-шагам.

**Цель:** Выпустить `v8-runner` v0.5.3 с параметром `test yaxunit --junit-output <path>`, который сохраняет исходный корректный JUnit XML атомарно, не оставляет устаревший отчёт и сохраняет действующую семантику результатов тестов.

**Архитектура:** CLI разрешает относительный путь от каталога основного `v8project.yaml` и передаёт его в transport-neutral `TestRequest`. Изолированный модуль `run_tests/junit_export.rs` подготавливает target и публикует проверенный внутренний XML через существующий `StagedPublication`; coordinator задаёт порядок подготовки, запуска, разбора, экспорта и очистки. При отсутствии параметра все новые ветки обходятся, поэтому текущие CLI и MCP-контракты сохраняются.

**Стек:** Rust 2021, `clap`, существующие `quick-xml`, `tempfile` и staged-publication utilities, `assert_cmd`, `insta`, стандартные `cargo test`/`cargo clippy`/`cargo fmt`.

---

## Структура изменений

- Создать `src/use_cases/run_tests/junit_export.rs`: проверка target, удаление старого файла, staging-копирование, `sync_all`, атомарная публикация и unit-тесты.
- Изменить `src/cli/args.rs`: добавить YAxUnit-only параметр `--junit-output`.
- Изменить `src/cli/execute.rs`: разрешить путь от primary config directory и добавить его в `TestRequest`; показать label шага экспорта.
- Изменить `src/use_cases/request.rs`: добавить `junit_output: Option<PathBuf>`.
- Изменить `src/mcp/service.rs`: явно передавать `None` в существующих MCP test requests.
- Изменить `src/domain/test.rs`: добавить стабильный код `junit_export_failed`.
- Изменить `src/use_cases/run_tests.rs`: подключить helper и тип состояния завершения Enterprise.
- Изменить `src/use_cases/run_tests/coordinator.rs`: встроить prepare/export в lifecycle и отложить ранний возврат после Enterprise error только при запрошенном экспорте.
- Изменить `tests/cli_help.rs`: публичный help-контракт.
- Изменить `tests/cli_test.rs`: end-to-end сценарии `all`, `module`, `--full`, failures, timeout, stale target и cleanup.
- Изменить `docs/CAPABILITIES.md` и `SKILL/references/testing.md`: пользовательский workflow.
- Изменить `Cargo.toml` и корневую запись `v8-runner` в `Cargo.lock`: версия 0.5.3.

Из-за требования `AGENTS.md` о двух независимых Rust-review до коммита промежуточные изменения Rust не коммитятся по задачам. После полного tester/reviewer/Rust-expert цикла создаётся один implementation commit установленного формата.

### Задача 1: Публичный CLI и transport-neutral запрос

**Файлы:**
- Изменить: `src/cli/args.rs:230`
- Изменить: `src/cli/execute.rs:80-128,426-469,849-865,2323-2332`
- Изменить: `src/use_cases/request.rs:47-56`
- Изменить: `src/mcp/service.rs:78-134`
- Тест: `tests/cli_help.rs`
- Тест: unit tests в `src/cli/execute.rs`
- Тест: unit tests в `src/cli/args.rs`

- [ ] **Шаг 1: Написать падающий help-тест**

Добавить в `tests/cli_help.rs` тест, проверяющий область видимости параметра:

```rust
#[test]
fn yaxunit_help_exposes_junit_output_only_for_yaxunit() {
    let yaxunit = v8_runner_command()
        .args(["test", "yaxunit", "--help"])
        .output()
        .expect("yaxunit help");
    assert!(yaxunit.status.success());
    assert!(String::from_utf8_lossy(&yaxunit.stdout)
        .contains("--junit-output <PATH>"));

    let va = v8_runner_command()
        .args(["test", "va", "--help"])
        .output()
        .expect("va help");
    assert!(va.status.success());
    assert!(!String::from_utf8_lossy(&va.stdout).contains("--junit-output"));
}
```

Поскольку integration-файл имеет `#![cfg(unix)]`, до реализации добавить кроссплатформенный unit-тест в `src/cli/args.rs` через `Cli::try_parse_from`: он разбирает `test yaxunit --junit-output build/junit.xml all`, проверяет `TestYaxunitArgs.junit_output`, а `test va --junit-output ...` обязан вернуть clap error. Этот unit-тест является обязательным evidence на Windows.

- [ ] **Шаг 2: Подтвердить RED**

Выполнить: `cargo test --test cli_help yaxunit_help_exposes_junit_output_only_for_yaxunit -- --exact`

Ожидается: FAIL на отсутствии `--junit-output` в help YAxUnit.

- [ ] **Шаг 3: Написать падающие unit-тесты mapping**

Сначала расширить тестовые литералы будущим полем и добавить проверки relative, absolute, empty и отличного `basePath`. Relative assertion использует parent `primary_config_path`, а не `config.base_path`. Для empty проверить, что mapping сохраняет `PathBuf::new()` для единой use-case validation, не превращая его в config directory.

- [ ] **Шаг 4: Подтвердить RED mapping**

Выполнить: `cargo test cli::execute::tests::maps_junit_output`

Ожидается: compile failure на отсутствии поля/новой сигнатуры mapping.

- [ ] **Шаг 5: Добавить CLI-поле и request-поле**

В `TestYaxunitArgs` добавить:

```rust
/// Export the original YAxUnit JUnit XML to this path
#[arg(long = "junit-output", value_name = "PATH")]
pub junit_output: Option<PathBuf>,
```

В imports `src/cli/args.rs` использовать `std::path::PathBuf`. В `TestRequest` добавить:

```rust
/// Optional external destination for the original YAxUnit JUnit XML.
pub junit_output: Option<PathBuf>,
```

Во всех существующих MCP-конструкторах `TestRequest` и в Vanessa mapping добавить `junit_output: None`.

- [ ] **Шаг 6: Разрешить путь на границе CLI**

Передать `primary_config_path.as_deref()` из `execute_command` в `execute_test`, затем в `map_test_request`. Добавить функцию:

```rust
fn resolve_junit_output(
    value: Option<&Path>,
    primary_config_path: Option<&Path>,
) -> Result<Option<PathBuf>, UseCaseError> {
    let Some(value) = value else { return Ok(None) };
    if value.as_os_str().is_empty() {
        return Ok(Some(value.to_path_buf()));
    }
    if value.is_absolute() {
        return Ok(Some(value.to_path_buf()));
    }
    let config = primary_config_path.ok_or_else(|| UseCaseError::new(
        UseCaseErrorKind::Validation,
        "--junit-output requires a resolved primary config path",
    ))?;
    let root = config.parent().ok_or_else(|| UseCaseError::new(
        UseCaseErrorKind::Validation,
        "primary config path has no parent directory",
    ))?;
    Ok(Some(root.join(value)))
}
```

YAxUnit mapping должен деструктурировать `TestYaxunitArgs { scope, junit_output }`, а Vanessa всегда записывать `None`. Добавить label:

```rust
"export_junit" => "export JUnit report".to_owned(),
```

- [ ] **Шаг 7: Завершить unit-тесты mapping**

Расширить все существующие конструкторы `TestYaxunitArgs` полем `junit_output: None`. Проверить:

```rust
assert_eq!(request.junit_output, Some(config_dir.join("build/junit.xml")));
```

для относительного пути и:

```rust
assert_eq!(request.junit_output, Some(absolute.clone()));
```

для абсолютного пути. Отдельно проверить, что `PathBuf::new()` остаётся пустым до use-case helper, а `basePath`, отличный от config directory, не влияет на результат. CLI integration покрывает и явный `--config`, и auto-discovery основного `v8project.yaml` из current directory.

- [ ] **Шаг 8: Подтвердить GREEN задачи**

Выполнить:

```text
cargo test --test cli_help yaxunit_help_exposes_junit_output_only_for_yaxunit -- --exact
cargo test cli::args::tests::parses_yaxunit_junit_output
cargo test cli::execute::tests
cargo test mcp::service::tests
```

Ожидается: все выбранные тесты PASS.

### Задача 2: Изолированный атомарный экспорт

**Файлы:**
- Создать: `src/use_cases/run_tests/junit_export.rs`
- Изменить: `src/use_cases/run_tests.rs`
- Изменить: `src/domain/test.rs`
- Изменить: `src/use_cases/staged_publication.rs`

- [ ] **Шаг 1: Написать unit-тесты подготовки target**

В новом модуле сначала добавить тесты:

```rust
#[test]
fn prepare_creates_parents_and_removes_stale_file() {
    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("nested/report.xml");
    fs::create_dir_all(target.parent().expect("parent")).expect("parent");
    fs::write(&target, "stale").expect("stale");

    let export = JunitExport::prepare(target.clone()).expect("prepare");

    assert_eq!(export.target(), target);
    assert!(!target.exists());
}

#[test]
fn prepare_rejects_directory_target_without_deleting_it() {
    let dir = tempdir().expect("tempdir");
    let error = JunitExport::prepare(dir.path().to_path_buf()).expect_err("directory");
    assert!(error.to_string().contains("points to a directory"));
    assert!(dir.path().exists());
}
```

- [ ] **Шаг 2: Подтвердить RED подготовки**

Выполнить: `cargo test use_cases::run_tests::junit_export::tests::prepare_`

Ожидается: compile failure, потому что `JunitExport` ещё не определён.

- [ ] **Шаг 3: Реализовать подготовку**

Создать тип и методы:

```rust
#[derive(Debug)]
pub(super) struct JunitExport {
    target: PathBuf,
}

impl JunitExport {
    pub(super) fn prepare(target: PathBuf) -> Result<Self, AppError> {
        if target.as_os_str().is_empty() || target.file_name().is_none() {
            return Err(AppError::Validation(
                "--junit-output must name a file".to_owned(),
            ));
        }
        if target.is_dir() {
            return Err(AppError::Validation(format!(
                "JUnit output path points to a directory: {}",
                target.display()
            )));
        }
        let parent = target.parent().ok_or_else(|| AppError::Validation(format!(
            "JUnit output path has no parent: {}",
            target.display()
        )))?;
        ensure_dir(parent).map_err(|error| AppError::Runtime(format!(
            "failed to create JUnit output parent '{}': {error}",
            parent.display()
        )))?;
        match fs::remove_file(&target) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(AppError::Runtime(format!(
                "failed to remove previous JUnit output '{}': {error}",
                target.display()
            ))),
        }
        Ok(Self { target })
    }

    pub(super) fn target(&self) -> &Path { &self.target }
}
```

- [ ] **Шаг 4: Написать падающий roundtrip-тест error code**

До добавления enum-варианта расширить таблицу `TestErrorKind` значением `(JunitExportFailed, "junit_export_failed")` и запустить `cargo test domain::test::tests::test_error_codes_roundtrip`. Ожидается compile failure на отсутствии варианта.

- [ ] **Шаг 5: Добавить отдельный error code**

В `src/domain/test.rs` добавить `TEST_ERROR_CODE_JUNIT_EXPORT_FAILED`, вариант `TestErrorKind::JunitExportFailed` и симметричные ветки `code`, `from_code`, `test_execution_status` и roundtrip-теста.

- [ ] **Шаг 6: Написать падающие тесты raw-copy и атомарной публикации**

Добавить тест с `Vec<u8>`, содержащим XML declaration, properties, CDATA/сообщение и нестандартные пробелы, затем проверить `fs::read(target) == input`. Добавить fault-injection seam для ошибок `write_all`, `sync_all`, metadata sidecar и publication; после каждой ошибки проверить отсутствие target, staging и sidecar, а при ошибке cleanup — наличие её текста в возвращённом `AppError`.

- [ ] **Шаг 7: Сделать cleanup shared publication наблюдаемым**

В `StagedPublication::prepare_file` при ошибке `write_stage_metadata` вызвать усиленный `cleanup_staging_path`, который удаляет stage/sidecar и добавляет ошибки удаления к исходному `AppError`. Тот же helper должен использоваться JUnit-export при ошибке `publish_file`. Покрыть частично созданный metadata sidecar детерминированным private test hook, а не permission-зависимым тестом.

- [ ] **Шаг 8: Реализовать публикацию проверенных байтов**

Добавить результат и метод:

```rust
pub(super) struct JunitExportOutcome {
    pub path: PathBuf,
    pub cleanup_warning: Option<String>,
    pub deferred_interruption: Option<ExecutionInterruption>,
}

pub(super) fn publish(
    &self,
    context: &ExecutionContext,
    bytes: &[u8],
) -> Result<JunitExportOutcome, AppError> {
    let publication = StagedPublication::prepare_file(
        &self.target,
        "yaxunit-junit-output",
        ".junit-stage",
        "xml",
    )?;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(publication.staging_path())
        .map_err(|error| cleanup_failed_stage(&publication, AppError::Runtime(format!(
            "failed to create staged JUnit report: {error}"
        ))))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| cleanup_failed_stage(&publication, AppError::Runtime(format!(
            "failed to write or synchronize staged JUnit report: {error}"
        ))))?;
    let outcome = publication
        .publish_file(context, "failed to publish JUnit report")
        .map_err(|error| cleanup_failed_stage(&publication, error))?;
    Ok(JunitExportOutcome {
        path: self.target.clone(),
        cleanup_warning: outcome.cleanup_warning,
        deferred_interruption: outcome.deferred_interruption,
    })
}
```

`cleanup_failed_stage` должен использовать наблюдаемый shared cleanup, удалить staging-файл и sidecar из `metadata_sidecar_path`, собрать обе ошибки удаления и добавить их в текст исходного `AppError`.

- [ ] **Шаг 9: Подтвердить GREEN helper**

Выполнить:

```text
cargo test use_cases::run_tests::junit_export::tests
cargo test domain::test::tests
```

Ожидается: все выбранные тесты PASS.

### Задача 3: Экспорт в штатном test lifecycle

**Файлы:**
- Изменить: `src/use_cases/run_tests.rs`
- Изменить: `src/use_cases/run_tests/coordinator.rs`
- Изменить: `tests/cli_test.rs`

- [ ] **Шаг 1: Написать падающие unit-тесты однократного чтения JUnit**

Перед изменением parser добавить тесты `parse_junit_report_returns_validated_raw_bytes`, `parse_junit_report_rejects_empty_bytes` и test hook, который после чтения подменяет файл на диске. Последний тест должен доказать, что `ValidatedJunit.bytes` и разобранный report относятся к исходному содержимому, а последующая подмена не влияет на export buffer.

- [ ] **Шаг 2: Подтвердить RED parser contract**

Выполнить: `cargo test use_cases::run_tests::tests::parse_junit_report_returns_validated_raw_bytes`

Ожидается: compile failure на отсутствии `ValidatedJunit.bytes`.

- [ ] **Шаг 3: Реализовать единое чтение и parse**

В `run_tests.rs` добавить:

```rust
struct ValidatedJunit {
    report: TestReport,
    bytes: Vec<u8>,
}
```

`parse_junit_report` должен выполнить один `fs::read`, отдельно классифицировать `NotFound` как `JunitNotProduced`, пустой buffer как `JunitEmpty`, вызвать `junit::parse_normalized(Cursor::new(bytes.as_slice()))` и при успехе вернуть `ValidatedJunit { report, bytes }`. Повторно открывать `report.xml` при экспорте запрещено.

- [ ] **Шаг 4: Написать failing CLI-тесты для `all` и `module`**

Добавить два теста, запускающих соответственно:

```rust
.args(["--config", config, "test", "yaxunit", "--junit-output", "build/results/yaxunit.xml", "all"])
```

и:

```rust
.args(["--config", config, "test", "yaxunit", "--junit-output", output, "module", "Foo"])
```

Проверить успешный exit, `fs::read(output) == JUNIT_SMOKE_REPORT_FIXTURE.as_bytes()`, наличие `JUnit report exported to` в stdout и отсутствие успешного run directory.

- [ ] **Шаг 5: Подтвердить RED lifecycle**

Выполнить: `cargo test --test cli_test junit_output -- --nocapture`

Ожидается: FAIL, внешний файл отсутствует.

- [ ] **Шаг 6: Подготовить export до use-case validation и build**

Сразу после вычисления `mode`, до `validate_target`, interruption и `validate_runner_profile_id`, вызвать:

```rust
let junit_export = match args.junit_output.clone() {
    Some(path) => match JunitExport::prepare(path.clone()) {
        Ok(export) => Some(export),
        Err(error) => return Err(export_failure_before_run(
            TestTarget::All, mode.clone(), path, error, started,
        )),
    },
    None => None,
};
```

`export_failure_before_run` имеет сигнатуру `fn export_failure_before_run(target: TestTarget, mode: TestOutputMode, output: PathBuf, error: AppError, started: Instant) -> TestExecutionFailure` и создаёт failed `export_junit` step, `TestErrorKind::JunitExportFailed`, `ExecutionStatus::Failed` и payload `TestRunResult`. Такая позиция гарантирует удаление stale target даже при invalid module и cancellation. Команды без параметра проходят прежнюю ветку без дополнительных файловых операций.

- [ ] **Шаг 7: Опубликовать после успешного parse**

Из `junit_parse.payload` извлечь `ValidatedJunit`, передать `validated.bytes.as_slice()` в `publish`, затем использовать `validated.report` для status и rendering. Вызов выполняется до `parse_runner_log`. При успехе:

```rust
let message = format!("JUnit report exported to {}", outcome.path.display());
steps.push(succeeded_step(
    "export_junit",
    ExecutionStepKind::Publish,
    export_started.elapsed().as_millis() as u64,
    message.clone(),
).with_target(outcome.path.display().to_string()));
warnings.extend(outcome.cleanup_warning);
diagnostics.push(message);
```

Использовать существующий `ExecutionStepKind::Publish`.

Если `outcome.deferred_interruption` задан, преобразовать его через `deferred_command_interruption_details(interruption, "export_junit", "JUnit report publication completed")`, добавить соответствующий `deferred_interruption_warning` в warnings и сохранить interruption в итоговом `ExecutionOutcome`. Артефакт остаётся опубликованным, а результат получает warning/cancellation metadata; значение нельзя молча отбрасывать.

Информационная строка экспорта не должна превращать успешный результат в `Tests completed with warnings`. В `test_has_actionable_success_signal` исключить сообщения с точным префиксом `JUnit report exported to `, сохранив их видимыми в failure-ветке. В успешной ветке `render_test_text` отдельно добавить внешний путь из succeeded-шага `export_junit`, потому что общий `append_step_signals` вызывается только для failure/warning:

```rust
if let Some(path) = result.steps.iter()
    .find(|step| step.name == "export_junit" && step.ok)
    .and_then(|step| step.target.as_deref())
{
    details.push(format!("JUnit report: {path}"));
}
```

- [ ] **Шаг 8: До реализации написать RED-тесты failed/full/export failure**

Добавить тесты failed JUnit, `--full`/compact byte equality и export publication failure. Для каждого сначала запустить отдельный exact test и подтвердить ожидаемое отсутствие/неверный status, а не compile error из другой задачи.

- [ ] **Шаг 9: Сформировать payload при export failure**

Добавить `export_failure_after_parse(target, mode, report, completion, artifacts, error, warnings, steps, started) -> TestExecutionFailure`. Он устанавливает `ExecutionStatus::Failed`, сохраняет run directory через `retain_run_artifacts`, добавляет metrics/rendered payload и формирует errors в порядке `JunitExportFailed`, Enterprise error, `TestFailures`. Первичным `AppError` остаётся конкретная ошибка экспорта; дубликаты одинаковых кодов не добавляются.

- [ ] **Шаг 10: Проверить failed tests и `--full`**

Добавить тест с JUnit, содержащим passed и failed cases, дважды выполнить command с `--full` и без него и проверить одинаковые raw bytes. Для failed run проверить прежний ненулевой exit и наличие внешнего файла/диагностики.

- [ ] **Шаг 11: Подтвердить GREEN штатного lifecycle**

Выполнить: `cargo test --test cli_test junit_output -- --nocapture`

Ожидается: тесты `all`, `module`, failed и full/compact PASS.

### Задача 4: Таймауты, process errors и stale-file гарантия

**Файлы:**
- Изменить: `src/use_cases/run_tests.rs`
- Изменить: `src/use_cases/run_tests/coordinator.rs`
- Изменить: `tests/cli_test.rs`

- [ ] **Шаг 1: Написать failing timeout-тест с корректным JUnit**

Использовать существующий `setup_project(..., sleep_seconds: Some(2))` при timeout 1: fake Enterprise записывает `report.xml` до sleep. Перед запуском записать `stale` в output. Проверить timeout exit/status, побайтово корректный внешний XML и диагностический путь.

- [ ] **Шаг 2: Написать RED-тесты матрицы abnormal completion**

До рефакторинга добавить unit/CLI cases: timeout+valid, timeout+missing, timeout+malformed, cancellation+valid и cancellation+missing. Для invalid JUnit ожидать primary JUnit error/status, затем Enterprise error в `execution.errors`, сохранённую interruption metadata и retained run directory. Для valid JUnit ожидать исходный timeout/cancel status и внешний файл. Каждый exact test сначала должен падать на текущем раннем возврате.

- [ ] **Шаг 3: Ввести состояние завершения Enterprise**

В `run_tests.rs` добавить внутренний enum:

```rust
enum EnterpriseCompletion {
    Completed(crate::platform::result::PlatformCommandResult),
    Failed {
        kind: Option<TestErrorKind>,
        error: AppError,
        interruption: Option<ExecutionInterruptionDetails>,
        status: ExecutionStatus,
    },
}
```

Добавить методы `diagnostics(&self, config) -> Vec<String>`, `process_exit_code(&self) -> Option<i32>`, `append_errors(&self, &mut Vec<ExecutionError>)` и `interruptions(&self) -> Vec<ExecutionInterruptionDetails>`. Они сопоставляют enum и никогда не обращаются к `PlatformCommandResult` в варианте `Failed`. В ветке `enterprise.run_launch` сохранить `Failed`, если `junit_export.is_some()`. Если экспорт не запрошен, оставить текущий немедленный возврат без изменения payload/status.

- [ ] **Шаг 4: Реализовать явную матрицу результата**

Coordinator обрабатывает состояния в следующем порядке:

```text
Completed + invalid JUnit             -> primary JUnit error, current parse behavior
Failed + invalid JUnit                -> primary JUnit error; Enterprise error second; interruption retained
Completed/Failed + valid + export Err -> primary JunitExportFailed; Enterprise/TestFailures appended
Completed + valid + export Ok         -> current process/test result
Failed + valid + export Ok            -> original Enterprise status/error/interruption
```

Для `Failed + invalid JUnit` diagnostics состоят из JUnit message, затем `completion.error.to_string()`. `ExecutionStatus` берётся из `test_execution_status(Some(junit_kind), false)`, retained artifacts обязательны, а возвращаемый `AppError::Runtime` содержит JUnit message. Для `Failed + valid + export Ok` status и primary `AppError` берутся из `completion`, report metrics/payload и export diagnostic сохраняются.

- [ ] **Шаг 5: После parse/export вернуть исходную Enterprise error**

После построения полного и rendered report сопоставить `EnterpriseCompletion::Failed`: добавить исходный error kind и interruption, metrics/payload, retained artifacts и export diagnostic, затем вернуть исходный `AppError`. При успешном экспорте не подменять timeout/cancellation/process status.

- [ ] **Шаг 6: До реализации написать RED-тесты stale/path failures**

Сначала расширить fake script режимом `ReportFixture::{Missing, Empty, Xml(&str)}`, чтобы missing не создавал файл, empty создавал ровно 0 bytes, а Xml записывал контролируемые bytes без добавочного newline. Добавить exact tests для build failure, invalid module, pre-run cancellation, missing, empty, malformed, directory target и write/publish failures; подтвердить RED каждого поведения.

- [ ] **Шаг 7: Реализовать и проверить отсутствие нового JUnit и ранние ошибки**

Добавить сценарии:

- старый output удаляется до build, а build failure не создаёт новый;
- missing, empty и malformed report после запуска оставляют output отсутствующим;
- directory target отклоняется и не удаляется;
- unwritable target/parent приводит к ненулевому exit (на Unix использовать permission guard и восстанавливать permissions перед teardown).

Во всех сценариях без `--junit-output` повторить ключевой baseline и убедиться, что прежнее поведение retention/exit code не изменилось.

- [ ] **Шаг 8: Подтвердить GREEN ошибок**

Выполнить:

```text
cargo test --test cli_test junit_output -- --nocapture
cargo test --test cli_test test_timeout_retains_artifacts -- --exact
```

Ожидается: новые сценарии PASS, существующий timeout baseline PASS.

### Задача 5: Документация и версия v0.5.3

**Файлы:**
- Изменить: `docs/CAPABILITIES.md`
- Изменить: `SKILL/references/testing.md`
- Изменить: `Cargo.toml`
- Изменить: `Cargo.lock`

- [ ] **Шаг 1: Обновить пользовательскую документацию**

Добавить обе формы команды и явно записать:

```text
Относительный --junit-output разрешается от каталога основного v8project.yaml.
Файл является оригинальным JUnit XML YAxUnit, сохраняется и при падениях тестов,
не зависит от --full и атомарно публикуется после удаления отчёта прошлого запуска.
```

В repo-local skill оставить краткий operational contract и пример CI-пути без внутренних деталей реализации.

- [ ] **Шаг 2: Поднять версию**

В `Cargo.toml` заменить package version на `0.5.3`, затем выполнить `cargo check --locked` после обновления lock либо `cargo check` для обновления корневой записи `v8-runner` в `Cargo.lock`. Не изменять версию стороннего пакета `outref 0.5.2`.

- [ ] **Шаг 3: Проверить версию**

Выполнить:

```text
cargo test --test cli_help root_version_flag_prints_application_version -- --exact
cargo run --quiet -- version
cargo run --quiet -- --version
```

Ожидается: обе команды печатают `v8-runner 0.5.3`, тест PASS.

### Задача 6: Независимое ревью, полная проверка и commit

**Файлы:** все изменённые файлы задачи.

- [ ] **Шаг 1: Выполнить форматирование и локальные проверки**

Выполнить:

```text
cargo fmt --all -- --check
cargo test --lib
cargo test --tests
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```

Ожидается: все команды завершаются с кодом 0.

На Windows `tests/cli_help.rs` и `tests/cli_test.rs` имеют `#![cfg(unix)]` и могут завершиться как `0 tests`. Поэтому дополнительно выполнить эти suites в доступном Linux/WSL окружении:

```text
wsl bash -lc 'cd /mnt/f/1C/Projects/v8-runner-rust && cargo test --test cli_help && cargo test --test cli_test'
```

Если WSL mount отличается, определить его командой `wsl wslpath -a 'F:\1C\Projects\v8-runner-rust'` и использовать фактический путь. Ожидается: оба Unix suites выполняют ненулевое количество тестов и PASS.

На Windows отдельно выполнить кроссплатформенные `cli::args`, `cli::execute`, `junit_export`, `staged_publication` и `support::fs` unit-тесты. Они должны реально запустить новые cases (не `0 tests`) и покрыть parser/mapping, directory target, stale removal, raw-byte write, sync/publication rollback и cleanup diagnostics. Unix permission/cancellation CLI cases дополняют, но не заменяют этот контур.

- [ ] **Шаг 2: Независимый tester-проход**

Tester-субагент запускает targeted и полный suites, сверяет acceptance criteria 1–10 с фактическими файлами и выводом. Каждый finding исправляется, затем соответствующие тесты повторяются.

- [ ] **Шаг 3: Независимый общий reviewer-проход**

Reviewer-субагент проверяет spec coverage, backward compatibility, error precedence, cleanup, cancellation, API boundaries и применяет checklist `/rust-expert-best-practices-code-review`. Каждый finding исправляется либо получает явный waiver.

- [ ] **Шаг 4: Отдельный Rust expert-проход**

Другой субагент независимо применяет `/rust-expert-best-practices-code-review` ко всему Rust diff: ownership/borrowing, errors, type safety, cross-platform filesystem behavior, атомарность и лишние allocations. Findings не объединять молча с общим review.

- [ ] **Шаг 5: Повторить verification после исправлений**

Повторить команды шага 1, затем проверить:

```text
git diff --check
git status --short
git diff -- Cargo.toml Cargo.lock
```

Убедиться, что root package имеет версию 0.5.3, документация и skill обновлены, staging-файлы/тестовые артефакты не попали в worktree.

- [ ] **Шаг 6: Создать implementation commit**

После обоих review-контуров и зелёной проверки:

```text
git add Cargo.toml Cargo.lock src tests docs/CAPABILITIES.md SKILL/references/testing.md docs/superpowers/plans/2026-07-26-junit-output.md docs/superpowers/specs/2026-07-26-junit-output-design.md
git commit -m "feat(test): export YAxUnit JUnit reports" -m "- add atomic --junit-output publication for all and module runs
- preserve reports and exit semantics across failures
- document the workflow and release v0.5.3"
```

- [ ] **Шаг 7: Финальный completion audit**

Сопоставить каждый пункт исходного требования и acceptance criteria 1–10 с конкретным тестом или свежим command output. Не объявлять завершение при отсутствии прямого evidence хотя бы по одному пункту.
