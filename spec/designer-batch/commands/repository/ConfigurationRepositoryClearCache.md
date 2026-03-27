# ConfigurationRepositoryClearCache

- Вид: `command`
- Раздел: Команды работы с хранилищем конфигурации
- Путь в оглавлении: Команды работы с хранилищем конфигурации -> Операции с кешем хранилища -> ConfigurationRepositoryClearCache
- Source pagePath: `zif3_configurationrepositoryclearcache`
- Source URL: `http://localhost:8080/ru/1%D0%A1%3A%D0%9F%D1%80%D0%B5%D0%B4%D0%BF%D1%80%D0%B8%D1%8F%D1%82%D0%B8%D0%B5?page=zif3_configurationrepositoryclearcache`

## Синтаксис

```text
/ConfigurationRepositoryClearCache [-Extension <имя расширения>]
```

## Нормализованное описание

— очистка локальной базы данных хранилища конфигурации.

**-Extension <имя расширения>** — Имя расширения. Если параметр не указан, выполняется попытка соединения с хранилищем основной конфигурации, и команда выполняется для основной конфигурации. Если параметр указан, выполняется попытка соединения с хранилищем указанного расширения, и команда выполняется для этого хранилища.
