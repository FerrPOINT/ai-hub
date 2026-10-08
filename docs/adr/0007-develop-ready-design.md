# ADR-0007: полный дизайн и development handoff

Дата: 2026-10-08. Статус: принято для AI Hub v1.
Scope: документация/design; application runtime не реализуется этим решением.

## Контекст

Первый baseline имел route map и typed API, но не rendered design. Пользователь
попросил полный дизайн и весь спектр документов до development-ready.
Структурная полнота сама не доказывает читаемый UI и отсутствие payload gaps.

## Решение

Хранить self-contained interactive prototype в docs/design, Base token snapshot
по exact .base-revision, design contract с15 routes/forms/states/API mapping.
Проверять финальный source hash через Codex in-app browser; сохранять geometry/
theme/state/keyboard evidence и изображения с kind prototype.
Дополнить typed DD, access/error/state/use-case/migration/implementation packets.
Production frontend позже реализует @sdlc/ui primitives, не копирует prototype app.

## Альтернативы и последствия

Source-only route table отклонён: не находит overflow/focus/payload/lifetime defects.
Production scaffolding сейчас отклонён: задача про readiness до реализации.
Новая внешняя design system/Figma destination не нужна для данного repository handoff.
Design acceptance не actual SSO/provider/cost/security acceptance. Complete handoff
получает DEVELOP_READY только после real rendered evidence, validator и publication.
