# ADR-0001: Выделенный AI Hub и владельцы

Дата: 2026-10-08.
Статус: Принято для проектного baseline AI Hub; existing Admin contract не superseded.
Scope: AI Hub v1 documentation baseline; runtime/соседние repo не изменяются.

## Контекст

Нужны standalone gateway, виртуальные модели, dev/test и статистика/расходы.
Сегодня AI foundation находится в Admin repo, inference не принят.

## Решение

AI Hub владеет AI config/revisions/credentials/inference/accounting/evaluations.
Admin сохраняет branding/service catalog, Central Auth — identity, Fleet — runtime/tools.
[ADMIN_HANDOFF](../contracts/ADMIN_HANDOFF_V1.md) описывает отдельную owner migration;
документирование не переводит existing state или consumer endpoints.

## Альтернативы

Завершить Admin runtime вместо выделения: меньше repo, но смешивает platform
governance и AI data-plane/financial domain. Отдельный UI поверх Octo: не даёт
единого trusted grant/attempt/ledger boundary. Выбран самостоятельный продукт.

## Последствия и проверка

Own storage/vault/client grants и contract integration обязательны.
Existing Admin ADR-0018 сохраняет authority своего runtime до accepted cutover.
[Требования](../PRODUCT_REQUIREMENTS.md), [traceability](../TRACEABILITY.md)
и [стадии](../IMPLEMENTATION_PLAN.md) связывают решение с будущими критериями.
