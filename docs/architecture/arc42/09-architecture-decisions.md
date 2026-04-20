## 9. Архитектурные решения

Существующие ADR-файлы:

- [ADR-0001: Границы поддержки IBCMD как ограниченного backend](../../decisions/0001-granitsy-podderzhki-ibcmd-kak-ogranichennogo-backend.md)
- [ADR-0002: Изолировать runtime state по source-set под workPath](../../decisions/0002-izolirovat-runtime-state-po-source-set-pod-workpath.md)
- [ADR-0003: Поддерживать серверные ИБ для всех инструментов](../../decisions/0003-podderzhivat-servernye-ib-dlya-vseh-instrumentov.md)
- [ADR-0004: Автообнаруживать компоненты платформы 1С по версии-маске](../../decisions/0004-avtoobnaruzhivat-komponenty-platformy-1s-po-versii-maske.md)

Важные уже реализованные решения, которые сейчас зафиксированы кодом и внутренними архитектурными заметками:

- транспортно-нейтральные контракты use case, общие для CLI и MCP;
- отдельные платформенные адаптеры для Designer, Enterprise, IBCMD и EDT;
- централизованный поиск компонентов платформы 1С по версии или версии-маске;
- общий интерактивный EDT actor ограничен MCP EDT syntax, а не всеми EDT-операциями;
- CLI и MCP intentionally expose different public surfaces: MCP не зеркалит CLI полностью;
- текущая поддержка `builder=IBCMD` ограничена файловыми ИБ, но целевой контракт требует server infobase support для всех инструментов;
- сохранённое инкрементальное состояние хранится в per-source-set `redb` contexts под `workPath`;
- presentation concerns (`Presenter`, `Envelope`, text formatting) остаются вне use case.

Рекомендуемое развитие:

- фиксировать эти решения в явных ADR, когда они меняются или когда добавляются новые backend/transport;
- следующим кандидатом на formal ADR является различие между CLI-only и MCP-published operations.
