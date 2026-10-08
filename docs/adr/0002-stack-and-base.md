# ADR-0002: Стек и воспроизводимость

Дата: 2026-10-08.
Статус: Принято.
Scope: AI Hub v1 documentation baseline; runtime/соседние repo не изменяются.

## Контекст

Пользователь выбрал технологии существующих сервисов Base; majority Rust/React
baseline подтверждён по SDK/product source.

## Решение

Rust 2024/Axum0.8/SQLx0.8/PostgreSQL17; React19/TS5.9/Vite6/Tailwind4/@sdlc/ui.
Build Rust1.88.0, Node22.20.0/pnpm10.28.1, exact Base SHA из .base-revision.
Actual lockfiles/manifests создаёт S1, не пустые заглушки в docs stage.

## Альтернативы

Полный LiteLLM Python service: быстрее reuse router, но другой стек и второй
владелец config/accounting. Java Agent foundation: runtime executor/tools scope
не соответствует lightweight gateway. Existing shared Base primitives выбраны.

## Последствия и проверка

Разные release toolchains во флоте допустимы; Admin Rust1.98.1/Node26.10.0 не
переносятся автоматически. Минимальные dependency версии подтверждает future gate.
README без runtime screenshots/CI/tech-proof badges; predev architecture table
показывает planned boundary вместо недоказанной runtime diagram.
[Требования](../PRODUCT_REQUIREMENTS.md), [traceability](../TRACEABILITY.md)
и [стадии](../IMPLEMENTATION_PLAN.md) связывают решение с будущими критериями.
