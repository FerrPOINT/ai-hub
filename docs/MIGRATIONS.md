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
Actual migrations `0001`–`0011` применились на own disposable PostgreSQL17;
foundation и financial evidence отражены в IMPLEMENTATION_STATUS. Все новые
миграции квалифицировались только на disposable fixture, не на installed PDLC DB.
Upgrade/nonempty restore остаются S7 gates; их PASS не выводится из fresh install.

`0007` включает Namespace binding в budget uniqueness, сохраняет существующие
policy IDs/балансы и запрещает изменение identity. Calendar periods и CAS
изменения cap проверены control fixture отдельно от финансового settlement.

`0008` замораживает wire mode новых requests и добавляет immutable result receipts
с original expiry. Legacy ciphertext не переписывается: новые FK/size constraints
enforce новые writes через `NOT VALID`, а старый content без qualified receipt не
выдаётся новым reader. Bounded TTL purge сохраняет финансовые facts и receipt
dedupe. Nonempty upgrade/constraint validation остаются отдельным S7 gate.

`0009` вводит immutable bounded queue deadline новых intents. Старый deadline
не угадывается: legacy unclaimed state допускает uncertainty recovery, но не
новый dispatch claim. Это не terminal no-billing witness и не release reserve.

`0010` добавляет stable source policies, immutable revisions и structural catalog
origin с own FKs. Current tier хранится в connection generation, legacy tier
остаётся unknown. Eligible catalog view не считает human quote labels evidence.
Financial snapshot FK/consumer guard присоединён `0011`; fresh SQL
fixture не означает квалифицированную nonempty migration или live catalog.

`0011` фиксирует source revision и policy version с own FK/immutable trigger.
Legacy missing version не backfill-ится выдуманным current state; новый dispatch
требует qualified source snapshot. Existing settlement history не переписывается.
