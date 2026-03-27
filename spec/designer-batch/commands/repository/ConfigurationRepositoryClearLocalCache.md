# ConfigurationRepositoryClearLocalCache

- Вид: `command`
- Раздел: Команды работы с хранилищем конфигурации
- Путь в оглавлении: Команды работы с хранилищем конфигурации -> Операции с кешем хранилища -> ConfigurationRepositoryClearLocalCache
- Source pagePath: `zif3_configurationrepositoryclearlocalcache`
- Source URL: `http://localhost:8080/ru/1%D0%A1%3A%D0%9F%D1%80%D0%B5%D0%B4%D0%BF%D1%80%D0%B8%D1%8F%D1%82%D0%B8%D0%B5?page=zif3_configurationrepositoryclearlocalcache`

## Синтаксис

```text
/ConfigurationRepositoryClearLocalCache [-Extension <имя расширения>]
```

## Нормализованное описание

- очистка локального кэша версий конфигурации

**-Extension <имя расширения>** — Имя расширения. Если параметр не указан, выполняется попытка соединения с хранилищем основной конфигурации, и команда выполняется для основной конфигурации. Если параметр указан, выполняется попытка соединения с хранилищем указанного расширения, и команда выполняется для этого хранилища.

**Пример для конфигурации, не присоединенной к текущему хранилищу:**

DESIGNER /F "D:\V8\Cfgs8\ИБ8" /ConfigurationRepositoryF "D:\V8\Cfgs8" /ConfigurationRepositoryN "Администратор" /ConfigurationRepositoryP xxx /ConfigurationRepositoryClearLocalCache

**Пример для конфигурации, присоединенной к хранилищу конфигурации:**

DESIGNER /F "D:\V8\Cfgs8\ИБ8" /ConfigurationRepositoryClearLocalCache
