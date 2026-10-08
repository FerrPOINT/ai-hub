# Миграции и совместимость

Canonical future schema path: backend/migrations, SQLx versioned SQL migrations.
Project v1 начинает own schema; чужие Admin tables не переименовываются автоматически.
Порядок S1: installations/providers/connections/profiles/clients/audit,
затем S3 requests/attempts/usage/prices/ledger/budgets, S4 subscriptions,
S5 aggregates/outbox, S6 datasets/evaluations. Каждый stage имеет explicit migrations.

Применённые IDs/SQL bytes/checksums immutable; новая семантика — новая migration.
Поведение fresh install и upgrade с предыдущего accepted schema проверяется
на own disposable PostgreSQL. Нельзя чинить schema из import/handler/model.
Production admission выключен до successful explicit migrate/readiness.

Изменение ledger/config schema: expand → compatible reader/writer → verify
new state → switch; downgrade только если старый binary умеет читать актуальную schema.
Rollback binary не откатывает facts и не освобождает unknown reserves.
Миграция с irreversible data transformation требует own backup/restore rehearsal.
Документы сейчас не создают migrations и не подтверждают их выполнение.
