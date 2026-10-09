# Изменения

Формат Keep a Changelog, будущие версии — SemVer.

## Unreleased

### Добавлено

- Предварительный контракт AI Hub: провайдеры, версии виртуальных моделей,
  совместимый inference API и воспроизводимые проверки.
- Правила учёта запросов, попыток, токенов, тарифов, подписок, расходов и бюджетов.
- Архитектура, доступ, модель данных, UI-сценарии и план вертикальных этапов.
- Критерии разработки/приёмки, трассировка требований и offline gate документации.

Runtime-реализации и выпущенной версии пока нет.

### Уточнено

- Полный UI design/prototype и source-bound IAB geometry/state/theme/flow evidence.
- Fields/access/errors/state/reconciliation, DD и migration/implementation packets.
- Отдельная выдача ключей, точный lookup, preserved draft/CAS, nullable fixed fees.

## Документация и дизайн — 2026-10-09

Добавлены Namespace, уведомления, per-model context, Admin field/storage mapping, потребительские примеры, совместимость и runbooks. Прототип сохраняет текущий стиль, моделирует новые действия и общий dataset. Backend/runtime/data не переносились.

## 2026-10-09 — Project tariffs

- Отдельный /tariffs design и contract: per-million input/output, default 20%, Namespace overrides.
- OpenRouter automatic catalog/usage cost и manual Ollama Online/ChatGPT subscription cost allocation.
- Immutable pricing-source/tariff/charge DTO и dataflow; runtime не реализован.
- Полный design handoff остаётся на актуализации после semantic audit; прошлый evidence исторический.

## 2026-10-09 — Semantic review closure

- Stable-ID Namespace binding, saved draft/proof/history and generation invalidation.
- Typed budget/context pagination/CAS, immutable price activation timeline, full profile/currency pricing.
- Dataset/run/manual score parity, canonical exports and full filter echo.
- Signed service adapter with exact v2 context/body binding and revoke tombstones; public SDK unchanged.
- Final evidence and current status refreshed after the coordinated gate; actual runtime not implemented.

## 2026-10-09 — Readiness-01–05

S2a financial primitives precede proof; truthful unconfigured pricing source, durable
scheduled tariff cancellation/CAS, canonical StatisticsFilters and protocol-discriminated
service DTO. OpenAPI 0.5.0-design. Fresh final documentation/design gate collected separately;
runtime/SQL/provider calls/main cutover remain outside this documentation delivery.
