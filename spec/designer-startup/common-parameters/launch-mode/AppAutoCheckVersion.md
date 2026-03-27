# AppAutoCheckVersion

- Вид: `parameter`
- Группа: Определение режима запуска
- Source pagePath: `zif2_appautocheckversion`
- Source URL: `http://localhost:8080/ru/1%D0%A1%3A%D0%9F%D1%80%D0%B5%D0%B4%D0%BF%D1%80%D0%B8%D1%8F%D1%82%D0%B8%D0%B5?page=zif2_appautocheckversion`

## Синтаксис

```text
/AppAutoCheckVersion [+/-]
```

## Нормализованное описание

— выполняет OpenID logout (завершение сеанса работы пользователя). Завершение сеанса работы выполняется вне зависимости от используемого в дальнейшем метода аутентификации.

**/AppAutoCheckVersion-** — автоматический подбор версии платформы не выполняется.

**/AppAutoCheckVersion+** — автоматический подбор версии платформы выполняется выполняется для каждой базы (по умолчанию).
