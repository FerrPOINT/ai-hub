# Миграции и совместимость

Canonical schema path: backend/migrations, SQLx versioned SQL migrations.
Project v1 начинает own schema; чужие Admin tables не переименовываются автоматически.
Порядок S1: installations/providers/clients/grants/audit; S2a вводит
connections/generations/prices/requests/attempts/usage/ledger/budgets/replay до probe,
S2b — profiles/catalog/publication, S3 — inference/tariff/service; S4 subscriptions,
S5 aggregates/outbox, S6 datasets/evaluations. Каждый stage имеет explicit migrations.

Применённые IDs/SQL bytes/checksums immutable; новая семантика — новая migration.
Поведение fresh install и upgrade с предыдущего accepted schema проверяется
на own disposable PostgreSQL. Нельзя чинить schema из import/handler/model.
Production admission выключен до successful explicit migrate/readiness.

Изменение ledger/config schema: expand → compatible reader/writer → verify
new state → switch; downgrade только если старый binary умеет читать актуальную schema.
Rollback binary не откатывает facts и не освобождает unknown reserves.
Миграция с irreversible data transformation требует own backup/restore rehearsal.
Actual migrations `0001`–`0006` применились на own disposable PostgreSQL17;
foundation и financial evidence отражены в IMPLEMENTATION_STATUS. Все новые
миграции квалифицировались только на disposable fixture, не на installed PDLC DB.
Upgrade/nonempty restore остаются S7 gates; их PASS не выводится из fresh install.
