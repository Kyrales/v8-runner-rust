# ConfigurationRepositoryOptimizeData

- Вид: `command`
- Раздел: Команды работы с хранилищем конфигурации
- Путь в оглавлении: Команды работы с хранилищем конфигурации -> Сервисные операции -> ConfigurationRepositoryOptimizeData
- Source pagePath: `zif3_configurationrepositoryoptimizedata`
- Source URL: `http://localhost:8080/ru/1%D0%A1%3A%D0%9F%D1%80%D0%B5%D0%B4%D0%BF%D1%80%D0%B8%D1%8F%D1%82%D0%B8%D0%B5?page=zif3_configurationrepositoryoptimizedata`

## Синтаксис

```text
/ConfigurationRepositoryOptimizeData [-Extension <имя расширения>]
```

## Нормализованное описание

— оптимизация базы данных хранилища конфигурации.

**-Extension <имя расширения>** — имя расширения. Если параметр не указан, выполняется попытка соединения с хранилищем основной конфигурации, и команда выполняется для основной конфигурации. Если параметр указан, выполняется попытка соединения с хранилищем указанного расширения, и команда выполняется для этого хранилища.
