# ADR-0004: Канонический financial ledger

Дата: 2026-10-08.
Статус: Принято.
Scope: AI Hub v1 documentation baseline; runtime/соседние repo не изменяются.

## Контекст

Пользователь требует статистику и расходы; token-cost estimate и subscription
платежи нельзя смешать с реальным списанием.

## Решение

Append-only per-attempt usage/ledger, immutable price snapshots, decimal amounts
per currency, confirmed/estimated/unknown. Atomic multi-scope reservations до I/O,
unknown holds и late settlement dedupe. [ACCOUNTING](../ACCOUNTING.md) — rule owner.
Statistics projection reconstructible; UI не financial authority.

## Альтернативы

Суммы SDK/прокси без provenance быстро дают card, но не billing evidence.
In-place totals не восстанавливают corrections/replay. External billing engine —
слишком широкий v1 и не нужен для own provider expenses.

## Последствия и проверка

New schema/recovery/real PG tests обязательны. Цена не из code constants.
Фиксированная subscription payment один раз; allocated cost отдельная estimate.
Mixed currencies не один total. Hard monetary budget requires qualified upper bound.
[Требования](../PRODUCT_REQUIREMENTS.md), [traceability](../TRACEABILITY.md)
и [стадии](../IMPLEMENTATION_PLAN.md) связывают решение с будущими критериями.

## Дополнение 2026-10-09

По прямому поручению владельца добавлены project tariffs: 1M input/output, default 20% markup, exact Namespace overrides. Project-charge journal отдельный от provider cash. [PROJECT_TARIFFS](../PROJECT_TARIFFS.md) определяет auto/manual, units, decimal scale и snapshots; область v1 остаётся внутренним учётом без payment execution.
