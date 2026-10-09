# ADR-0009: готовность контракта перед реализацией

Дата: 2026-10-09. Статус: принято для документации/design v1.

## Решение

1. S2a canonical financial primitives предшествуют S2b proof; общий engine переиспользуется S3.
2. Отсутствующий pricing source — guarded nullable FK и explicit unconfigured, без fake IDs/rates.
3. Scheduled cancellation — append-only own fact/CAS, без worker и переписывания snapshots.
4. StatisticsFilters одна на query/echo/export/cursor; protocol discriminates полный service body.

## Альтернативы

Поздний ledger в S3 или отдельный mock probe bypass ломает порядок и учёт. Fake price/source
для null случая создаёт ложное provenance. UPDATE/delete activation теряет audit/replay.
Отдельные filter DTO и permissive service union оставляют drift generated clients.
Выбран existing domain/DB/schema boundary; новых сервисов и runtime frameworks нет.

## Последствия и проверка

check_readiness, negative mutation/schema fixtures, PostgreSQL prerequisites при реализации
и IAB cancellation/unknown/group/timezone flows. Код, реальный provider, shared SDK pins и
cutover не изменяются. [Пакеты](../IMPLEMENTATION_PACKETS.md), [API](../API.md),
[тарифы](../PROJECT_TARIFFS.md), [accounting](../ACCOUNTING.md) — canonical owners.
