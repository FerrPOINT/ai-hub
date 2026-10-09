# План schema increments

Design dictionary не применяется напрямую; fresh/upgrade migration SQL создаётся
в backend/migrations вместе с corresponding application behavior.
Коммит документации не разрешает DB initialization или destructive rollback.

| Increment  | Scope                                                                                                                       | Verification                                                                                              |
| ---------- | --------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| DB-01 / S1 | installations/providers, clients/keys/grants, operations/audit                                                              | Own bootstrap marker/key; identity + denied scopes; no defaults replacing lost state                      |
| DB-02 / S2 | S2a early request/attempt/usage/price/ledger/budget/replay; S2b connections/generations/credentials/catalog/probes/profiles | Financial primitives precede proof, no future FK; unknown limits, intent/reserve/settlement and draft CAS |
| DB-03 / S3 | project tariff/policy/activation/cancellation/charge records, service delegation; public inference extensions of S2 schema  | Exact numeric38/18 vs project50/24, cross-scope locks/restart, no duplicate financial engine              |
| DB-04 / S4 | login attempts, subscriptions/quota, receipt indexes                                                                        | Native own checkpoint, duplicate charge/source ownership                                                  |
| DB-05 / S5 | aggregates, alert outbox, retention/archive indexes                                                                         | Rebuild equivalence/watermark, privacy deletion preserving dedupe                                         |
| DB-06 / S6 | dataset versions/evaluation runs/cases/leases                                                                               | Fenced workers/unique case invocation, TTL/cancel/unknown                                                 |
| DB-07 / S7 | Only proven integration refinements                                                                                         | Upgrade/rollback compatibility + nonempty restore                                                         |

Composite FK/unique constraints из DD обязательно проверяются на PostgreSQL17.
CAS alone не защита amount/actor/source dedupe races. Effective source binding intervals use exact
half-open nonoverlap per billing scope under transaction; raw immutable quote allowed
windows may overlap. Unique source/time and stable CAS derive the effective timeline;
no exclusion constraint that rejects every future quote over an older open quote. PK plus row lookup не должно позволять cross-profile active pointer.
If previous schema checksum differs, fail; never patch startup ledger SQL.
Downgrade binary only after verifying readable actual persisted schema; financial
facts and unknown reserves cannot be recreated as empty state.

## Дополнение 2026-10-09

DB-01/S1: namespace_bindings и nullable bindings клиентов. DB-02/S2: model_context_preferences с CAS. DB-03/S3: frozen request binding. DB-05/S5: notifications и actor ACK отдельно от alert_outbox delivery. DB-06/S6: dataset binding. DB-07/S7: legacy_import_provenance и точный import mapping. Applied SQL отсутствует; документация не запускает DDL.

## Дополнение тарифов

S2: pricing_source_revisions. S3: project_tariff_revisions, requests.project_tariff_snapshot и project_charge_events. S5: projection/control UI. Own FK, nonoverlap, CAS/dedupe и decimal scale требуют настоящих SQL migrations при реализации; ручного переноса legacy aggregate в выдуманные project charges нет.

## Semantic review increments

S3: stable project tariff policies/activation timelines и service delegations/tombstones. S6: evaluation_manual_scores. DB constraints: unique policy/time, policy-version CAS, run/candidate/case/repetition score version, issuer/grant tombstone, same-owner FK. Source SDK pins не меняются; legacy grants/ordinal IDs не конвертируются автоматически.

## READY-01–03: проверяемая граница

Dictionary.phase — первая миграция таблицы, не момент полной FR acceptance.
Каждый own FK ссылается на same/earlier stage. S2a financial core вводится до первого
verification intent; S2b proof не зависит от S3 tables. S3 cancellation witness ссылается
на own activation/policy и S1 operation. Current activation state derived, не UPDATE
canonical activation. CHECK source state/FK/null confidence и same-policy cancellation
проверяются реальным PostgreSQL gate при реализации. Applied SQL сейчас отсутствует.
