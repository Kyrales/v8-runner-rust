# SimulateServerCallDelay

- Вид: `parameter`
- Группа: Настройки отладки
- Source pagePath: `zif2_SimulateServerCallDelay`
- Source URL: `http://localhost:8080/ru/1%D0%A1%3A%D0%9F%D1%80%D0%B5%D0%B4%D0%BF%D1%80%D0%B8%D1%8F%D1%82%D0%B8%D0%B5?page=zif2_SimulateServerCallDelay`

## Синтаксис

```text
/SimulateServerCallDelay [-CallXXXXX] [-SendYYYYY] [-ReceiveZZZZZ]
```

## Нормализованное описание

— имитация работы клиента в условиях медленного соединения.

**-Call** — указывает величину задержки (XXXXX) при вызове сервера в секундах, если не указан, то 4,45 сек;
**-Send** — указывает величину задержки (YYYYY) в секундах в расчете на каждые 1 Кбайт данных, отправляемых на сервер. Если не указан, то 0,45 сек;
**-Receive** — указывает величину задержки (ZZZZZ) в секундах в расчете на каждые 1 Кбайт данных, принятых с  сервера. Если не указан, то 0,15 сек.

Максимальное значение временных задержек — 10 сек.

**Пример**:
/SimulateServerCallDelay -Call2.1 -Send1.3 -Receive1.2
