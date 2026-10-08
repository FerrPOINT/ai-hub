# ADR-0003: Immutable profiles и explicit fallback

Дата: 2026-10-08.
Статус: Принято.
Scope: AI Hub v1 documentation baseline; runtime/соседние repo не изменяются.

## Контекст

Нужно стабильное имя модели с provider fallback для разработки и воспроизводимость
для тестов, не скрытая подмена итоговой модели.

## Решение

Virtual slug с immutable revision; development explicit ordered fallback,
pinned_test exact one deployment/no cache/retry. Admission freezes snapshot.
[ROUTING](../ROUTING.md) defines bounds/certainty/cooldown.
После content/tool output или ambiguous acceptance никаких new inference attempts.

## Альтернативы

Random/cheapest auto routing снижает некоторые цены, но ухудшает воспроизводимость.
Blind retries увеличивают риск paid duplicates. Fixed upstream-only для всего
продукта исключает нужный dev failover; сохранён отдельно pinned mode.

## Последствия и проверка

Each attempt accounted, actual model confidence visible. Provider hidden
retry/substitution требует qualification, не best-effort assumption.
[Требования](../PRODUCT_REQUIREMENTS.md), [traceability](../TRACEABILITY.md)
и [стадии](../IMPLEMENTATION_PLAN.md) связывают решение с будущими критериями.
