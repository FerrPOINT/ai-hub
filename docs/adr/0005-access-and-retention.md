# ADR-0005: Доступ, own credentials и retention

Дата: 2026-10-08.
Статус: Принято.
Scope: AI Hub v1 documentation baseline; runtime/соседние repo не изменяются.

## Контекст

Существующая платформа имеет central identity и browser policy; сервис должен
сохранять независимый AI boundary и не раскрывать transcripts/secrets.

## Решение

Base central active browser policy без второй системы global roles;
PAT exact Hub scopes, inference own scoped client key/project grants.
Own encrypted credential state/key, audited generation and revocation.
[SECURITY](../SECURITY.md) defines TTL: no regular transcript analytics,
replay<=24h/dedupe30d, evaluation7d, metadata90d/ledger365d with archival.

## Альтернативы

Shared vault/global Codex auth снижает настройку, но смешивает authority и rollback.
Auth local users дублируют Central Auth. Бесконечное хранение requests для analytics
не нужно: canonical usage/financial metadata достаточно.

## Последствия и проверка

Current session validation обязательна, service scopes integrate at S1.
Financial retention не license/юридическое обещание; operator policy задаёт archive.
Удостоверение user/client/project проверяется server-side, metadata untrusted.
[Требования](../PRODUCT_REQUIREMENTS.md), [traceability](../TRACEABILITY.md)
и [стадии](../IMPLEMENTATION_PLAN.md) связывают решение с будущими критериями.
