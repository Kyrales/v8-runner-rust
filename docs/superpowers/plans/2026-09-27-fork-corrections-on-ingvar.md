# Перенос поправок форка на линию Ингвара — план реализации

> **Для исполнителей:** отмечать шаги `- [ ]`. Перед каждой группой сверять действующие `spec/rules/` и не копировать старые коммиты целиком. По решению владельца от 2026-09-28 ревью реализации проводится после всех задач двумя общими раундами `sol high`, без промежуточного ревью каждой задачи; отдельная проверка Rust-экспертом остаётся обязательной.

**Цель:** добавить нужное поведение форка `0.5.5` в нынешних владельцев кода v8-runner `0.11.0`, проверить его и сохранить возможность брать будущие изменения Ингвара.

**Архитектура:** CLI переводит новые ключи в существующие запросы `use_cases`; сценарии используют нынешние EDT, process и staged-publication адаптеры. Каждая группа завершается своим поведением, тестами и обновлением затронутого публичного контракта. Миграция `v8-ci` начинается отдельным планом после проверки раннера.

**Стек:** Rust, clap, serde, cargo tests, фальшивые исполняемые файлы в интеграционных тестах; живой EDT-проект для заключительной проверки при наличии 1С/EDT.

**Спецификация:** [2026-09-27-fork-corrections-on-ingvar-design.md](../specs/2026-09-27-fork-corrections-on-ingvar-design.md).

## Общие ограничения

- Основа — линия Ингвара от `10fa0a7`; `develop`/`v0.5.5` (`83e4f69`) служит источником требований и старых проверок, а не патчем для слияния. Уже внесённое в рабочую копию уточнение спецификации включить в первый документальный commit группы.
- Не возвращать `builder`, общий `execution_timeout`, старые реализации `syntax`/`--no-build`, отдельные парсеры JUnit или второй механизм публикации. Использовать `providers`, `check`, `--no-push`, `StagedPublication` и два контекста изменений EDT.
- Рабочий `F:\1C\Projects\v8-ci\` и его раннер `0.5.5` не менять в этом плане. Для живой проверки использовать копию `F:\1C\Projects\otus_JenkinsExample_EDT\`, не исходный проект.
- Перед решением читать правила затронутой области. Код, проверки, `spec/rules/`, `README.md`/`docs/CAPABILITIES.md`/`docs/CONFIGURATION.md`/`docs/DEEP_DIVE.md` и `SKILL/SKILL.md` обновлять вместе, когда меняется их контракт. Порожденные схемы обновлять только штатными тестами.
- Для каждой нетривиальной группы: skeptic до реализации и tester по мере готовности; после всей реализации провести два общих reviewer pass и отдельный Rust-expert pass перед commit. Сверить diff со спецификацией и правилами, исправить замечания или получить разрешённый waiver. Очистка дублирования требует reintroduction guard.

## Текущий статус

- [x] Вложенный JUnit Vanessa, читаемые Windows-пути `init` и выбор EDT-проекта при `push` внесены отдельными commit; живой `push` на одноразовой копии EDT выполнен.
- [x] Исключения EDT, значения Vanessa после `tools download va`, управляемое ожидание процесса, события сканирования и Windows contract реализованы и прошли доступные адресные проверки.
- [x] CI smoke-скрипты переведены с `builder`/`build` на `providers.push`/`push`; синтаксис Bash проверен, Windows contract прошёл.
- [x] На одноразовой копии EDT выполнены `push`, `check`, YaXUnit и Vanessa; результаты и ограничения записаны в рабочем журнале.
- [x] Внешний JUnit YaXUnit: произвольный путь сохранён по согласованной спецификации; identity-проверки и безопасные операции удаления/публикации реализованы.
- [x] Для будущей публикации JUnit XML читается один раз в буфер и разбирается из него; unit-тест закрепляет точные байты после замены исходного файла. Публикация подключена к успешному и аварийному завершению прогона, включая типизированную ошибку экспорта.
- [x] Заключительные два общих ревью `sol high` проведены; замечания обоих раундов исправлены и повторно проверены. Отдельный Rust-expert checklist также проведён: усилены захват identity до удаления, восстановление dangling symlink и потоковый разбор JUnit.
- [x] Unix/Linux ограничение записано: текущая машина Windows, WSL не содержит Rust toolchain, Unix-only CLI-тесты не запускались.

## Карта владельцев

| Группа | Нынешний владелец | Проверки и контракт |
| --- | --- | --- |
| JUnit YaXUnit | `src/cli/{args,execute}.rs`, `src/use_cases/request.rs`, `src/use_cases/run_tests/{coordinator,helpers}.rs`, `src/use_cases/staged_publication.rs`, `src/domain/test.rs` | `tests/cli_test.rs`, `spec/rules/wire/test-data.md`, схемы `docs/schemas/` |
| Исключения EDT | `src/cli/{args,execute}.rs`, `src/use_cases/{request,check_syntax}.rs`, `src/mcp/{service,edt_syntax}.rs`; разбор журнала остаётся в `src/parsers/edt_validation.rs` | `tests/cli_syntax.rs`, `spec/rules/{cli/check-is-one-command,wire/check-data}.md` |
| Экспорт EDT | `src/use_cases/build_project.rs` и нынешний `src/platform/edt.rs` | `tests/cli_build.rs`, `spec/rules/use-cases/edt-keeps-two-change-contexts.md` |
| Vanessa | `src/use_cases/{vanessa,tools_download}.rs`, `src/domain/tools_download.rs`, `src/cli/execute.rs` | `tests/{cli_test,cli_tools_download}.rs`, контракт `tools download` и конфигурации |
| Ожидание процесса | `src/platform/process.rs`, вызывающие `src/use_cases/launch_app.rs` и `run_tests` | unit tests процесса, `tests/cli_launch.rs`, правила отмены и wire |
| Windows и сканирование | `src/use_cases/config_init.rs`, `src/change_detection/{analyzer,scanner}.rs` | `tests/cli_config_init.rs`, проверки сканирования, правила выбора full/partial |

## Пять сценариев для особого ревью

1. Внешний JUnit указывает на ссылку или цель, подменённую во время теста: чужой файл остаётся нетронутым. Проверки задачи 1.
2. YaXUnit завершился с ошибкой или отменой после записи XML: отчёт экспортируется, а исход прогона не превращается в успех. Проверки задачи 2.
3. EDT вернул ненулевой код и все распознанные проблемы попали в исключения: результат остаётся `tool_failed`. Проверки задачи 3.
4. `tools download va` запущен без `VAParams.json` или `features`: EPF загружен, некорректная `tests.va` не создана, прежние `push`/`check` доступны. Проверки задачи 6.
5. Ожидание процесса дало ошибку очистки после timeout/отмены: причина не скрывается статусом timeout/успеха. Проверки задачи 7.

### Задача 1: Путь и защита внешнего JUnit

**Файлы:** `src/cli/args.rs`, `src/cli/execute.rs`, `src/use_cases/request.rs`, `src/use_cases/run_tests/coordinator.rs`, `src/support/fs.rs` для защищённой файловой операции; `tests/cli_test.rs`.

**Интерфейс:** `TestYaxunitArgs.junit_output: Option<PathBuf>` переходит в `TestRequest.junit_output: Option<PathBuf>`; MCP создаёт запрос с `None`. Проверка цели получает путь основного конфига, не `basePath`, и выполняется до `push`/запуска под существующим замком `workPath`.

- [x] Добавить CLI-тесты `junit_output_rejects_va_and_unsafe_targets` и `junit_output_resolves_against_primary_config`: пустой путь, путь без имени файла, каталог, symlink/reparse point, основной/локальный YAML, объявленный source-set, читаемый файл инструмента и отличие `basePath`; ни один отказ не запускает платформу.
- [x] Добавить детерминированные race-тесты подмены непосредственно между проверкой и файловым syscall удаления и между повторной проверкой и заменой при публикации: новая цель не удаляется и не заменяется.
- [x] Запустить `cargo test --locked --test cli_test junit_output_` и убедиться, что новые проверки падают по отсутствующему контракту.
- [x] Добавить ключ только в YaXUnit, передать `Option<PathBuf>` в запрос и проверить тип, идентичность и защищённые пути до подготовительных шагов. Удалять старую цель через файловый примитив, связанный с проверенной identity: OS-операция по открытому handle, где доступна; иначе атомарно перенести цель в частный sibling quarantine, проверить identity перенесённого объекта и удалять только при совпадении. При несовпадении восстановить цель без удаления содержимого; при невозможности восстановления сохранить quarantine и вернуть его путь в ошибке. Не считать замок `workPath` защитой от внешнего actor.
- [x] На границе замены повторно сравнить identity в `StagedPublication`/файловом примитиве, не ослабляя `a-target-is-rechecked-before-publication`.
- [x] Повторить `cargo test --locked --test cli_test junit_output_`; обновить описание CLI в `docs/CAPABILITIES.md` и `SKILL/SKILL.md`. Commit: `feat(test): validate external JUnit target`.

### Задача 2: Публикация JUnit и результат теста

**Файлы:** `src/use_cases/run_tests/{coordinator,helpers}.rs`, `src/use_cases/run_tests.rs`, `src/domain/test.rs`, `src/command_envelope.rs`, `src/command_data.rs`; `tests/cli_test.rs`, `spec/rules/wire/test-data.md`, порождённые схемы.

**Интерфейс:** `RunArtifacts.junit_xml` читается один раз в `Vec<u8>`; существующий JUnit-парсер проверяет `Cursor<&[u8]>`, затем те же байты передаются `StagedPublication::prepare_file`/публикации. Шаг называется `export_junit`, код инфраструктурного отказа — `junit_export_failed`. Для уже завершившегося прогона с отменой нужен узкий путь критической публикации готового отчёта: он откладывает повторную отмену до завершения файловой операции, затем возвращает первоначальный статус отмены.

- [x] Добавить проверки побайтового равенства внутреннего и внешнего XML в compact/full и подмены XML-файла после разбора: экспортируются ровно проверенные байты буфера. Без флага прежний ответ и порядок шагов сохраняются. Добавить сценарии упавших тестов, ненулевого выхода, timeout/отмены до публикации и во время её границы, отсутствующего XML, ошибки публикации и очистки staging. При одновременной ошибке прогона и экспорта проверять обе типизированные причины в ответе.
- [x] Запустить `cargo test --locked --test cli_test junit_output_` и подтвердить ожидаемые падения новых проверок.
- [x] Вынести финализацию XML в общий путь результата: вызывать её и после успешного `enterprise.run_launch`, и после его раннего отказа в `run_tests/coordinator.rs` (сейчас отказ возвращается до `parse_junit`). Если корректный XML появился, прочитать его один раз в буфер, разобрать `Cursor<&[u8]>` существующим парсером и опубликовать тот же буфер через текущего владельца `StagedPublication`. Для уже полученной отмены использовать узкую критическую публикацию готового отчёта, не общий обход отмены. Сохранить исходную классификацию запуска/отмены и payload формы `test`; при ошибке экспорта сохранить также исходную причину теста/процесса, внутренний каталог и наблюдаемую ошибку очистки staging.
- [x] Обновить `spec/rules/wire/test-data.md`, тестируемую форму конверта, пользовательские документы и `SKILL/SKILL.md`; породить схемы командой `$env:UPDATE_COMMAND_DATA_SCHEMAS='1'; cargo test --locked generated_command_data_schemas_are_current; Remove-Item Env:UPDATE_COMMAND_DATA_SCHEMAS`. Если изменён общий envelope, выполнить PowerShell-команды с `$env:UPDATE_ENVELOPE_SCHEMA='1'` и тестом `generated_envelope_schema_is_current`, затем удалить переменную.
- [x] Прогнать `cargo test --locked --test cli_test junit_output_` и `cargo test --locked generated_command_data_schemas_are_current`; commit: `feat(test): publish YaXUnit JUnit report`.

### Задача 3: Точные исключения EDT в `check`

**Файлы:** `src/cli/{args,execute}.rs`, `src/use_cases/{request,check_syntax}.rs`, `src/parsers/edt_validation.rs`, `src/mcp/{service,edt_syntax}.rs` для сохранения нынешнего MCP-вызова; `tests/cli_syntax.rs`, `spec/rules/wire/check-data.md`, `docs/CAPABILITIES.md`, `SKILL/SKILL.md`.

**Интерфейс:** `SyntaxTargetRequest::Edt` получает `exception_file: Option<PathBuf>`; MCP передаёт `None`, Designer с ключом отказывает. `edt_validation::parse_detailed(&str) -> ParsedEdtValidation` возвращает `issues` и число нераспознанных строк; существующий `parse` при необходимости остаётся совместимой обёрткой. Нормализация пары путь/сообщение имеет одного владельца рядом с фильтрацией EDT в `check_syntax`, не в JUnit/Designer-парсерах.

- [x] Добавить тесты CLI: табуляция разделяет два поля; точное совпадение после Unicode lowercase и нормализации пробелов; подстрока не совпадает; комментарии и пустые строки пропускаются; неверная строка/нечитаемый файл отказывают до EDT CLI; Designer ключ отклоняет; preview файл не читает.
- [x] Добавить матрицу кодов EDT: `0` и оставшиеся проблемы → `issues_found`; ненулевой код и полностью отфильтрованные проблемы, нераспознанные строки или stderr → `tool_failed`; валидные оставшиеся проблемы без этих условий → `issues_found`. Отдельно проверить, что парсер возвращает число нераспознанных строк и `check` использует его при ненулевом коде. Проверить несколько проектов и старый MCP-вызов без файла.
- [x] Запустить `cargo test --locked --test cli_syntax exception_file_` и unit tests `edt_validation::`; новые проверки должны показать текущий пробел.
- [x] Реализовать `parse_detailed`, разбор файла и точную фильтрацию результата в нынешней ветке `run_edt_syntax`. Обновить все конструкторы/деструктурирование EDT-варианта в `mcp/service.rs` и `mcp/edt_syntax.rs`, передавая `None`; не добавлять поле в публичный MCP-контракт.
- [x] Обновить правило/схему ответа только если меняется публичная форма `check`; запустить `cargo test --locked --test cli_syntax exception_file_` и соответствующий schema gate. Commit: `feat(check): filter exact EDT exceptions`.

### Задача 4: Выбор EDT-проекта при `push`

**Файлы:** `src/use_cases/build_project.rs`, при подтверждённой необходимости `src/platform/edt.rs`; `tests/cli_build.rs`.

**Интерфейс:** выбранный `source-set.path` определяет проект экспорта; имеющийся `EdtDsl::export_project_path(&Path, &Path)` используется, если тест обнаружит выбор по имени другого проекта.

- [x] Тестом с двумя EDT-проектами, разными именами набора/каталога/`.project` и неизменным набором зафиксировать выбранный проект и порядок обновления `edt-`/`designer-`.
- [x] Подтвердить прежний выбор по имени, заменить вызов экспорта существующим адаптером по пути; Windows `cargo check --locked --tests` прошёл, Unix-тесты на этой машине отключены.
- [ ] Выполнить `cargo test --locked --bin v8-runner edt_export_selected_` и `cargo test --locked --test cli_build build_edt_text_interleaves_export_stage_after_edt_log` на Linux/macOS; остальные проверки и живой `push` на одноразовой копии пройдены, правка в commit `03a9834`.

### Задача 5: Путь JUnit в параметрах Vanessa

**Файлы:** `src/use_cases/vanessa.rs`, `tests/cli_test.rs`.

**Интерфейс:** `apply_test_overlay` пишет один `junit_dir` в верхний `КаталогВыгрузкиJUnit` и в `ОтчетJUnit.КаталогВыгрузкиJUnit`, не заменяя прочие поля объекта.

- [x] Добавить тесты шаблона с отсутствующим и существующим `ОтчетJUnit`, чужими полями и неверным типом узла; ошибка называет `ОтчетJUnit` до запуска Enterprise.
- [x] Запустить `cargo test --locked --bin v8-runner va_nested_junit` (Windows unit tests; `tests/cli_test.rs` доступен только на Unix), реализовать адресное обновление JSON и повторить тест.
- [x] Обновить пользовательское описание Vanessa при изменении его контракта; commit `fix(test): set Vanessa nested JUnit path`.
- [ ] После замечания ревью добавлены CLI проверки вложенного пути и отказа до запуска Enterprise; выполнить `cargo test --locked --test cli_test test_va_` на Linux/macOS и закрыть замечание по результату. На Windows `tests/cli_test.rs` отключён условием `cfg(unix)`.

### Задача 6: Настройки Vanessa после `tools download va`

**Файлы:** `src/use_cases/tools_download.rs`, `src/domain/tools_download.rs`, `src/cli/execute.rs`; `tests/cli_tools_download.rs`, `spec/rules/wire/tools-download-data.md`, `docs/CONFIGURATION.md`, `docs/CAPABILITIES.md`, `SKILL/SKILL.md`.

**Интерфейс:** добавить к `ToolsDownloadResult` внутреннее `warnings: Vec<String>` с `#[serde(skip)]` и `#[schemars(skip)]`, чтобы форма `data` и схема не изменились; `src/cli/execute.rs` переносит их в верхний `Envelope.warnings` и текстовый renderer. Для Vanessa читать оба исходных YAML и эффективное рекурсивное объединение (локальный слой имеет приоритет); добавлять только поля, отсутствующие в обоих слоях, в один локальный YAML адресной вставкой, не переписывая основной. Уже заданные пользовательские поля не перезаписываются. Подготовленный локальный текст проверять через общий разбор/слияние в режиме допуска `tools download` и адресную проверку Vanessa до единственной публикации; не требовать наличия иных инструментов/исходников. При ошибке проверки исходные файлы остаются побайтово прежними.

- [x] Добавить тесты для пустой, частичной, полной и неподдерживаемой секции `tests`, сохранения комментариев/посторонних секций в обоих YAML и идемпотентного повторного вызова. Покрыть значения только в основном, только в локальном и разделённые между слоями, включая пользовательские значения, отличные от стандартных; проверить точные добавляемые значения `3600`, `tools/VAParams.json`, `all`, `3600000`, `features`, `[IgnoreOnCIMainBuild]`. Отдельно проверить, что ожидающий другой инструмент/исходник проект по-прежнему допускается к `tools download va`.
- [x] Добавить тест без `VAParams.json`/`features`: EPF скачан, `tests.va` не создана, предупреждение есть в text и в верхнем JSON `warnings`, отсутствует `data.warnings`, `push`/`check` загружают конфиг. Для уже заданных некорректных путей/профиля в `tests.va` проверить отказ без изменения обоих YAML; обновить прежний тест `tools_download_repairs_pending_vanessa_configuration`, который ожидает успешное маскирование ошибочных пользовательских значений.
- [x] Запустить `cargo test --locked --test cli_tools_download vanessa_` и убедиться, что новые проверки падают на текущем поведении.
- [x] Реализовать проверку предпосылок, заранее проверить уже заданные пользовательские значения Vanessa, передачу предупреждений и адресную вставку в локальный файл без сериализации всего YAML через `serde_yaml::Value`. Проверять предложенное объединение общим разбором и валидацией режима `tools download`, затем только затронутые настройки Vanessa; не включать полный Planned/full gate для чужих недостающих инструментов/исходников. Изменение `tools.va.epf_path` выполняется в той же подготовленной локальной версии и сохраняет пользовательское значение. Один локальный файл публикуется один раз, чтобы отказ на второй записи не оставил частично применённые настройки.
- [x] Повторить `cargo test --locked --test cli_tools_download vanessa_` и проверить разбор полученного YAML нынешним загрузчиком.
- [x] Обновить релевантные правила/docs/schema; commit `feat(tools): complete valid Vanessa defaults`.

### Задача 7: Ошибки очистки при ожидании процесса

**Файлы:** `src/platform/process.rs`, `src/use_cases/launch_app.rs`, `src/platform/agent.rs` и `src/use_cases/run_tests/{coordinator,helpers}.rs` только если требуется сохранить форму отказа; unit tests процесса и `tests/cli_launch.rs`.

**Интерфейс:** `ManagedSpawnResult::wait_for_exit` возвращает timeout только после подтверждённого завершения группы и успешного reap. Нынешний `terminate_child_group_gracefully` возвращает `()` и скрывает сбои; заменить его **в управляемом ожидании** на typed результат очистки с отдельными причинами terminate, проверки группы и `wait`, не теряя исходную ошибку `try_wait`. Снятие child из `self.child` не должно обходить cleanup при ошибке наблюдения. После неподтверждённого terminate не выполнять безграничный блокирующий `wait`: ограниченная попытка принудительного завершения и reap либо typed отказ. Windows `JobObject` и результат `taskkill /T /F` сверять как единый контракт группы; допустимую гонку «процесс уже вышел» отличать от оставшихся потомков. Отмена остаётся отменой при успешной очистке; сбой очистки сохраняется вместе с причиной отмены/timeout, а не выдаётся за их успешное завершение. Новый общий срок команды не вводится. `cancel`, `terminate`, `Drop` и очистка при startup остаются отдельным контрактом и не входят в это исправление.

- [x] Добавить тестовый seam для инъекции отказов наблюдения, завершения группы, проверки живости и ожидания; тестами закрепить сбой `try_wait`, ошибку terminate/`wait` по отдельности и вместе, успешную очистку после ошибки наблюдения, timeout и отмену. Проверять тип причины, а не текст сообщения; на Windows покрыть потомков `JobObject` и отказ `taskkill`, когда это доступно.
- [x] Запустить `cargo test --locked platform::process::tests::managed_wait_` и увидеть ожидаемый отказ новых проверок.
- [x] Перевести используемый `wait_for_exit` путь очистки с `terminate_child_group_gracefully` на typed helper, сохраняющий OS-ошибки `kill`/`taskkill`/`start_kill`, Unix `try_wait`/проверки группы и `child.wait`; при сбое `try_wait` пытаться очистить группу через тот же путь, затем подтвердить очистку до результата или вернуть обе причины.
- [x] Повторить `cargo test --locked platform::process::tests::managed_wait_` до зелёного результата.
- [x] Проверить `cargo test --locked --test cli_launch thin_external_epf_wait_` и формы `external_epf_wait.timed_out`, `execution.status`, exit code, `provider_dispatched` при сбое после старта; в `launch_app.rs` не классифицировать произвольную ошибку очистки как обычную отмену. Добавить типизированную классификацию нового `ProcessError` в `run_tests/helpers.rs` вместо debug-assert fallback. Проверить обработку того же результата при остановке сервиса агента в `platform/agent.rs` и у тестового сценария. При изменении wire обновить правило/схему. Commit `fix(process): preserve managed wait cleanup errors`.

### Задача 8: Windows-пути `init`

**Файлы:** `src/use_cases/config_init.rs`, при необходимости существующий `src/support/path.rs`; `tests/cli_config_init.rs`, `spec/rules/wire/config-init-data.md` при изменении формы.

**Интерфейс:** внутренние проверки идентичности используют канонический путь; путь в ответе и логические относительные source-set пути читаемы без `\\?\` и с `/` для относительных YAML значений.

- [x] Добавить Windows-тесты каталога и файла конфига с verbatim-представлением и переносимых относительных путей; Unix-тест сохраняет прежний вид. Запустить `cargo test --locked --test cli_config_init config_init_windows_path_`.
- [x] Использовать существующий нормализатор, если он подходит; не заменять канонический путь, по которому проверяется идентичность. Повторить targeted test и `cargo test --locked --test cli_config_init`.
- [x] Обновить документы/правило только при изменении публичного контракта; commit `fix(init): render portable Windows paths`.

### Задача 9: Наблюдаемость сканирования и воспроизводимые Windows-пробелы

**Файлы:** `src/change_detection/{analyzer,scanner}.rs` и их unit tests, релевантные Windows-проверки; `docs/DEEP_DIVE.md` только если меняется описание диагностики.

- [x] Сопоставить события нынешнего сканирования со спецификацией: начало, состояние, прогресс большого дерева, результат и причина fallback. Добавить проверки только отсутствующих событий, включая отсутствие секретов.
- [x] Разделить событие окончания `scanner::scan` и решение `analyze_context`/полного повторного сканирования: успех обхода ещё не означает успешный commit или выбор full/partial. Прогресс писать только счётчиками после порога и с ограниченной частотой, без имени файла, содержимого и дополнительного обхода. Проверить существующие fallback-предупреждения с `%e`, где встречаются абсолютные пути; для новых событий использовать безопасный код причины. Убедиться, что логирование не меняет cutoff, хеширование, fallback и commit.
- [x] Запустить `cargo test --locked change_detection::`, добавить недостающие `tracing`-события без изменения решения full/partial, повторить тест.
- [x] Пройти старые Windows-сценарии `c6d8a7a`, `0bf4b14`, `4f3598f` и `/C` из `96adc36`: зафиксировать имеющуюся проверку либо добавить тест только для воспроизводимого пробела. Для устранённой регрессии записать reintroduction guard. Commit только при реальном diff: `fix(changes): expose scan decisions`.

### Задача 10: Сводная проверка раннера и передача к миграции `v8-ci`

**Файлы:** только результаты проверки/релевантные документы и схемы; рабочий `v8-ci` не меняется.

- [x] Сверить фактический diff со всеми пунктами спецификации, правилами затронутых областей и текущими публичными документами; проверено отсутствие старого `builder`/общего `execution_timeout`, второго экспорта/парсера JUnit и лишнего слоя совместимости 0.5.5.
- [x] Выполнить `cargo fmt --all -- --check`, `cargo clippy --locked --bins -- -D warnings` и адресные тесты на Windows, включая Windows contract из `scripts/test/README.md`. Полный native `cargo test --locked` имеет известные платформенные hardcoded-path/CRLF сбои; Linux/macOS контур недоступен: WSL без Rust toolchain. Независимые tester/reviewer/Rust-expert passes проведены.
- [x] Проверить и перевести доступные live-помощники `ci-designer-config.sh`, `ci-rust.sh`, `live-cli-*` на текущие `providers.push`/`push`; Bash syntax и Windows contract прошли. Полный `ci-happy-path.sh` не запускался из-за отсутствия CI credentials.
- [x] На одноразовой копии `F:\1C\Projects\otus_JenkinsExample_EDT_codex_20260927\` убрать устаревшие `builder`/`execution_timeout`, назначить `providers` и `infobases.origin`; выполнены загрузка конфига, `push`, `check`, YaXUnit и Vanessa. Исходный проект не менялся; результаты и ограничения записаны в журнале.
- [x] Подготовлены входные данные отдельного плана `v8-ci` в `docs/superpowers/plans/2026-09-27-v8-ci-migration-inputs.md`: версия `0.5.5` → новая версия, `build` → `push`, strict JSON envelope, `builder` → `providers`, `--no-build` → `--no-push`, JUnit/Vanessa, local/Docker, `launch --wait-for-exit`, `make`, отмена и отказы. До принятия этого плана рабочий оркестратор остаётся на 0.5.5.

## Передача к исполнению

Сохранять отдельные commit там, где разделение предметных групп не мешает общему ревью. Обнаруженное несогласие правила и кода разбирать по `AGENTS.md`; изменение обязательства требует решения владельца. Этот план заканчивается проверенным раннером и материалами для отдельной миграции `v8-ci`.


