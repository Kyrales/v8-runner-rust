# Конфигурация

Этот документ описывает все поддержанные ключи `application.yaml`, их текущий статус и ограничения реализации.

Цель документа:

- дать единое место со всеми настройками;
- отделить реально работающие настройки от задела на будущее;
- явно ответить на вопросы про интерактивный EDT и дополнительные параметры запуска клиента 1С.

## Полный пример

```yaml
basePath: /path/to/project
workPath: /tmp/v8-test-runner/project
format: EDT
builder: DESIGNER
connection: "File=/path/to/ib"

credentials:
  user: Admin
  password: secret

source-set:
  - name: main
    purpose: CONFIGURATION
    path: main
  - name: ext
    purpose: EXTENSION
    path: ext

build:
  partialLoadThreshold: 20

tools:
  platform:
    path: /opt/1cv8/x86_64
    version: 8.3.27.1859
  edt-cli:
    path: 2025.2.3
    version: 2025.2.3
    auto-start: false
    startup-timeout-ms: 300000
    command-timeout-ms: 300000

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

tests:
  execution_timeout_seconds: 300
```

## Обязательные ключи

### `basePath`

- Тип: путь
- Обязателен: да
- Значение: корень исходников проекта

Поведение:

- должен существовать и быть каталогом.

### `workPath`

- Тип: путь
- Обязателен: да
- Значение: рабочий каталог для временных файлов, логов, hash storage и EDT workspace

Поведение:

- будет создан автоматически, если отсутствует;
- используется как корень для:
  - `workPath/hash-storages`
  - `workPath/logs`
  - `workPath/temp`
  - `workPath/edt-workspace`
  - `workPath/designer`

### `connection`

- Тип: строка
- Обязателен: да

Поведение:

- передаётся в платформенные утилиты как строка подключения;
- для `builder=IBCMD` должна указывать на файловую ИБ.

### `source-set`

- Тип: список
- Обязателен: да

Каждый элемент:

- `name`: логическое имя набора исходников
- `purpose`: `CONFIGURATION` или `EXTENSION`
- `path`: путь к исходникам

Поведение:

- должен быть хотя бы один `CONFIGURATION`;
- `name` должен быть уникальным;
- для `format=EDT` путь должен существовать;
- для `format=EDT` generated Designer copy идёт в `workPath/designer/<name>`.

## Базовые режимы

### `format`

- Тип: enum
- Значения: `DESIGNER`, `EDT`
- По умолчанию: `DESIGNER`

### `builder`

- Тип: enum
- Значения: `DESIGNER`, `IBCMD`
- По умолчанию: `DESIGNER`

Ограничения:

- `format=EDT` сейчас требует `builder=DESIGNER`.

## Опциональные секции

### `credentials`

- `credentials.user`
- `credentials.password`

Используются как логин/пароль для подключения к ИБ.

### `build`

- `build.partialLoadThreshold`
- Тип: integer
- По умолчанию: `20`
- Минимум: `1`

Используется для решения между partial и full load.

### `tests`

- `tests.execution_timeout_seconds`
- Тип: integer
- По умолчанию: `300`
- Допустимый диапазон: `1..=86400`

### `mcp.http`

- `bind_address`: адрес HTTP listener, по умолчанию `127.0.0.1:3000`
- `path`: HTTP path, по умолчанию `/mcp`
- `stateful_sessions`: `true` по умолчанию
- `max_sessions`: `64` по умолчанию
- `idle_ttl_secs`: `900` по умолчанию

### `mcp.execution`

- `max_concurrent_calls`: по умолчанию `1`
- `shutdown_grace_period_secs`: по умолчанию `30`

## `tools.platform`

### `tools.platform.path`

- Тип: путь
- Обязателен: нет

Может указывать:

- на конкретный бинарь `1cv8`, `1cv8c` или `ibcmd`;
- на каталог `bin`;
- на корень установки с версиями.

### `tools.platform.version`

- Тип: строка
- Обязателен: нет
- Формат: строго `major.minor.patch.build`

Пример:

```yaml
tools:
  platform:
    version: 8.3.27.1859
```

Если `path` не указан, будет идти автопоиск по стандартным корням установки.

## `tools.edt_cli`

### `tools.edt_cli.path`

- Тип: путь или version-like hint
- Обязателен: нет

Поддержанные варианты:

- абсолютный путь к `1cedtcli`;
- путь к каталогу установки EDT;
- version-like hint, например `2025.2.3`.

Пример:

```yaml
tools:
  edt-cli:
    path: 2025.2.3
```

Это находит установленный EDT вида `1c-edt-2025.2.3+30-x86_64`.

### `tools.edt_cli.version`

- Тип: строка
- Обязателен: нет

Отдельная version-like подсказка для автопоиска EDT.

Пример:

```yaml
tools:
  edt-cli:
    version: 2025.2.3
```

### `tools.edt_cli.startup_timeout_ms`

- Тип: integer
- По умолчанию: `300000`
- Также принимает: `startup-timeout-ms`

Используется при старте интерактивной EDT session и ожидании prompt.

### `tools.edt_cli.command_timeout_ms`

- Тип: integer
- По умолчанию: `300000`
- Также принимает: `command-timeout-ms`

Используется как timeout для EDT-команд в MCP `check_syntax_edt`.

### `tools.edt_cli.auto-start`

- Тип: boolean
- По умолчанию: `false`

Текущий статус:

- ключ читается конфигом;
- явного отдельного поведения от него сейчас нет;
- shared interactive EDT session поднимается лениво при первом реальном EDT MCP-вызове.

То есть на текущем этапе `auto-start` скорее зарезервирован, чем полноценно влияет на runtime.

### `tools.edt_cli.working-directory`

Текущий статус:

- не поддержан моделью конфигурации;
- будет проигнорирован как неизвестный ключ YAML;
- рабочий каталог EDT session сейчас фиксирован: `workPath/edt-workspace`.

## Интерактивный EDT: что реально работает

Сейчас интерактивный режим `1cedtcli` используется только для MCP-инструмента `check_syntax_edt`.

Реально поддержано:

- автопоиск `1cedtcli`;
- ленивый старт shared session;
- timeout старта через `tools.edt_cli.startup_timeout_ms`;
- timeout команды через `tools.edt_cli.command_timeout_ms`;
- workspace в `workPath/edt-workspace`.

Пока не поддержано как отдельная настраиваемая функция:

- явный prewarm через `auto-start`;
- произвольный `working-directory`;
- дополнительные аргументы для старта `1cedtcli` сверх `-data <workPath/edt-workspace>`.

## Запуск клиента 1С: что реально поддержано

Команда `launch` сейчас поддерживает только выбор режима:

- `designer`
- `thin`
- `thick`

Внутри формируется запуск:

- `designer` -> `1cv8 DESIGNER`
- `thin` -> `1cv8c ENTERPRISE`
- `thick` -> `1cv8 ENTERPRISE`

Дополнительно автоматически передаются только:

- аргументы из `connection`;
- `credentials.user/password`, если они заданы.

### Дополнительные параметры клиента 1С

Текущий статус:

- отдельной конфигурации для проброса дополнительных CLI-параметров в `launch` сейчас нет;
- в `application.yaml` нет поддержанных ключей вида `launch.args`, `launch.additional-args`, `tools.platform.client-args` и т.п.;
- MCP `launch_app` тоже принимает только тип запуска, без дополнительного набора аргументов.

Если нужен запуск с чем-то вроде:

- `/RunModeOrdinaryApplication`
- `/UsePrivilegedMode`
- `/C <payload>`
- `/Execute <epf>`
- `/DisableStartupDialogs`

то это сейчас потребует доработки use case и конфигурационной модели.

## Что стоит помнить

- `docs/CAPABILITIES.md` описывает пользовательские возможности и матрицу сценариев.
- Этот файл описывает именно конфигурацию и её текущие runtime-ограничения.
