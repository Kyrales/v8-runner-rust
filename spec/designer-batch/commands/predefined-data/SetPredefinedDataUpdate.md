# SetPredefinedDataUpdate

- Вид: `command`
- Раздел: Предопределенные данные
- Путь в оглавлении: Предопределенные данные -> SetPredefinedDataUpdate
- Source pagePath: `zif3_setpredefineddataupdate`
- Source URL: `http://localhost:8080/ru/1%D0%A1%3A%D0%9F%D1%80%D0%B5%D0%B4%D0%BF%D1%80%D0%B8%D1%8F%D1%82%D0%B8%D0%B5?page=zif3_setpredefineddataupdate`

## Синтаксис

```text
/SetPredefinedDataUpdate [-Auto] [-UpdateAutomatically] [-DoNotUpdateAutomatically]
```

## Нормализованное описание

— предназначен для указания режимов обновления предопределенных данных. Значение параметра по умолчанию -**Auto**.

**Auto** — фактическое значение вычисляется автоматически. Для главного узла информационной базы - значение будет равно **UpdateAutomatically**, для периферийного узла информационной базы будет равно **DoNotUpdateAutomatically**.

**UpdateAutomatically **— при реструктуризации информационной базы будет выполняться автоматическое создание предопределенных элементов и обновление существующих значений.

**DoNotUpdateAutomatically **— при реструктуризации информационной базы не будет выполняться автоматическое создание новых предопределенных элементов и обновление их значений.
