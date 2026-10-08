# Словарь данных и constraints

[Data dictionary](contracts/data-dictionary.v1.json) — typed design schema.
Код/SQL migrations ещё не созданы; это не выполненная migration или DB PASS.
Logical overview — DATA_MODEL; exact column rules и FK здесь, Wire DTO в OpenAPI.

## installations — S1

| Column         | PostgreSQL type | Nullable | Rule                  |
| -------------- | --------------- | -------- | --------------------- |
| id             | uuid            | нет      | PK                    |
| stable_key     | text            | нет      | unique                |
| status         | text            | нет      | active or blocked     |
| blocked_reason | text            | да       | safe admission reason |
| created_at     | timestamptz     | нет      | UTC                   |

## providers — S1

| Column          | PostgreSQL type | Nullable | Rule              |
| --------------- | --------------- | -------- | ----------------- |
| id              | uuid            | нет      | PK                |
| installation_id | uuid            | нет      | installations.id  |
| kind            | text            | нет      | adapter allowlist |
| display_name    | varchar(120)    | нет      | presentation only |
| created_at      | timestamptz     | нет      | UTC               |

## connections — S2

| Column              | PostgreSQL type | Nullable | Rule                                           |
| ------------------- | --------------- | -------- | ---------------------------------------------- |
| id                  | uuid            | нет      | PK                                             |
| provider_id         | uuid            | нет      | providers.id                                   |
| display_name        | varchar(120)    | нет      | user label                                     |
| endpoint_policy_ref | text            | нет      | deployment allowlist                           |
| billing_mode        | text            | нет      | metered subscription local unknown             |
| generation          | bigint          | нет      | monotonic >=1                                  |
| status              | text            | нет      | enabled disabled revoked authorization_unknown |
| version             | bigint          | нет      | CAS >=1                                        |

## connection_generations — S2

| Column               | PostgreSQL type | Nullable | Rule                                   |
| -------------------- | --------------- | -------- | -------------------------------------- |
| connection_id        | uuid            | нет      | composite PK connections.id            |
| generation           | bigint          | нет      | composite PK >=1                       |
| authorization_state  | text            | нет      | absent prepared active revoked unknown |
| adapter_revision     | text            | нет      | exact adapter                          |
| endpoint_policy_hash | text            | нет      | immutable snapshot                     |
| created_at           | timestamptz     | нет      | UTC                                    |

## credential_versions — S2

| Column        | PostgreSQL type | Nullable | Rule                           |
| ------------- | --------------- | -------- | ------------------------------ |
| connection_id | uuid            | нет      | connection_generations binding |
| generation    | bigint          | нет      | same generation composite PK   |
| ciphertext    | bytea           | нет      | encrypted no raw secret        |
| nonce         | bytea           | нет      | unique 12-byte AES-GCM nonce   |
| key_id        | text            | нет      | separate key escrow reference  |
| state         | text            | нет      | prepared active revoked        |
| created_at    | timestamptz     | нет      | UTC                            |

## upstream_models — S2

| Column            | PostgreSQL type | Nullable | Rule                    |
| ----------------- | --------------- | -------- | ----------------------- |
| id                | uuid            | нет      | PK                      |
| connection_id     | uuid            | нет      | exact connection        |
| generation        | bigint          | нет      | exact generation        |
| provider_model_id | text            | нет      | exact case-sensitive ID |
| input_limit       | bigint          | да       | unknown not default     |
| output_limit      | bigint          | да       | unknown not default     |
| metadata          | jsonb           | нет      | safe allowlisted fields |
| observed_at       | timestamptz     | нет      | catalog freshness       |

## operations — S1

| Column          | PostgreSQL type | Nullable | Rule                                       |
| --------------- | --------------- | -------- | ------------------------------------------ |
| id              | uuid            | нет      | PK                                         |
| installation_id | uuid            | нет      | own installation                           |
| principal_kind  | text            | нет      | human client internal                      |
| principal_id    | text            | нет      | trusted subject                            |
| idempotency_key | uuid            | нет      | caller stable intent                       |
| binding_hmac    | bytea           | нет      | method path payload ETag principal         |
| action          | text            | нет      | allowlisted operation                      |
| resource_id     | uuid            | да       | own target                                 |
| state           | text            | нет      | pending succeeded failed unknown cancelled |
| safe_result     | jsonb           | да       | no raw credentials or one-time secret      |
| created_at      | timestamptz     | нет      | UTC                                        |
| updated_at      | timestamptz     | нет      | UTC                                        |
| expires_at      | timestamptz     | нет      | 30-day control dedupe                      |

## login_attempts — S4

| Column               | PostgreSQL type | Nullable | Rule                                               |
| -------------------- | --------------- | -------- | -------------------------------------------------- |
| id                   | uuid            | нет      | PK operation reference                             |
| connection_id        | uuid            | нет      | exact target                                       |
| generation           | bigint          | нет      | fenced authorization                               |
| state                | text            | нет      | pending completed expired cancelled failed unknown |
| encrypted_checkpoint | bytea           | да       | own vault-only projection                          |
| key_id               | text            | да       | checkpoint key reference                           |
| expires_at           | timestamptz     | нет      | provider device login deadline                     |

## probe_snapshots — S2

| Column         | PostgreSQL type | Nullable | Rule                                          |
| -------------- | --------------- | -------- | --------------------------------------------- |
| id             | uuid            | нет      | PK                                            |
| operation_id   | uuid            | нет      | own operation                                 |
| scope          | text            | нет      | connection_model or profile_draft             |
| config_hash    | text            | нет      | frozen shape                                  |
| draft_model_id | uuid            | да       | only profile scope                            |
| draft_version  | bigint          | да       | only profile scope                            |
| configuration  | jsonb           | нет      | secret-free exact targets bounds capabilities |
| created_at     | timestamptz     | нет      | UTC                                           |

## verification_evidence — S2

| Column            | PostgreSQL type | Nullable | Rule                            |
| ----------------- | --------------- | -------- | ------------------------------- |
| id                | uuid            | нет      | PK                              |
| probe_snapshot_id | uuid            | нет      | probe snapshot FK               |
| state             | text            | нет      | pending verified failed expired |
| child_evidence    | jsonb           | нет      | ordered exact proof IDs         |
| receipt_digest    | text            | да       | trusted terminal witness        |
| expires_at        | timestamptz     | нет      | publication freshness only      |
| created_at        | timestamptz     | нет      | UTC                             |

## runtime_qualifications — S2

| Column               | PostgreSQL type | Nullable | Rule                     |
| -------------------- | --------------- | -------- | ------------------------ |
| id                   | uuid            | нет      | PK                       |
| connection_id        | uuid            | нет      | exact account            |
| generation           | bigint          | нет      | immutable generation     |
| provider_model_id    | text            | нет      | exact model              |
| adapter_revision     | text            | нет      | exact code               |
| endpoint_policy_hash | text            | нет      | scope                    |
| proof_id             | uuid            | нет      | trusted evidence         |
| capabilities         | jsonb           | нет      | qualified set and bounds |
| state                | text            | нет      | active invalidated       |
| invalidated_reason   | text            | да       | safe reason              |

## virtual_models — S2

| Column             | PostgreSQL type | Nullable | Rule                           |
| ------------------ | --------------- | -------- | ------------------------------ |
| id                 | uuid            | нет      | PK                             |
| installation_id    | uuid            | нет      | own installation               |
| slug               | varchar(63)     | нет      | unique immutable API name      |
| display_name       | varchar(120)    | нет      | presentation                   |
| draft              | jsonb           | нет      | validated ProfileInput         |
| draft_version      | bigint          | нет      | CAS >=1                        |
| active_revision_id | uuid            | да       | own profile revision only      |
| status             | text            | нет      | draft active disabled archived |

## profile_revisions — S2

| Column                  | PostgreSQL type | Nullable | Rule                           |
| ----------------------- | --------------- | -------- | ------------------------------ |
| id                      | uuid            | нет      | PK                             |
| virtual_model_id        | uuid            | нет      | owner FK                       |
| sequence                | bigint          | нет      | unique per model               |
| configuration           | jsonb           | нет      | immutable params bounds policy |
| config_hash             | text            | нет      | exact normalized bytes         |
| profile_verification_id | uuid            | нет      | exact draft proof              |
| created_by              | text            | нет      | trusted actor                  |
| created_at              | timestamptz     | нет      | UTC                            |

## profile_deployments — S2

| Column              | PostgreSQL type | Nullable | Rule                                           |
| ------------------- | --------------- | -------- | ---------------------------------------------- |
| revision_id         | uuid            | нет      | composite PK profile_revisions.id              |
| ordinal             | integer         | нет      | ordered composite PK >=1                       |
| connection_id       | uuid            | нет      | exact connection                               |
| generation          | bigint          | нет      | frozen immutable generation                    |
| model_id            | text            | нет      | exact provider ID                              |
| verification_id     | uuid            | нет      | child witness                                  |
| deployment_snapshot | jsonb           | нет      | adapter endpoint capabilities bounds no secret |

## clients — S1

| Column           | PostgreSQL type | Nullable | Rule                                               |
| ---------------- | --------------- | -------- | -------------------------------------------------- |
| id               | uuid            | нет      | PK                                                 |
| installation_id  | uuid            | нет      | own scope                                          |
| application_key  | varchar(120)    | нет      | unique per installation                            |
| project_binding  | text            | нет      | trusted control mapping                            |
| allowed_profiles | jsonb           | нет      | bounded UUID allowlist                             |
| scopes           | jsonb           | нет      | infer read_own_usage read_result                   |
| cost_policy      | text            | нет      | budget_guaranteed or explicit cost_unknown_allowed |
| expires_at       | timestamptz     | нет      | UTC                                                |
| max_concurrency  | integer         | нет      | 1..100 and own global cap                          |
| max_rpm          | integer         | нет      | 1..10000                                           |
| status           | text            | нет      | enabled disabled revoked                           |
| version          | bigint          | нет      | CAS                                                |

## client_keys — S1

| Column     | PostgreSQL type | Nullable | Rule                    |
| ---------- | --------------- | -------- | ----------------------- |
| id         | uuid            | нет      | PK                      |
| client_id  | uuid            | нет      | clients.id              |
| key_hash   | bytea           | нет      | unique no plaintext     |
| prefix     | text            | нет      | safe display identifier |
| expires_at | timestamptz     | нет      | UTC                     |
| revoked_at | timestamptz     | да       | revoke effective        |

## grants — S1

| Column           | PostgreSQL type | Nullable | Rule                           |
| ---------------- | --------------- | -------- | ------------------------------ |
| id               | uuid            | нет      | PK                             |
| installation_id  | uuid            | нет      | scope                          |
| principal_id     | text            | нет      | trusted identity not body user |
| client_id        | uuid            | да       | machine binding                |
| project_binding  | text            | нет      | exact allowed project          |
| action           | text            | нет      | allowlisted permission         |
| virtual_model_id | uuid            | да       | exact model for infer          |
| bounds           | jsonb           | нет      | purpose/caps                   |
| expires_at       | timestamptz     | нет      | validity                       |
| revoked_at       | timestamptz     | да       | deny new access                |

## requests — S3

| Column              | PostgreSQL type | Nullable | Rule                                                              |
| ------------------- | --------------- | -------- | ----------------------------------------------------------------- |
| id                  | uuid            | нет      | PK                                                                |
| installation_id     | uuid            | нет      | own scope                                                         |
| client_id           | uuid            | нет      | own normal/internal client                                        |
| principal_id        | text            | нет      | trusted actor                                                     |
| project_binding     | text            | нет      | frozen grant                                                      |
| request_kind        | text            | нет      | inference verification evaluation                                 |
| profile_revision_id | uuid            | да       | required non-verification                                         |
| probe_snapshot_id   | uuid            | да       | required verification                                             |
| idempotency_key     | uuid            | нет      | client key or generated own ID                                    |
| payload_hmac        | bytea           | нет      | semantic intent binding                                           |
| state               | text            | нет      | admitted dispatching streaming completed failed cancelled unknown |
| cancel_requested    | boolean         | нет      | default false separate from terminal                              |
| admitted_at         | timestamptz     | нет      | UTC                                                               |
| finished_at         | timestamptz     | да       | transport terminal                                                |
| version             | bigint          | нет      | CAS fence                                                         |

## replay_payloads — S3

| Column             | PostgreSQL type | Nullable | Rule                        |
| ------------------ | --------------- | -------- | --------------------------- |
| request_id         | uuid            | нет      | PK own request              |
| encrypted_response | bytea           | нет      | own result buffer only      |
| key_id             | text            | нет      | separate own encryption key |
| expires_at         | timestamptz     | нет      | <=24h                       |
| protocol           | text            | нет      | chat_completions responses  |

## attempts — S3

| Column                | PostgreSQL type | Nullable | Rule                                                             |
| --------------------- | --------------- | -------- | ---------------------------------------------------------------- |
| id                    | uuid            | нет      | PK                                                               |
| request_id            | uuid            | нет      | requests.id                                                      |
| ordinal               | integer         | нет      | unique request ordinal                                           |
| deployment_snapshot   | jsonb           | нет      | frozen exact scope                                               |
| state                 | text            | нет      | intended dispatched streaming completed failed cancelled unknown |
| accepted              | text            | нет      | not_accepted accepted unknown                                    |
| dispatch_intent_at    | timestamptz     | нет      | durable before I/O                                               |
| dispatched_at         | timestamptz     | да       | sent boundary                                                    |
| first_content_at      | timestamptz     | да       | TTFT                                                             |
| finished_at           | timestamptz     | да       | terminal witness                                                 |
| provider_request_id   | text            | да       | own readback                                                     |
| actual_model          | text            | да       | not guessed                                                      |
| actual_model_verified | boolean         | нет      | default false                                                    |

## usage_facts — S3

| Column          | PostgreSQL type | Nullable | Rule                                    |
| --------------- | --------------- | -------- | --------------------------------------- |
| id              | uuid            | нет      | PK                                      |
| attempt_id      | uuid            | нет      | attempts.id                             |
| source          | text            | нет      | trusted ingestion kind                  |
| source_event_id | text            | нет      | unique within source installation       |
| categories      | jsonb           | нет      | disjoint known/unknown token categories |
| provenance      | jsonb           | нет      | adapter receipt/category inclusion      |
| observed_at     | timestamptz     | нет      | UTC                                     |

## price_revisions — S3

| Column          | PostgreSQL type | Nullable | Rule                               |
| --------------- | --------------- | -------- | ---------------------------------- |
| id              | uuid            | нет      | PK                                 |
| connection_id   | uuid            | нет      | own account                        |
| model_id        | text            | нет      | exact upstream                     |
| tier            | text            | нет      | billing tier                       |
| currency        | char(3)         | нет      | supported explicit currency        |
| input_uncached  | numeric(30,12)  | нет      | nonnegative per-million            |
| input_cached    | numeric(30,12)  | да       | unknown not zero                   |
| output_billable | numeric(30,12)  | нет      | nonnegative per-million            |
| request_fee     | numeric(38,18)  | да       | known fixed per-attempt or unknown |
| effective_from  | timestamptz     | нет      | inclusive                          |
| effective_to    | timestamptz     | да       | exclusive > from                   |
| source          | text            | нет      | price provenance                   |

## ledger_entries — S3

| Column            | PostgreSQL type | Nullable | Rule                                                  |
| ----------------- | --------------- | -------- | ----------------------------------------------------- |
| id                | uuid            | нет      | PK                                                    |
| installation_id   | uuid            | нет      | own scope                                             |
| attempt_id        | uuid            | да       | required invocation charge                            |
| charge_key        | text            | нет      | stable logical charge identity                        |
| kind              | text            | нет      | reserve release charge correction subscription_charge |
| amount            | numeric(38,18)  | да       | null only unknown charge                              |
| currency          | char(3)         | нет      | explicit                                              |
| confidence        | text            | нет      | confirmed estimated unknown                           |
| original_entry_id | uuid            | да       | correction/settlement reference                       |
| source            | text            | нет      | trusted origin                                        |
| source_event_id   | text            | нет      | dedupe                                                |
| price_revision_id | uuid            | да       | estimate provenance                                   |
| occurred_at       | timestamptz     | нет      | cash basis                                            |

## budget_policies — S3

| Column          | PostgreSQL type | Nullable | Rule                                |
| --------------- | --------------- | -------- | ----------------------------------- |
| id              | uuid            | нет      | PK                                  |
| installation_id | uuid            | нет      | scope                               |
| scope_type      | text            | нет      | installation project client profile |
| scope_id        | text            | нет      | exact target                        |
| currency        | char(3)         | нет      | no mixed currencies                 |
| period          | text            | нет      | utc_day utc_month                   |
| hard_limit      | numeric(38,18)  | нет      | >=0                                 |
| thresholds      | jsonb           | нет      | ascending unique 1..99              |
| version         | bigint          | нет      | CAS                                 |
| status          | text            | нет      | active disabled                     |

## budget_periods — S3

| Column       | PostgreSQL type | Nullable | Rule                        |
| ------------ | --------------- | -------- | --------------------------- |
| id           | uuid            | нет      | PK                          |
| policy_id    | uuid            | нет      | budget_policies.id          |
| period_start | timestamptz     | нет      | UTC exact boundary          |
| period_end   | timestamptz     | нет      | exclusive                   |
| charged      | numeric(38,18)  | нет      | canonical effective charges |
| reserved     | numeric(38,18)  | нет      | held >=0                    |
| version      | bigint          | нет      | row lock/CAS                |

## reservations — S3

| Column           | PostgreSQL type | Nullable | Rule                  |
| ---------------- | --------------- | -------- | --------------------- |
| attempt_id       | uuid            | нет      | composite PK          |
| budget_period_id | uuid            | нет      | composite PK          |
| amount           | numeric(38,18)  | нет      | bounded >=0           |
| currency         | char(3)         | нет      | same policy           |
| state            | text            | нет      | held released settled |
| created_at       | timestamptz     | нет      | before dispatch       |

## subscriptions — S4

| Column           | PostgreSQL type | Nullable | Rule                         |
| ---------------- | --------------- | -------- | ---------------------------- |
| id               | uuid            | нет      | PK                           |
| connection_id    | uuid            | нет      | own account                  |
| period_start     | timestamptz     | нет      | service period inclusive     |
| period_end       | timestamptz     | нет      | service period exclusive     |
| amount           | numeric(38,18)  | нет      | >=0                          |
| currency         | char(3)         | нет      | explicit                     |
| payment_status   | text            | нет      | planned pending paid unknown |
| receipt_ref      | text            | да       | trust importer reference     |
| source_event_id  | text            | да       | dedupe trusted payment       |
| cash_occurred_at | timestamptz     | да       | known paid charge            |
| confidence       | text            | нет      | confirmed estimated unknown  |

## quota_observations — S4

| Column        | PostgreSQL type | Nullable | Rule                |
| ------------- | --------------- | -------- | ------------------- |
| id            | uuid            | нет      | PK                  |
| connection_id | uuid            | нет      | exact account       |
| generation    | bigint          | нет      | exact scope         |
| unit          | text            | нет      | provider quota unit |
| limit         | bigint          | да       | known nonnegative   |
| remaining     | bigint          | да       | known nonnegative   |
| source        | text            | нет      | provider witness    |
| observed_at   | timestamptz     | нет      | freshness           |
| reset_at      | timestamptz     | да       | known reset         |

## dataset_versions — S6

| Column          | PostgreSQL type | Nullable | Rule                         |
| --------------- | --------------- | -------- | ---------------------------- |
| id              | uuid            | нет      | PK                           |
| dataset_id      | uuid            | нет      | stable dataset               |
| sequence        | bigint          | нет      | unique per dataset           |
| project_binding | text            | нет      | owner scope                  |
| created_by      | text            | нет      | trusted actor                |
| cases_hash      | text            | нет      | immutable exact dataset      |
| case_count      | integer         | нет      | 1..1000                      |
| content_ref     | text            | нет      | own encrypted artifact store |
| expires_at      | timestamptz     | нет      | 7d content default           |

## evaluation_runs — S6

| Column             | PostgreSQL type | Nullable | Rule                                      |
| ------------------ | --------------- | -------- | ----------------------------------------- |
| id                 | uuid            | нет      | PK                                        |
| dataset_version_id | uuid            | нет      | exact version                             |
| project_binding    | text            | нет      | own scope                                 |
| config_snapshot    | jsonb           | нет      | profiles scorer params repetitions        |
| budget_policy_id   | uuid            | нет      | applicable scope                          |
| state              | text            | нет      | queued running completed failed cancelled |
| lease_generation   | bigint          | нет      | fencing                                   |
| lease_expires_at   | timestamptz     | да       | worker lease                              |
| cancel_requested   | boolean         | нет      | default false                             |
| created_at         | timestamptz     | нет      | UTC                                       |

## evaluation_cases — S6

| Column                | PostgreSQL type | Nullable | Rule                                           |
| --------------------- | --------------- | -------- | ---------------------------------------------- |
| run_id                | uuid            | нет      | composite unique                               |
| candidate_revision_id | uuid            | нет      | frozen exact profile                           |
| repetition            | integer         | нет      | 1..10                                          |
| case_id               | text            | нет      | dataset case                                   |
| request_id            | uuid            | нет      | unique dispatch binding                        |
| outcome               | text            | нет      | pass fail invalid incomparable skipped unknown |
| score                 | jsonb           | да       | scorer witness                                 |
| artifact_ref          | text            | да       | scoped TTL content                             |

## aggregate_buckets — S5

| Column             | PostgreSQL type | Nullable | Rule                         |
| ------------------ | --------------- | -------- | ---------------------------- |
| id                 | uuid            | нет      | PK                           |
| filter_hash        | text            | нет      | authorized grouping identity |
| time_bucket        | timestamptz     | нет      | explicit basis               |
| dimension_snapshot | jsonb           | нет      | bounded group                |
| known_counts       | jsonb           | нет      | no fake unknown zeros        |
| totals             | jsonb           | нет      | per currency confidence      |
| watermark          | timestamptz     | нет      | rebuild provenance           |

## audit_events — S1

| Column          | PostgreSQL type | Nullable | Rule                |
| --------------- | --------------- | -------- | ------------------- |
| id              | uuid            | нет      | PK                  |
| installation_id | uuid            | нет      | own scope           |
| actor           | text            | нет      | trusted subject     |
| action          | text            | нет      | allowlist           |
| object_id       | text            | нет      | safe identity       |
| revision        | text            | да       | exact revision      |
| operation_id    | uuid            | нет      | own operation       |
| reason          | text            | да       | safe text no secret |
| happened_at     | timestamptz     | нет      | UTC                 |

## alert_outbox — S5

| Column         | PostgreSQL type | Nullable | Rule                               |
| -------------- | --------------- | -------- | ---------------------------------- |
| id             | uuid            | нет      | PK                                 |
| event_key      | text            | нет      | unique scope period threshold      |
| scope_snapshot | jsonb           | нет      | own target                         |
| kind           | text            | нет      | budget unknown projection provider |
| state          | text            | нет      | pending delivered failed           |
| safe_payload   | jsonb           | нет      | no secrets/content                 |
| created_at     | timestamptz     | нет      | UTC                                |
| delivered_at   | timestamptz     | да       | in-app ack                         |

## Составные инварианты

- credential_versions(connection_id,generation) references connection_generations; no absent local auth credential requirement
- runtime qualification binds connection/generation/model/adapter/endpoint and immutable proof
- active_revision_id references revision of same virtual_model, not any existing revision
- request_kind verification iff probe_snapshot_id non-null and profile_revision_id null
- unique installation/client/idempotency intent; control operation separate principal namespace
- unknown charge amount null, confirmed/estimated amount exact; correction linked and append-only
- effective charge confirmed replaces linked estimate; aggregates not financial authority
- price intervals non-overlapping per connection/model/tier/currency
- reservations unique attempt/budget period; sorted multi-scope locks
- evaluation case unique run/candidate/repetition/case; restart no replay unknown I/O
- operation safe_result never stores plaintext client/provider keys or model output
- UTC half-open periods; cash occurred_at distinct subscription service period

## Index и migration policy

PK/unique/FK создаются вместе с stage schema. Request/ledger query indexes из DATA_MODEL
проверяются EXPLAIN на representative own data. Защищённые FK не CASCADE financial
history. Nullable fields означают knownness/lifecycle, не undocumented defaults.
Operation binding включает HTTP method/path/semantic body/ETag/principal; duplicate
same key в другой action не permission на new intent.
Local metadata-only connections имеют generation без credential: секрет не нужен
для pure local approved endpoint. Secret state не хранится в safe result/audit.

Любое расхождение JSON/DD/wire исправляет canonical owner до зависимой реализации.
Applied migrations immutable; fresh+upgrade paths должны пройти real PG tests.
