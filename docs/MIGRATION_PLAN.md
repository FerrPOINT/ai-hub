# План schema increments

Design dictionary не применяется напрямую; fresh/upgrade migration SQL создаётся
в backend/migrations вместе с corresponding application behavior.
Коммит документации не разрешает DB initialization или destructive rollback.

| Increment  | Scope                                                                                                     | Verification                                                                         |
| ---------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| DB-01 / S1 | installations/providers, clients/keys/grants, operations/audit                                            | Own bootstrap marker/key; identity + denied scopes; no defaults replacing lost state |
| DB-02 / S2 | connections/generations/credentials, catalog, probes/proofs/qualification, profiles/revisions/deployments | Nullable unknown limits, FK generation, draft CAS/proof invalidation                 |
| DB-03 / S3 | requests/attempts/replay/usage, prices/ledger/budgets/reservations                                        | intent-before-I/O, exact numeric38/18 vs rate30/12, dedupe/locks/restart             |
| DB-04 / S4 | login attempts, subscriptions/quota, receipt indexes                                                      | Native own checkpoint, duplicate charge/source ownership                             |
| DB-05 / S5 | aggregates, alert outbox, retention/archive indexes                                                       | Rebuild equivalence/watermark, privacy deletion preserving dedupe                    |
| DB-06 / S6 | dataset versions/evaluation runs/cases/leases                                                             | Fenced workers/unique case invocation, TTL/cancel/unknown                            |
| DB-07 / S7 | Only proven integration refinements                                                                       | Upgrade/rollback compatibility + nonempty restore                                    |

Composite FK/unique constraints из DD обязательно проверяются на PostgreSQL17.
CAS alone не защита amount/actor/source dedupe races. Price windows use exact
half-open nonoverlap per billing scope under transaction; extension choice tested
before relying on it. PK plus row lookup не должно позволять cross-profile active pointer.
If previous schema checksum differs, fail; never patch startup ledger SQL.
Downgrade binary only after verifying readable actual persisted schema; financial
facts and unknown reserves cannot be recreated as empty state.

## Дополнение 2026-10-09

DB-01/S1: namespace_bindings и nullable bindings клиентов. DB-02/S2: model_context_preferences с CAS. DB-03/S3: frozen request binding. DB-05/S5: notifications и actor ACK отдельно от alert_outbox delivery. DB-06/S6: dataset binding. DB-07/S7: legacy_import_provenance и точный import mapping. Applied SQL отсутствует; документация не запускает DDL.
