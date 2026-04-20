# Архитектурные решения (ADR)

Этот каталог хранит архитектурные решения проекта в формате ADR.

## Индекс

- [ADR-0001: Границы поддержки IBCMD как ограниченного backend](0001-granitsy-podderzhki-ibcmd-kak-ogranichennogo-backend.md) — `accepted`, `2026-04-02`
- [ADR-0002: Изолировать runtime state по source-set под workPath](0002-izolirovat-runtime-state-po-source-set-pod-workpath.md) — `accepted`, `2026-04-20`
- [ADR-0003: Поддерживать серверные ИБ для всех инструментов](0003-podderzhivat-servernye-ib-dlya-vseh-instrumentov.md) — `accepted`, `2026-04-20`
- [ADR-0004: Автообнаруживать компоненты платформы 1С по версии-маске](0004-avtoobnaruzhivat-komponenty-platformy-1s-po-versii-maske.md) — `accepted`, `2026-04-20`

## Правила обновления

- Для изменений архитектурных ограничений добавляйте новый ADR или обновляйте существующий с явным указанием статуса.
- При обновлении публичного контракта синхронизируйте связанные документы (`README.md`, `docs/CAPABILITIES.md`, `docs/DEEP_DIVE.md`, `docs/GIT_WORKFLOW.md`, `ARCHITECTURE.md`).
