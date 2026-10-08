# ADR-0006: Адаптеры и evidence

Дата: 2026-10-08.
Статус: Принято.
Scope: AI Hub v1 documentation baseline; runtime/соседние repo не изменяются.

## Контекст

OpenAI-compatible label не гарантирует identical tools/Responses/context/usage
или отсутствие hidden retries, особенно для subscription transports.

## Решение

Native Rust provider ports normalize typed protocol/usage, preserve caller semantics,
return acceptance certainty/actual model provenance. Live qualification per exact
account/model/generation/adapter/environment; metadata не proof.
Probe/eval проходятся через frozen own request admission/ledger после S3.

## Альтернативы

Использовать Octo как непрозрачный router everywhere: быстрее старт, но unknown
attempt/billing/model chain не даёт pinned/budget guarantees.
Переписать все providers заранее — ненужный scope; квалифицировать по stage.

## Последствия и проверка

Adapters только заявленные capabilities; unsupported fields fail до I/O.
Managed subscription отдельный transport/authorization, not invented API key.
[PROVIDERS](../PROVIDERS.md) и [INFERENCE](../contracts/INFERENCE_V1.md) обязательны.
[Требования](../PRODUCT_REQUIREMENTS.md), [traceability](../TRACEABILITY.md)
и [стадии](../IMPLEMENTATION_PLAN.md) связывают решение с будущими критериями.
