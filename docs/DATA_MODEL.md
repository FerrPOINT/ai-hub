# Модель данных AI Hub

Проектная схема PostgreSQL 17; SQL migrations пока не созданы.
UUID v4 IDs, timestamptz UTC; financial amounts NUMERIC(38,18), rates NUMERIC(30,12),
units явные. Input precision проверяется до DB cast, implicit rounding запрещено.
External project/subject IDs — opaque strings с provenance, без cross-service FK.

## Таблицы

| Таблица                | Существенные поля                                                                                                        | Ограничение                                                       |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------- |
| installations          | id, stable_key, created_at                                                                                               | stable_key unique, immutable                                      |
| providers              | id, kind, display_name, endpoint_policy                                                                                  | kind allowlist, installation FK                                   |
| connections            | id, provider_id, generation, status, adapter_revision                                                                    | generation monotonic; no hard delete referenced                   |
| credential_versions    | connection_id, generation, ciphertext, nonce, key_id, status                                                             | unique connection/generation; ciphertext private                  |
| upstream_models        | connection_id, generation, provider_model_id, metadata, observed_at                                                      | unique exact model/generation; limits nullable                    |
| verification_evidence  | id, connection_generation, model, adapter_revision, capabilities, expires_at, receipt_digest                             | exact binding; no caller-owned proof                              |
| probe_snapshots        | id, scope, model/draft_version/config_hash, generation/deployments, bounds                                               | immutable; no published profile prerequisite                      |
| runtime_qualifications | connection/generation/model/adapter/endpoint, bounds/capabilities, proof_id, state                                       | active/invalidated; publication TTL independent                   |
| virtual_models         | id, installation_id, slug, display_name, draft_version, active_revision, status                                          | unique installation/slug; pointer only own model                  |
| profile_revisions      | id, virtual_model_id, sequence, config_hash, params, bounds, mode, created_by                                            | unique model/sequence; immutable                                  |
| profile_deployments    | revision_id, ordinal, connection_id, generation, model_id, verification_id                                               | unique ordinal; complete frozen snapshot                          |
| clients                | id, installation_id, application_key, project_binding, status                                                            | application key unique, trusted project provenance                |
| client_keys            | id, client_id, hash, prefix, expiry, revoked_at                                                                          | hash unique, raw key absent                                       |
| grants                 | client/principal/project, action, virtual_model_id, bounds, expiry                                                       | exact subject/binding; no wildcard by default                     |
| requests               | id, client_id, principal, project, kind, revision_id/probe_snapshot_id, idempotency_key, payload_hmac, state, timestamps | exactly one snapshot by kind; unique installation/client/key; CAS |
| replay_payloads        | request_id, encrypted_response, key_id, expires_at                                                                       | optional <=24h; separate restricted access                        |
| attempts               | id, request_id, ordinal, deployment_snapshot, dispatch_intent_at, state, accepted, provider_request_id                   | unique request/ordinal; no accepted guess                         |
| usage_facts            | id, attempt_id, source, source_event_id, categories, provenance, observed_at                                             | unique source/event; category knownness explicit                  |
| price_revisions        | id, account/model/tier, currency, rates, effective_from/to, source                                                       | nonoverlapping scope interval; immutable                          |
| ledger_entries         | id, charge_key, kind, amount, currency, confidence, original_entry_id, source_event_id, occurred_at                      | unique source/event; append-only; correction linked               |
| budget_policies        | id, scope_type/id, currency, period, hard_limit, thresholds, version                                                     | unique active scope/currency; nonnegative hard bound              |
| budget_periods         | policy_id, period_start/end, charged, reserved, version                                                                  | unique policy/start; row lock admission                           |
| reservations           | attempt_id, budget_period_id, amount, currency, state                                                                    | unique attempt/budget; no release on unknown                      |
| subscriptions          | id, connection_id, period_start/end, amount, currency, receipt/status                                                    | unique payment source/id; no per-request cash copy                |
| quota_observations     | connection/generation, unit, limit/remaining, source, observed_at, reset_at                                              | values nullable, not inferred from token count                    |
| dataset_versions       | dataset_id, version, cases_hash, content_ref, owner/project, expires_at                                                  | immutable version; content separate TTL                           |
| evaluation_runs        | id, config_snapshot, dataset_version, grant_binding, state, lease_generation, cursor                                     | stable per-run config; worker fencing                             |
| evaluation_cases       | run_id, candidate/repetition/case, request_id, score/provenance                                                          | unique run/candidate/repetition/case                              |
| aggregate_buckets      | scope/filter_dimensions, time_bucket, counters, knownness, watermark                                                     | derived, rebuildable, no money authority                          |
| audit_events           | id, actor, action, object/revision, operation_id, reason, happened_at                                                    | append-only, no secret body                                       |
| alert_outbox           | id, event_key, scope, channel, state, attempts                                                                           | event_key unique, bounded delivery retry                          |

## State transitions

Requests: admitted → dispatching → streaming → completed/failed/cancelled/unknown.
Без stream допустим dispatching → completed. Unknown → completed/failed/cancelled
только с trusted reconciliation, никогда назад к dispatching.
Rejected admission — отдельный bounded diagnostic event, не admitted request.
Attempts: intended → dispatched → streaming → terminal или unknown.
Ledger/reservation transition принадлежит той же application operation; manual SQL repair запрещён.

## Constraints и индексы

Requests (client_id, admitted_at DESC, id), (project, admitted_at,id),
(revision_id,admitted_at); attempts (request_id,ordinal), (state,updated_at).
Ledger (currency,occurred_at,id), (charge_key,id), unique (source,source_event_id).
Evidence (connection_id,generation,model_id,expires_at); audit (object,id)/(actor,time).
Budget lock order installation → project → client → profile, затем stable UUID.
Не создавать JOIN в чужую продуктовую БД ради красивого display_name.

Immutable revisions/facts append-only enforce store permissions и application checks;
post-deploy indexes измеряются на representative data, не добавляются на каждое поле.
Account balances/aggregate cache не правятся отдельно от ledger transaction.
Retention удаляет только разрешённые content/projections; financial dedupe identity
и external receipt ссылки проходят documented archival/restore, не orphan delete.

## Полный typed design

DATA_DICTIONARY и contracts/data-dictionary.v1.json задают актуальные таблицы и поля (счётчики проверяются по JSON),
operation/login ledgers и immutable connection generations. Wire DTO — OpenAPI.
Operations safe_result не содержит one-time keys/model result; FK/proof/lifecycle
и stage increments перечислены в MIGRATION_PLAN/STATE_MACHINES.

## Дополнение Namespace и выделения

Namespace projection, model context preferences, notifications/actor acknowledgements и legacy import provenance описаны в [DATA_DICTIONARY](DATA_DICTIONARY.md). External IDs не FK к чужой БД. Null binding явно unbound. Сохранённый context budget не verification evidence.

## Цены проектов

Pricing source revisions и project tariff revisions отдельны от provider price revisions. Admission сохраняет project_tariff_snapshot; project_charge_events не заменяют provider ledger. [Контракт](PROJECT_TARIFFS.md); typed dictionary содержит поля, собственные FK и ключи dedupe.

## Новые owner records

project_tariff_policies и project_tariff_activations задают стабильный CAS timeline отдельно от immutable revisions. evaluation_manual_scores содержит own score witness/actor/version. service_delegations хранит verified context, fence и revoke tombstones без prompt/provider secrets. Все отношения own DB; wire в OpenAPI 0.4.0-design.
