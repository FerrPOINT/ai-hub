# Словарь данных AI Hub

Проектная схема; SQL migrations/runtime не реализованы. Canonical typed source: [data-dictionary](contracts/data-dictionary.v1.json).
47 таблиц / 420 полей. Phase — первая schema introduction; S2a prerequisites предшествуют S2b proof.

## installations — S1

| Поле           | Тип         | Nullable | Правило               |
| -------------- | ----------- | -------- | --------------------- |
| id             | uuid        | нет      | PK                    |
| stable_key     | text        | нет      | unique                |
| status         | text        | нет      | active or blocked     |
| blocked_reason | text        | да       | safe admission reason |
| created_at     | timestamptz | нет      | UTC                   |

## providers — S1

| Поле            | Тип          | Nullable | Правило           |
| --------------- | ------------ | -------- | ----------------- |
| id              | uuid         | нет      | PK                |
| installation_id | uuid         | нет      | installations.id  |
| kind            | text         | нет      | adapter allowlist |
| display_name    | varchar(120) | нет      | presentation only |
| created_at      | timestamptz  | нет      | UTC               |

## connections — S2

| Поле                | Тип          | Nullable | Правило                                        |
| ------------------- | ------------ | -------- | ---------------------------------------------- |
| id                  | uuid         | нет      | PK                                             |
| provider_id         | uuid         | нет      | providers.id                                   |
| display_name        | varchar(120) | нет      | user label                                     |
| endpoint_policy_ref | text         | нет      | deployment allowlist                           |
| billing_mode        | text         | нет      | metered subscription local unknown             |
| generation          | bigint       | нет      | monotonic >=1                                  |
| status              | text         | нет      | enabled disabled revoked authorization_unknown |
| version             | bigint       | нет      | CAS >=1                                        |

## connection_generations — S2

| Поле                 | Тип         | Nullable | Правило                                |
| -------------------- | ----------- | -------- | -------------------------------------- |
| connection_id        | uuid        | нет      | composite PK connections.id            |
| generation           | bigint      | нет      | composite PK >=1                       |
| authorization_state  | text        | нет      | absent prepared active revoked unknown |
| adapter_revision     | text        | нет      | exact adapter                          |
| endpoint_policy_hash | text        | нет      | immutable snapshot                     |
| created_at           | timestamptz | нет      | UTC                                    |

## credential_versions — S2

| Поле          | Тип         | Nullable | Правило                        |
| ------------- | ----------- | -------- | ------------------------------ |
| connection_id | uuid        | нет      | connection_generations binding |
| generation    | bigint      | нет      | same generation composite PK   |
| ciphertext    | bytea       | нет      | encrypted no raw secret        |
| nonce         | bytea       | нет      | unique 12-byte AES-GCM nonce   |
| key_id        | text        | нет      | separate key escrow reference  |
| state         | text        | нет      | prepared active revoked        |
| created_at    | timestamptz | нет      | UTC                            |

## upstream_models — S2

| Поле              | Тип         | Nullable | Правило                 |
| ----------------- | ----------- | -------- | ----------------------- |
| id                | uuid        | нет      | PK                      |
| connection_id     | uuid        | нет      | exact connection        |
| generation        | bigint      | нет      | exact generation        |
| provider_model_id | text        | нет      | exact case-sensitive ID |
| input_limit       | bigint      | да       | unknown not default     |
| output_limit      | bigint      | да       | unknown not default     |
| metadata          | jsonb       | нет      | safe allowlisted fields |
| observed_at       | timestamptz | нет      | catalog freshness       |

## operations — S1

| Поле            | Тип         | Nullable | Правило                                    |
| --------------- | ----------- | -------- | ------------------------------------------ |
| id              | uuid        | нет      | PK                                         |
| installation_id | uuid        | нет      | own installation                           |
| principal_kind  | text        | нет      | human client internal                      |
| principal_id    | text        | нет      | trusted subject                            |
| idempotency_key | uuid        | нет      | caller stable intent                       |
| binding_hmac    | bytea       | нет      | method path payload ETag principal         |
| action          | text        | нет      | allowlisted operation                      |
| resource_id     | uuid        | да       | own target                                 |
| state           | text        | нет      | pending succeeded failed unknown cancelled |
| safe_result     | jsonb       | да       | no raw credentials or one-time secret      |
| created_at      | timestamptz | нет      | UTC                                        |
| updated_at      | timestamptz | нет      | UTC                                        |
| expires_at      | timestamptz | нет      | 30-day control dedupe                      |

## login_attempts — S4

| Поле                 | Тип         | Nullable | Правило                                            |
| -------------------- | ----------- | -------- | -------------------------------------------------- |
| id                   | uuid        | нет      | PK operation reference                             |
| connection_id        | uuid        | нет      | exact target                                       |
| generation           | bigint      | нет      | fenced authorization                               |
| state                | text        | нет      | pending completed expired cancelled failed unknown |
| encrypted_checkpoint | bytea       | да       | own vault-only projection                          |
| key_id               | text        | да       | checkpoint key reference                           |
| expires_at           | timestamptz | нет      | provider device login deadline                     |

## probe_snapshots — S2

| Поле           | Тип         | Nullable | Правило                                       |
| -------------- | ----------- | -------- | --------------------------------------------- |
| id             | uuid        | нет      | PK                                            |
| operation_id   | uuid        | нет      | own operation                                 |
| scope          | text        | нет      | connection_model or profile_draft             |
| config_hash    | text        | нет      | frozen shape                                  |
| draft_model_id | uuid        | да       | only profile scope                            |
| draft_version  | bigint      | да       | only profile scope                            |
| configuration  | jsonb       | нет      | secret-free exact targets bounds capabilities |
| created_at     | timestamptz | нет      | UTC                                           |

## verification_evidence — S2

| Поле              | Тип         | Nullable | Правило                         |
| ----------------- | ----------- | -------- | ------------------------------- |
| id                | uuid        | нет      | PK                              |
| probe_snapshot_id | uuid        | нет      | probe snapshot FK               |
| state             | text        | нет      | pending verified failed expired |
| child_evidence    | jsonb       | нет      | ordered exact proof IDs         |
| receipt_digest    | text        | да       | trusted terminal witness        |
| expires_at        | timestamptz | нет      | publication freshness only      |
| created_at        | timestamptz | нет      | UTC                             |

## runtime_qualifications — S2

| Поле                    | Тип     | Nullable | Правило                                                                                                |
| ----------------------- | ------- | -------- | ------------------------------------------------------------------------------------------------------ |
| id                      | uuid    | нет      | PK                                                                                                     |
| connection_id           | uuid    | нет      | exact account                                                                                          |
| generation              | bigint  | нет      | immutable generation                                                                                   |
| provider_model_id       | text    | нет      | exact model                                                                                            |
| adapter_revision        | text    | нет      | exact code                                                                                             |
| endpoint_policy_hash    | text    | нет      | scope                                                                                                  |
| proof_id                | uuid    | нет      | trusted evidence                                                                                       |
| capabilities            | jsonb   | нет      | qualified set and bounds                                                                               |
| state                   | text    | нет      | active invalidated                                                                                     |
| invalidated_reason      | text    | да       | safe reason                                                                                            |
| billing_currency        | char(3) | да       | qualified currency from account statement/receipt; null unqualified, never inferred from provider name |
| billing_currency_origin | text    | да       | trusted statement/receipt witness linked through proof_id; required iff currency known                 |

## virtual_models — S2

| Поле               | Тип          | Nullable | Правило                        |
| ------------------ | ------------ | -------- | ------------------------------ |
| id                 | uuid         | нет      | PK                             |
| installation_id    | uuid         | нет      | own installation               |
| slug               | varchar(63)  | нет      | unique immutable API name      |
| display_name       | varchar(120) | нет      | presentation                   |
| draft              | jsonb        | нет      | validated ProfileInput         |
| draft_version      | bigint       | нет      | CAS >=1                        |
| active_revision_id | uuid         | да       | own profile revision only      |
| status             | text         | нет      | draft active disabled archived |

## profile_revisions — S2

| Поле                    | Тип         | Nullable | Правило                        |
| ----------------------- | ----------- | -------- | ------------------------------ |
| id                      | uuid        | нет      | PK                             |
| virtual_model_id        | uuid        | нет      | owner FK                       |
| sequence                | bigint      | нет      | unique per model               |
| configuration           | jsonb       | нет      | immutable params bounds policy |
| config_hash             | text        | нет      | exact normalized bytes         |
| profile_verification_id | uuid        | нет      | exact draft proof              |
| created_by              | text        | нет      | trusted actor                  |
| created_at              | timestamptz | нет      | UTC                            |

## profile_deployments — S2

| Поле                | Тип     | Nullable | Правило                                        |
| ------------------- | ------- | -------- | ---------------------------------------------- |
| revision_id         | uuid    | нет      | composite PK profile_revisions.id              |
| ordinal             | integer | нет      | ordered composite PK >=1                       |
| connection_id       | uuid    | нет      | exact connection                               |
| generation          | bigint  | нет      | frozen immutable generation                    |
| model_id            | text    | нет      | exact provider ID                              |
| verification_id     | uuid    | нет      | child witness                                  |
| deployment_snapshot | jsonb   | нет      | adapter endpoint capabilities bounds no secret |

## clients — S1

| Поле                 | Тип          | Nullable | Правило                                            |
| -------------------- | ------------ | -------- | -------------------------------------------------- |
| id                   | uuid         | нет      | PK                                                 |
| installation_id      | uuid         | нет      | own scope                                          |
| application_key      | varchar(120) | нет      | unique per installation                            |
| project_binding      | text         | нет      | trusted control mapping                            |
| allowed_profiles     | jsonb        | нет      | bounded UUID allowlist                             |
| scopes               | jsonb        | нет      | infer read_own_usage read_result                   |
| cost_policy          | text         | нет      | budget_guaranteed or explicit cost_unknown_allowed |
| expires_at           | timestamptz  | нет      | UTC                                                |
| max_concurrency      | integer      | нет      | 1..100 and own global cap                          |
| max_rpm              | integer      | нет      | 1..10000                                           |
| status               | text         | нет      | enabled disabled revoked                           |
| version              | bigint       | нет      | CAS                                                |
| namespace_binding_id | uuid         | да       | own verified binding FK; null explicitly unbound   |

## client_keys — S1

| Поле       | Тип         | Nullable | Правило                 |
| ---------- | ----------- | -------- | ----------------------- |
| id         | uuid        | нет      | PK                      |
| client_id  | uuid        | нет      | clients.id              |
| key_hash   | bytea       | нет      | unique no plaintext     |
| prefix     | text        | нет      | safe display identifier |
| expires_at | timestamptz | нет      | UTC                     |
| revoked_at | timestamptz | да       | revoke effective        |

## grants — S1

| Поле             | Тип         | Nullable | Правило                        |
| ---------------- | ----------- | -------- | ------------------------------ |
| id               | uuid        | нет      | PK                             |
| installation_id  | uuid        | нет      | scope                          |
| principal_id     | text        | нет      | trusted identity not body user |
| client_id        | uuid        | да       | machine binding                |
| project_binding  | text        | нет      | exact allowed project          |
| action           | text        | нет      | allowlisted permission         |
| virtual_model_id | uuid        | да       | exact model for infer          |
| bounds           | jsonb       | нет      | purpose/caps                   |
| expires_at       | timestamptz | нет      | validity                       |
| revoked_at       | timestamptz | да       | deny new access                |

## requests — S2

| Поле                    | Тип         | Nullable | Правило                                                                               |
| ----------------------- | ----------- | -------- | ------------------------------------------------------------------------------------- |
| id                      | uuid        | нет      | PK                                                                                    |
| installation_id         | uuid        | нет      | own scope                                                                             |
| client_id               | uuid        | нет      | own normal/internal client                                                            |
| principal_id            | text        | нет      | trusted actor                                                                         |
| project_binding         | text        | нет      | frozen grant                                                                          |
| request_kind            | text        | нет      | inference verification evaluation                                                     |
| profile_revision_id     | uuid        | да       | required non-verification                                                             |
| probe_snapshot_id       | uuid        | да       | required verification                                                                 |
| idempotency_key         | uuid        | нет      | client key or generated own ID                                                        |
| payload_hmac            | bytea       | нет      | semantic intent binding                                                               |
| state                   | text        | нет      | admitted dispatching streaming completed failed cancelled unknown                     |
| cancel_requested        | boolean     | нет      | default false separate from terminal                                                  |
| admitted_at             | timestamptz | нет      | UTC                                                                                   |
| finished_at             | timestamptz | да       | transport terminal                                                                    |
| version                 | bigint      | нет      | CAS fence                                                                             |
| namespace_binding_id    | uuid        | да       | own verified binding FK; null explicitly unbound                                      |
| project_tariff_snapshot | jsonb       | да       | immutable tariff/source/default 2000 bps and exact Namespace; null explicitly unbound |

## replay_payloads — S2

| Поле               | Тип         | Nullable | Правило                     |
| ------------------ | ----------- | -------- | --------------------------- |
| request_id         | uuid        | нет      | PK own request              |
| encrypted_response | bytea       | нет      | own result buffer only      |
| key_id             | text        | нет      | separate own encryption key |
| expires_at         | timestamptz | нет      | <=24h                       |
| protocol           | text        | нет      | chat_completions responses  |

## attempts — S2

| Поле                  | Тип         | Nullable | Правило                                                          |
| --------------------- | ----------- | -------- | ---------------------------------------------------------------- |
| id                    | uuid        | нет      | PK                                                               |
| request_id            | uuid        | нет      | requests.id                                                      |
| ordinal               | integer     | нет      | unique request ordinal                                           |
| deployment_snapshot   | jsonb       | нет      | frozen exact scope                                               |
| state                 | text        | нет      | intended dispatched streaming completed failed cancelled unknown |
| accepted              | text        | нет      | not_accepted accepted unknown                                    |
| dispatch_intent_at    | timestamptz | нет      | durable before I/O                                               |
| dispatched_at         | timestamptz | да       | sent boundary                                                    |
| first_content_at      | timestamptz | да       | TTFT                                                             |
| finished_at           | timestamptz | да       | terminal witness                                                 |
| provider_request_id   | text        | да       | own readback                                                     |
| actual_model          | text        | да       | not guessed                                                      |
| actual_model_verified | boolean     | нет      | default false                                                    |

## usage_facts — S2

| Поле            | Тип         | Nullable | Правило                                 |
| --------------- | ----------- | -------- | --------------------------------------- |
| id              | uuid        | нет      | PK                                      |
| attempt_id      | uuid        | нет      | attempts.id                             |
| source          | text        | нет      | trusted ingestion kind                  |
| source_event_id | text        | нет      | unique within source installation       |
| categories      | jsonb       | нет      | disjoint known/unknown token categories |
| provenance      | jsonb       | нет      | adapter receipt/category inclusion      |
| observed_at     | timestamptz | нет      | UTC                                     |

## price_revisions — S2

| Поле            | Тип            | Nullable | Правило                            |
| --------------- | -------------- | -------- | ---------------------------------- |
| id              | uuid           | нет      | PK                                 |
| connection_id   | uuid           | нет      | own account                        |
| model_id        | text           | нет      | exact upstream                     |
| tier            | text           | нет      | billing tier                       |
| currency        | char(3)        | нет      | supported explicit currency        |
| input_uncached  | numeric(30,12) | нет      | nonnegative per-million            |
| input_cached    | numeric(30,12) | да       | unknown not zero                   |
| output_billable | numeric(30,12) | нет      | nonnegative per-million            |
| request_fee     | numeric(38,18) | да       | known fixed per-attempt or unknown |
| effective_from  | timestamptz    | нет      | inclusive                          |
| effective_to    | timestamptz    | да       | exclusive > from                   |
| source          | text           | нет      | price provenance                   |

## ledger_entries — S2

| Поле              | Тип            | Nullable | Правило                                               |
| ----------------- | -------------- | -------- | ----------------------------------------------------- |
| id                | uuid           | нет      | PK                                                    |
| installation_id   | uuid           | нет      | own scope                                             |
| attempt_id        | uuid           | да       | required invocation charge                            |
| charge_key        | text           | нет      | stable logical charge identity                        |
| kind              | text           | нет      | reserve release charge correction subscription_charge |
| amount            | numeric(38,18) | да       | null only unknown charge                              |
| currency          | char(3)        | нет      | explicit                                              |
| confidence        | text           | нет      | confirmed estimated unknown                           |
| original_entry_id | uuid           | да       | correction/settlement reference                       |
| source            | text           | нет      | trusted origin                                        |
| source_event_id   | text           | нет      | dedupe                                                |
| price_revision_id | uuid           | да       | estimate provenance                                   |
| occurred_at       | timestamptz    | нет      | cash basis                                            |

## budget_policies — S2

| Поле                 | Тип            | Nullable | Правило                                          |
| -------------------- | -------------- | -------- | ------------------------------------------------ |
| id                   | uuid           | нет      | PK                                               |
| installation_id      | uuid           | нет      | scope                                            |
| scope_type           | text           | нет      | installation project client profile              |
| scope_id             | text           | нет      | exact target                                     |
| currency             | char(3)        | нет      | no mixed currencies                              |
| period               | text           | нет      | utc_day utc_month                                |
| hard_limit           | numeric(38,18) | нет      | >=0                                              |
| thresholds           | jsonb          | нет      | ascending unique 1..99                           |
| version              | bigint         | нет      | CAS                                              |
| status               | text           | нет      | active disabled                                  |
| namespace_binding_id | uuid           | да       | own verified binding FK; null explicitly unbound |

## budget_periods — S2

| Поле         | Тип            | Nullable | Правило                     |
| ------------ | -------------- | -------- | --------------------------- |
| id           | uuid           | нет      | PK                          |
| policy_id    | uuid           | нет      | budget_policies.id          |
| period_start | timestamptz    | нет      | UTC exact boundary          |
| period_end   | timestamptz    | нет      | exclusive                   |
| charged      | numeric(38,18) | нет      | canonical effective charges |
| reserved     | numeric(38,18) | нет      | held >=0                    |
| version      | bigint         | нет      | row lock/CAS                |

## reservations — S2

| Поле             | Тип            | Nullable | Правило               |
| ---------------- | -------------- | -------- | --------------------- |
| attempt_id       | uuid           | нет      | composite PK          |
| budget_period_id | uuid           | нет      | composite PK          |
| amount           | numeric(38,18) | нет      | bounded >=0           |
| currency         | char(3)        | нет      | same policy           |
| state            | text           | нет      | held released settled |
| created_at       | timestamptz    | нет      | before dispatch       |

## subscriptions — S4

| Поле             | Тип            | Nullable | Правило                      |
| ---------------- | -------------- | -------- | ---------------------------- |
| id               | uuid           | нет      | PK                           |
| connection_id    | uuid           | нет      | own account                  |
| period_start     | timestamptz    | нет      | service period inclusive     |
| period_end       | timestamptz    | нет      | service period exclusive     |
| amount           | numeric(38,18) | нет      | >=0                          |
| currency         | char(3)        | нет      | explicit                     |
| payment_status   | text           | нет      | planned pending paid unknown |
| receipt_ref      | text           | да       | trust importer reference     |
| source_event_id  | text           | да       | dedupe trusted payment       |
| cash_occurred_at | timestamptz    | да       | known paid charge            |
| confidence       | text           | нет      | confirmed estimated unknown  |

## quota_observations — S4

| Поле          | Тип         | Nullable | Правило             |
| ------------- | ----------- | -------- | ------------------- |
| id            | uuid        | нет      | PK                  |
| connection_id | uuid        | нет      | exact account       |
| generation    | bigint      | нет      | exact scope         |
| unit          | text        | нет      | provider quota unit |
| limit         | bigint      | да       | known nonnegative   |
| remaining     | bigint      | да       | known nonnegative   |
| source        | text        | нет      | provider witness    |
| observed_at   | timestamptz | нет      | freshness           |
| reset_at      | timestamptz | да       | known reset         |

## dataset_versions — S6

| Поле                 | Тип         | Nullable | Правило                                          |
| -------------------- | ----------- | -------- | ------------------------------------------------ |
| id                   | uuid        | нет      | PK                                               |
| dataset_id           | uuid        | нет      | stable dataset                                   |
| sequence             | bigint      | нет      | unique per dataset                               |
| project_binding      | text        | нет      | owner scope                                      |
| created_by           | text        | нет      | trusted actor                                    |
| cases_hash           | text        | нет      | immutable exact dataset                          |
| case_count           | integer     | нет      | 1..1000                                          |
| content_ref          | text        | нет      | own encrypted artifact store                     |
| expires_at           | timestamptz | нет      | 7d content default                               |
| namespace_binding_id | uuid        | да       | own verified binding FK; null explicitly unbound |

## evaluation_runs — S6

| Поле               | Тип         | Nullable | Правило                                   |
| ------------------ | ----------- | -------- | ----------------------------------------- |
| id                 | uuid        | нет      | PK                                        |
| dataset_version_id | uuid        | нет      | exact version                             |
| project_binding    | text        | нет      | own scope                                 |
| config_snapshot    | jsonb       | нет      | profiles scorer params repetitions        |
| budget_policy_id   | uuid        | нет      | applicable scope                          |
| state              | text        | нет      | queued running completed failed cancelled |
| lease_generation   | bigint      | нет      | fencing                                   |
| lease_expires_at   | timestamptz | да       | worker lease                              |
| cancel_requested   | boolean     | нет      | default false                             |
| created_at         | timestamptz | нет      | UTC                                       |

## evaluation_cases — S6

| Поле                  | Тип     | Nullable | Правило                                        |
| --------------------- | ------- | -------- | ---------------------------------------------- |
| run_id                | uuid    | нет      | composite unique                               |
| candidate_revision_id | uuid    | нет      | frozen exact profile                           |
| repetition            | integer | нет      | 1..10                                          |
| case_id               | text    | нет      | dataset case                                   |
| request_id            | uuid    | нет      | unique dispatch binding                        |
| outcome               | text    | нет      | pass fail invalid incomparable skipped unknown |
| score                 | jsonb   | да       | scorer witness                                 |
| artifact_ref          | text    | да       | scoped TTL content                             |

## aggregate_buckets — S5

| Поле               | Тип         | Nullable | Правило                      |
| ------------------ | ----------- | -------- | ---------------------------- |
| id                 | uuid        | нет      | PK                           |
| filter_hash        | text        | нет      | authorized grouping identity |
| time_bucket        | timestamptz | нет      | explicit basis               |
| dimension_snapshot | jsonb       | нет      | bounded group                |
| known_counts       | jsonb       | нет      | no fake unknown zeros        |
| totals             | jsonb       | нет      | per currency confidence      |
| watermark          | timestamptz | нет      | rebuild provenance           |

## audit_events — S1

| Поле                 | Тип         | Nullable | Правило                                          |
| -------------------- | ----------- | -------- | ------------------------------------------------ |
| id                   | uuid        | нет      | PK                                               |
| installation_id      | uuid        | нет      | own scope                                        |
| actor                | text        | нет      | trusted subject                                  |
| action               | text        | нет      | allowlist                                        |
| object_id            | text        | нет      | safe identity                                    |
| revision             | text        | да       | exact revision                                   |
| operation_id         | uuid        | нет      | own operation                                    |
| reason               | text        | да       | safe text no secret                              |
| happened_at          | timestamptz | нет      | UTC                                              |
| namespace_binding_id | uuid        | да       | own verified binding FK; null explicitly unbound |

## alert_outbox — S5

| Поле           | Тип         | Nullable | Правило                            |
| -------------- | ----------- | -------- | ---------------------------------- |
| id             | uuid        | нет      | PK                                 |
| event_key      | text        | нет      | unique scope period threshold      |
| scope_snapshot | jsonb       | нет      | own target                         |
| kind           | text        | нет      | budget unknown projection provider |
| state          | text        | нет      | pending delivered failed           |
| safe_payload   | jsonb       | нет      | no secrets/content                 |
| created_at     | timestamptz | нет      | UTC                                |
| delivered_at   | timestamptz | да       | in-app ack                         |

## namespace_bindings — S1

| Поле                 | Тип         | Nullable | Правило                                          |
| -------------------- | ----------- | -------- | ------------------------------------------------ |
| id                   | uuid        | нет      | PK                                               |
| installation_id      | uuid        | нет      | own installation FK                              |
| registry_instance_id | uuid        | нет      | external Admin identity; nonnil                  |
| namespace_id         | uuid        | нет      | external immutable ID; no lookup by display name |
| tracker_instance_id  | uuid        | нет      | external verified instance                       |
| tracker_project_id   | uuid        | нет      | external verified project                        |
| state                | text        | нет      | active archived unavailable; no inferred active  |
| generation           | bigint      | нет      | positive verified projection generation          |
| observed_at          | timestamptz | нет      | UTC last verified readback                       |

## model_context_preferences — S2

| Поле                  | Тип         | Nullable | Правило                                             |
| --------------------- | ----------- | -------- | --------------------------------------------------- |
| connection_id         | uuid        | нет      | own connection FK                                   |
| model_id              | text        | нет      | exact upstream ID, 1..256; no case/name normalization |
| context_window_tokens | bigint      | нет      | configured budget 1..4294967295; not physical bound |
| version               | bigint      | нет      | CAS                                                 |
| updated_at            | timestamptz | нет      | UTC                                                 |

## notifications — S5

| Поле                 | Тип         | Nullable | Правило                                 |
| -------------------- | ----------- | -------- | --------------------------------------- |
| id                   | uuid        | нет      | PK                                      |
| installation_id      | uuid        | нет      | own installation FK                     |
| namespace_binding_id | uuid        | да       | own binding FK; null installation event |
| event_key            | text        | нет      | unique stable domain event key          |
| kind                 | text        | нет      | typed notification category             |
| severity             | text        | нет      | info warning error                      |
| safe_payload         | jsonb       | нет      | no keys/prompt/raw upstream error       |
| object_kind          | text        | нет      | allowlisted owner resource kind         |
| object_id            | uuid        | да       | own safe resource reference             |
| occurred_at          | timestamptz | нет      | event time UTC                          |

## notification_acknowledgements — S5

| Поле            | Тип         | Nullable | Правило                        |
| --------------- | ----------- | -------- | ------------------------------ |
| notification_id | uuid        | нет      | own notification FK            |
| actor_subject   | text        | нет      | verified central subject       |
| idempotency_key | uuid        | нет      | stable actor intent            |
| acknowledged_at | timestamptz | нет      | UTC; unique notification/actor |

## legacy_import_provenance — S7

| Поле               | Тип  | Nullable | Правило                                                     |
| ------------------ | ---- | -------- | ----------------------------------------------------------- |
| id                 | uuid | нет      | PK                                                          |
| source_instance_id | uuid | нет      | explicit source                                             |
| source_workspace   | text | нет      | not a Namespace                                             |
| source_kind        | text | нет      | allowlisted ledger/profile/preference/operation             |
| source_record_id   | text | нет      | immutable source identity                                   |
| source_digest      | text | нет      | SHA256                                                      |
| target_kind        | text | нет      | own typed kind                                              |
| target_id          | uuid | нет      | own identity                                                |
| status             | text | нет      | staged verified rejected; duplicate different digest denied |

## pricing_source_revisions — S2

| Поле              | Тип         | Nullable | Правило                                                                                      |
| ----------------- | ----------- | -------- | -------------------------------------------------------------------------------------------- |
| id                | uuid        | нет      | PK                                                                                           |
| connection_id     | uuid        | нет      | own connection FK                                                                            |
| model_id          | text        | нет      | exact upstream model                                                                         |
| mode              | text        | нет      | provider_auto manual                                                                         |
| price_revision_id | uuid        | да       | required manual; own immutable price                                                         |
| version           | bigint      | нет      | CAS per connection_id/model_id/currency; immutable increasing revision                       |
| actor_subject     | text        | нет      | verified actor                                                                               |
| created_at        | timestamptz | нет      | UTC                                                                                          |
| currency          | char(3)     | нет      | ISO uppercase; part of exact connection/model/currency policy key                            |
| effective_from    | timestamptz | нет      | UTC; unique started interval per policy key; cannot backdate changed receipts                |
| effective_to      | timestamptz | да       | half-open UTC interval; later than from; expiry preserves unknown, not older source fallback |

## project_tariff_revisions — S3

| Поле                 | Тип         | Nullable | Правило                                                                 |
| -------------------- | ----------- | -------- | ----------------------------------------------------------------------- |
| id                   | uuid        | нет      | PK                                                                      |
| namespace_binding_id | uuid        | нет      | own verified binding FK                                                 |
| virtual_model_id     | uuid        | да       | null project-wide; own model                                            |
| currency             | char(3)     | нет      | no FX                                                                   |
| mode                 | text        | нет      | default_markup custom_rates                                             |
| markup_bps           | integer     | да       | 2000 default; 0..1000000 for markup                                     |
| rates                | jsonb       | да       | exact per-million decimal strings, custom only                          |
| effective_from       | timestamptz | нет      | inclusive                                                               |
| effective_to         | timestamptz | да       | exclusive; no overlap                                                   |
| version              | bigint      | нет      | logical key CAS                                                         |
| actor_subject        | text        | нет      | verified actor                                                          |
| created_at           | timestamptz | нет      | UTC                                                                     |
| policy_id            | uuid        | нет      | own stable policy FK; draft revision window, not mutable active pointer |

## project_charge_events — S3

| Поле                    | Тип            | Nullable | Правило                                                                                                                                                                             |
| ----------------------- | -------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| id                      | uuid           | нет      | PK                                                                                                                                                                                  |
| request_id              | uuid           | нет      | own request FK                                                                                                                                                                      |
| attempt_id              | uuid           | нет      | own attempt FK                                                                                                                                                                      |
| namespace_binding_id    | uuid           | да       | frozen own binding; null explicitly unbound installation default                                                                                                                    |
| tariff_revision_id      | uuid           | да       | null immutable installation default snapshot                                                                                                                                        |
| cost_source_revision_id | uuid           | да       | real immutable source FK when configured; null only unconfigured frozen source, never fake identity                                                                                 |
| currency                | char(3)        | нет      | no FX                                                                                                                                                                               |
| cost_basis              | numeric(50,24) | да       | null unknown; confidence separate                                                                                                                                                   |
| project_amount          | numeric(50,24) | да       | null unknown; no float or per-request rounding                                                                                                                                      |
| margin_amount           | numeric(50,24) | да       | charge minus basis; may be negative                                                                                                                                                 |
| basis_confidence        | text           | нет      | confirmed estimated unknown                                                                                                                                                         |
| status                  | text           | нет      | final provisional pending                                                                                                                                                           |
| source_event_id         | text           | нет      | unique attempt/event semantic identity                                                                                                                                              |
| supersedes_id           | uuid           | да       | append-only correction FK                                                                                                                                                           |
| occurred_at             | timestamptz    | нет      | UTC                                                                                                                                                                                 |
| cost_source_state       | text           | нет      | configured iff real configuration FK non-null; independent of financial confidence. Unconfigured remains null across late receipt.                                                  |
| basis_source            | text           | нет      | unavailable -> null/unknown basis; price_estimate -> real source snapshot/estimated basis; provider_receipt -> trusted own event/confirmed basis even without pricing configuration |

## project_tariff_policies — S3

| Поле                 | Тип     | Nullable | Правило                                                  |
| -------------------- | ------- | -------- | -------------------------------------------------------- |
| id                   | uuid    | нет      | PK                                                       |
| namespace_binding_id | uuid    | нет      | own binding FK                                           |
| virtual_model_id     | uuid    | да       | own model; null project default                          |
| currency             | char(3) | нет      | logical key part                                         |
| version              | bigint  | нет      | CAS stable policy row; unique Namespace/profile/currency |

## project_tariff_activations — S3

| Поле               | Тип         | Nullable | Правило                                                                                                                                                              |
| ------------------ | ----------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| id                 | uuid        | нет      | PK                                                                                                                                                                   |
| policy_id          | uuid        | нет      | own policy FK                                                                                                                                                        |
| tariff_revision_id | uuid        | нет      | immutable own revision FK                                                                                                                                            |
| activate_at        | timestamptz | нет      | unique policy/time; inclusive                                                                                                                                        |
| status             | text        | нет      | initial scheduled/active acceptance state; current wire status derives from time, next non-cancelled activation and cancellation witness; no canonical row overwrite |
| policy_version     | bigint      | нет      | monotonic policy CAS                                                                                                                                                 |
| actor_subject      | text        | нет      | verified actor                                                                                                                                                       |

## evaluation_manual_scores — S6

| Поле                | Тип          | Nullable | Правило                               |
| ------------------- | ------------ | -------- | ------------------------------------- |
| id                  | uuid         | нет      | PK                                    |
| evaluation_id       | uuid         | нет      | own evaluation FK                     |
| case_id             | text         | нет      | existing case                         |
| profile_revision_id | uuid         | нет      | frozen candidate                      |
| repetition          | integer      | нет      | 1..10                                 |
| score               | numeric(4,3) | нет      | 0..1                                  |
| reason              | text         | нет      | bounded 500                           |
| actor_subject       | text         | нет      | verified scorer                       |
| reviewed_at         | timestamptz  | нет      | UTC                                   |
| version             | bigint       | нет      | CAS per run/candidate/case/repetition |

## service_delegations — S3

| Поле               | Тип           | Nullable | Правило                                     |
| ------------------ | ------------- | -------- | ------------------------------------------- |
| grant_id           | uuid          | нет      | PK; tombstones may precede inference        |
| issuer_instance_id | uuid          | нет      | trusted configured signer                   |
| client_id          | uuid          | да       | own client FK; absent tombstone only        |
| request_id         | uuid          | да       | original request; absent tombstone only     |
| execution_id       | uuid          | да       | Fleet execution; no ownership transfer      |
| fencing_token      | numeric(20,0) | да       | u64 checked; monotonic per issuer/execution |
| body_digest        | text          | да       | SHA256 raw request bytes                    |
| valid_until        | timestamptz   | да       | minimum grant and lease expiry              |
| revoked_at         | timestamptz   | да       | trusted issuer event                        |
| context_snapshot   | jsonb         | да       | verified Base v2 context; no prompt/secret  |

## project_tariff_cancellations — S3

| Поле           | Тип         | Nullable | Правило                                                   |
| -------------- | ----------- | -------- | --------------------------------------------------------- |
| id             | uuid        | нет      | PK                                                        |
| activation_id  | uuid        | нет      | own activation FK; unique one cancellation per activation |
| policy_id      | uuid        | нет      | same policy as activation; own FK                         |
| policy_version | bigint      | нет      | CAS increment serialized with activation; immutable       |
| cancelled_at   | timestamptz | нет      | database transaction UTC time strictly before activate_at |
| actor_subject  | text        | нет      | verified control principal                                |
| reason         | text        | нет      | bounded 1..500                                            |
| operation_id   | uuid        | нет      | own idempotent operation FK, original result retained     |

## Own relationships

| Таблица.поле                                  | Собственная цель              |
| --------------------------------------------- | ----------------------------- |
| providers.installation_id                     | installations.id              |
| connections.provider_id                       | providers.id                  |
| connection_generations.connection_id          | connections.id                |
| upstream_models.connection_id                 | connections.id                |
| operations.installation_id                    | installations.id              |
| login_attempts.connection_id                  | connections.id                |
| probe_snapshots.operation_id                  | operations.id                 |
| verification_evidence.probe_snapshot_id       | probe_snapshots.id            |
| runtime_qualifications.proof_id               | verification_evidence.id      |
| virtual_models.installation_id                | installations.id              |
| profile_revisions.virtual_model_id            | virtual_models.id             |
| profile_deployments.revision_id               | profile_revisions.id          |
| clients.installation_id                       | installations.id              |
| client_keys.client_id                         | clients.id                    |
| grants.client_id                              | clients.id                    |
| requests.client_id                            | clients.id                    |
| requests.profile_revision_id                  | profile_revisions.id          |
| requests.probe_snapshot_id                    | probe_snapshots.id            |
| replay_payloads.request_id                    | requests.id                   |
| attempts.request_id                           | requests.id                   |
| usage_facts.attempt_id                        | attempts.id                   |
| price_revisions.connection_id                 | connections.id                |
| ledger_entries.attempt_id                     | attempts.id                   |
| ledger_entries.original_entry_id              | ledger_entries.id             |
| ledger_entries.price_revision_id              | price_revisions.id            |
| budget_periods.policy_id                      | budget_policies.id            |
| reservations.budget_period_id                 | budget_periods.id             |
| reservations.attempt_id                       | attempts.id                   |
| subscriptions.connection_id                   | connections.id                |
| quota_observations.connection_id              | connections.id                |
| evaluation_runs.dataset_version_id            | dataset_versions.id           |
| evaluation_runs.budget_policy_id              | budget_policies.id            |
| evaluation_cases.run_id                       | evaluation_runs.id            |
| evaluation_cases.request_id                   | requests.id                   |
| evaluation_cases.candidate_revision_id        | profile_revisions.id          |
| clients.namespace_binding_id                  | namespace_bindings.id         |
| requests.namespace_binding_id                 | namespace_bindings.id         |
| budget_policies.namespace_binding_id          | namespace_bindings.id         |
| dataset_versions.namespace_binding_id         | namespace_bindings.id         |
| audit_events.namespace_binding_id             | namespace_bindings.id         |
| model_context_preferences.connection_id       | connections.id                |
| notifications.namespace_binding_id            | namespace_bindings.id         |
| notification_acknowledgements.notification_id | notifications.id              |
| pricing_source_revisions.connection_id        | connections.id                |
| pricing_source_revisions.price_revision_id    | price_revisions.id            |
| project_tariff_revisions.namespace_binding_id | namespace_bindings.id         |
| project_tariff_revisions.virtual_model_id     | virtual_models.id             |
| project_charge_events.request_id              | requests.id                   |
| project_charge_events.attempt_id              | attempts.id                   |
| project_charge_events.namespace_binding_id    | namespace_bindings.id         |
| project_charge_events.tariff_revision_id      | project_tariff_revisions.id   |
| project_charge_events.cost_source_revision_id | pricing_source_revisions.id   |
| project_tariff_policies.namespace_binding_id  | namespace_bindings.id         |
| project_tariff_revisions.policy_id            | project_tariff_policies.id    |
| project_tariff_activations.policy_id          | project_tariff_policies.id    |
| project_tariff_activations.tariff_revision_id | project_tariff_revisions.id   |
| evaluation_manual_scores.evaluation_id        | evaluation_runs.id            |
| project_charge_events.supersedes_id           | project_charge_events.id      |
| project_tariff_cancellations.activation_id    | project_tariff_activations.id |
| project_tariff_cancellations.policy_id        | project_tariff_policies.id    |
| project_tariff_cancellations.operation_id     | operations.id                 |

## Composite invariants

- credential_versions(connection_id,generation) references connection_generations; no absent local auth credential requirement
- runtime qualification binds connection/generation/model/adapter/endpoint and immutable proof
- active_revision_id references revision of same virtual_model, not any existing revision
- request_kind verification iff probe_snapshot_id non-null and profile_revision_id null
- unique installation/client/idempotency intent; control operation separate principal namespace
- unknown charge amount null, confirmed/estimated amount exact; correction linked and append-only
- effective charge confirmed replaces linked estimate; aggregates not financial authority
- effective source-bound price intervals non-overlapping per connection/model/current tier/currency; immutable quote allowed windows may overlap
- reservations unique attempt/budget period; sorted multi-scope locks
- evaluation case unique run/candidate/repetition/case; restart no replay unknown I/O
- operation safe_result never stores plaintext client/provider keys or model output
- UTC half-open periods; cash occurred_at distinct subscription service period
- unique registry_instance_id/namespace_id within own installation; exact Tracker binding; no external FK
- unique connection_id/model_id preference; CAS; no unknown physical-bound default
- notification outbox delivery acknowledgement differs from actor read/ACK; no financial effect
- unique source_instance_id/source_kind/source_record_id import identity with digest conflict protection
- project tariff precedence exact namespace/profile then namespace default then installation 2000 bps; currency-qualified nonoverlapping intervals
- project charge events append-only and deduped by attempt/source event; unknown amount null; old snapshot used for late receipt
- manual subscription allocation never counted as a second provider cash charge; budget policy remains provider cost
- tariff draft windows may overlap; activation timeline CAS derives nonoverlapping intervals, supersession never changes revision bytes
- service delegation raw body SHA256 and exact issuer/client/Namespace/context/profile binding required; revocation tombstone and fence checked before each attempt
- manual score unique run/candidate/case/repetition version and verified actor; no financial mutation
- S2 proof uses the same early request/attempt/usage/price/ledger/budget primitives; no future-stage FK or unaccounted synthetic probe
- unconfigured cost source is null FK plus explicit state/unknown basis, never a generated source identity
- scheduled tariff cancellation appends one same-policy witness before activation time, increments policy CAS and preserves cancelled time identity
- StatisticsFilters is the single canonical query/echo/cursor/export shape; protocol selects exactly one service invocation schema
- unconfigured rate admission requires independently qualified billing currency and explicit bounded unknown-cost grant; active hard monetary budgets still fail closed
- configuration absence does not prevent trusted late receipt confirmation; source_event identity and original tariff retained, no invented price row or double effective charge
- all project policy mutations including draft creation/activation/cancellation serialize and increment stable policy CAS, distinct from immutable revision sequence
- cancellation composite FK (activation_id,policy_id) references activation(id,policy_id); unique activation_id; no canonical activation update
