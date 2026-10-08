# Пакеты реализации для разработчика

Все components ниже — target files; код ещё не создан. Source authority:
requirements/OpenAPI/DD/design; routine coding не требует повторной discovery.

| Stage | First working vertical path                                     | Required deliverables                                                                                                        | Gate / dependencies                                                      |
| ----- | --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| S1    | Authenticated own shell + health/schema + trusted client config | Actual Cargo/pnpm locks, explicit env/initializer, own migrations, Base bridge, scopes/grants/audit, generated OpenAPI/types | Source/SDK/frozen/real PG/SSO; no real provider needed                   |
| S2    | Connection → draft → controlled proof → revision                | Typed CRUD/CAS/operation ledger, generations, no-secret responses, native design forms                                       | S1; actual controlled HTTP proof, not catalog-only                       |
| S3    | Client key → full/stream/tools → usage/reserve/settlement       | Admission context, adapter pool, intent/unknown/cancel, rate/fee snapshot, atomic budgets, own result buffer                 | S2; 2 processes/cross-scope budgets/SSE/recovery/partial delivery        |
| S4    | Each required provider and own subscription                     | Own native auth state, account/endpoint qualification, trusted receipt import, subscription/quota                            | S3; actual accounts and bounded explicit budget, no global auth import   |
| S5    | Canonical stats/expense → breakdown/export/alerts               | Same authorized snapshot, labels/bases/counts, rebuild/TTL, in-app outbox, all operational screens                           | S3+S4; UI states/themes/mobile, actual ledger sums                       |
| S6    | Dataset/pinned revisions → comparison/cancel                    | Frozen config/scorer/tool schemas, fenced worker, per-case requests/expense                                                  | S4+S5; synthetic then qualified models, incomparable outcome             |
| S7    | One real consumer + restore/rollback/release                    | Exact source/Base/image/target receipt, own nonempty restore, compatibility/cutover                                          | S6; all FR/NFR/TC plus live/served identity, no status-only substitution |

## File plan

backend/crates/api: handlers/DTO/OpenAPI export; application: own use cases/context;
domain: transitions/Money/Scope/Revision; infrastructure: SQLx/pooled adapters/vault.
backend/migrations: immutable stage changes; frontend: app/routes/shared API+
feature/page composition by design. deploy: explicit isolated source/config/images.
No generated client type hand edits, SQL in handlers, global preferences for
request-scoped fields, or copied provider policy in pages.

## Checklist on each packet

- Schema/DTO/source constraints ready before dependent call sites.
- Positive/negative/neighboring-case feedback loop crosses actual public boundary.
- Current design spec/fields/states mapped to operation IDs; implementation may not
  silently remove case/limit/actor/cost evidence.
- Focused fast checks per slice; one integrated heavy milestone gate.
- Before delivery: minimality/owner review, clean source, exact source evidence.
- Current UI design screenshots are prototype evidence; replace with actual served
  app receipts at UI implementation checkpoints.
