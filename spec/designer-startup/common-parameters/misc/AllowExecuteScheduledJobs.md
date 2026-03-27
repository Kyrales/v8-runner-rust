# AllowExecuteScheduledJobs

- Вид: `parameter`
- Группа: Прочие параметры
- Source pagePath: `zif2_AllowExecuteScheduledJobs`
- Source URL: `http://localhost:8080/ru/1%D0%A1%3A%D0%9F%D1%80%D0%B5%D0%B4%D0%BF%D1%80%D0%B8%D1%8F%D1%82%D0%B8%D0%B5?page=zif2_AllowExecuteScheduledJobs`

## Синтаксис

```text
/AllowExecuteScheduledJobs -Off|-Force
```

## Нормализованное описание

— управление запуском регламентных заданий. Регламентные задания начинают выполняться на первом запущенном по порядку клиенте, у которого не **AllowExecuteScheduledJobs –Off**. После завершения сеанса этого клиента, выполнение переходит к какому-либо из других запущенных сеансов. Если запускается сеанс с **AllowExecuteScheduledJobs –Force**, то регламентные задания начинают выполняться на нем, не зависимо от наличия других сеансов.
