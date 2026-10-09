# API AI Hub v1

[openapi.v1.json](contracts/openapi.v1.json) — проектный OpenAPI 3.1 контракт.
Handlers/runtime отсутствуют. `openapi/openapi.json` после реализации генерируется
из Rust DTO/handlers; draft сохраняется как исходная design revision, parity migration
не расширяет capabilities автоматически. Версия 0.5.0-design — не release приложения.

## Поверхности

- /api/v1: control, reads/analytics и own operation readback.
- /v1: ограниченный OpenAI-compatible inference contract, Hub client auth.
- /health: liveness/readiness, без provider billable probe.

Central Auth/HubClientKey являются альтернативами только для authorized own
request reads/cancel. Client scope не даёт управления providers/prices/clients.
All control mutations требуют active browser access либо точный ai-hub:write PAT.
Чтение по PAT — ai-hub:read; конфигурация machine grants не authentication identity.
Project/client access additionally checked; cursor/export не обходят grants.

## Общие правила

UUID IDs; timestamps UTC RFC3339; money decimal strings/currency/confidence.
Unknown financial amount=null; known zero explicit. If-Match strong quoted version
для CAS; stale → 412. Mutable config GET возвращает ETag; list limit default50/max100,
opaque next_cursor bound to actor/query/snapshot, no floating offset races.
Request readback для machine разрешён только own client/request binding.

Control side-effect mutations требуют UUID Idempotency-Key: same principal+key+
canonical payload возвращает прежнюю durable operation; иной payload →409.
Replay key retention 30 дней; raw replay response buffer <=24h, после expiry
replay_expired/409 + safe metadata readback, no new model call.
Повтор nonstream terminal request может вернуть сохранённый оригинальный reply.
Повтор stream, in-progress или unknown request возвращает 409 stream_replay_unavailable,
request_in_progress либо request_unknown с original X-AIHub-Request-Id; новый I/O
не выполняется. GET request/result выдаёт terminal reply только own read_result
grant в пределах encrypted buffer TTL; metadata grant не даёт content access.
Credential mutation ответ metadata/operation; provider secret никогда не response.
Hub access key показывается один раз; replay issueClientKey не создаёт новый key,
при недоступном one-time response нужна explicit rotation operation.

Inference без custom Idempotency-Key допускается для обычного SDK: Hub создаёт
новый request ID и не обещает dedupe отдельного повторного HTTP запроса.
Надёжный клиент передаёт свой key и при uncertain network outcome читает request.
X-Request-Id bounded verified UUID; invalid → server UUID. X-AIHub-Request-Id и
X-AIHub-Profile-Revision выданы до stream; CORS expose headers для own UI.
Optional input X-AIHub-Profile-Revision только published allowed revision данного
virtual slug; нет raw connection/upstream override. Idempotency binding включает
этот header, semantic payload и authenticated principal.

## Операции

| Method | Target path                                       | Operation ID                  |
| ------ | ------------------------------------------------- | ----------------------------- |
| GET    | /health/live                                      | healthLive                    |
| GET    | /health/ready                                     | healthReady                   |
| GET    | /api/v1/auth/me                                   | readIdentity                  |
| GET    | /api/v1/providers                                 | listProviders                 |
| POST   | /api/v1/providers/{provider}/connections          | createConnection              |
| GET    | /api/v1/connections/{connection_id}               | readConnection                |
| PATCH  | /api/v1/connections/{connection_id}               | updateConnection              |
| DELETE | /api/v1/connections/{connection_id}               | disableConnection             |
| PUT    | /api/v1/connections/{connection_id}/credentials   | writeCredential               |
| DELETE | /api/v1/connections/{connection_id}/credentials   | revokeConnectionAuthorization |
| POST   | /api/v1/connections/{connection_id}/login         | startManagedLogin             |
| GET    | /api/v1/connections/{connection_id}/models        | readModelCatalog              |
| POST   | /api/v1/connections/{connection_id}/verifications | startVerification             |
| GET    | /api/v1/verifications/{verification_id}           | readVerification              |
| GET    | /api/v1/operations/{operation_id}                 | readOperation                 |
| POST   | /api/v1/operations/{operation_id}/cancel          | cancelOperation               |
| GET    | /api/v1/virtual-models                            | listVirtualModels             |
| POST   | /api/v1/virtual-models                            | createVirtualModel            |
| GET    | /api/v1/virtual-models/{model_id}                 | readVirtualModel              |
| PATCH  | /api/v1/virtual-models/{model_id}                 | updateVirtualModelDraft       |
| DELETE | /api/v1/virtual-models/{model_id}                 | archiveVirtualModel           |
| POST   | /api/v1/virtual-models/{model_id}/publish         | publishRevision               |
| POST   | /api/v1/virtual-models/{model_id}/rollback        | rollbackRevision              |
| GET    | /api/v1/virtual-models/{model_id}/revisions       | listRevisions                 |
| GET    | /api/v1/clients                                   | listClients                   |
| POST   | /api/v1/clients                                   | createClient                  |
| PATCH  | /api/v1/clients/{client_id}                       | updateClient                  |
| POST   | /api/v1/clients/{client_id}/keys                  | issueClientKey                |
| DELETE | /api/v1/clients/{client_id}/keys/{key_id}         | revokeClientKey               |
| GET    | /api/v1/requests                                  | listRequests                  |
| GET    | /api/v1/requests/{request_id}                     | readRequest                   |
| POST   | /api/v1/requests/{request_id}/cancel              | cancelRequest                 |
| GET    | /api/v1/statistics/summary                        | readStatisticsSummary         |
| GET    | /api/v1/statistics/timeseries                     | readStatisticsTimeseries      |
| GET    | /api/v1/statistics/breakdown                      | readStatisticsBreakdown       |
| GET    | /api/v1/statistics/expenses                       | readStatisticsExpenses        |
| GET    | /api/v1/prices                                    | listPrices                    |
| POST   | /api/v1/prices                                    | createPriceRevision           |
| GET    | /api/v1/subscriptions                             | listSubscriptions             |
| POST   | /api/v1/subscriptions                             | recordSubscription            |
| GET    | /api/v1/budgets                                   | listBudgets                   |
| POST   | /api/v1/budgets                                   | createBudget                  |
| PATCH  | /api/v1/budgets/{budget_id}                       | updateBudget                  |
| POST   | /api/v1/datasets                                  | createDatasetVersion          |
| GET    | /api/v1/datasets                                  | listDatasetVersions           |
| GET    | /api/v1/evaluations                               | listEvaluations               |
| POST   | /api/v1/evaluations                               | startEvaluation               |
| GET    | /api/v1/evaluations/{evaluation_id}               | readEvaluation                |
| POST   | /api/v1/evaluations/{evaluation_id}/cancel        | cancelEvaluation              |
| GET    | /api/v1/exports                                   | exportStatistics              |
| GET    | /api/v1/audit                                     | listAudit                     |
| GET    | /api/v1/settings/runtime                          | readEffectiveSettings         |
| GET    | /v1/models                                        | listGatewayModels             |
| POST   | /v1/chat/completions                              | createChatCompletion          |
| POST   | /v1/responses                                     | createResponse                |
| POST   | /api/v1/ledger/corrections                        | requestLedgerCorrection       |
| GET    | /api/v1/requests/{request_id}/result              | readRequestResult             |
| POST   | /api/v1/virtual-models/{model_id}/disable         | disableVirtualModel           |
| POST   | /api/v1/virtual-models/{model_id}/enable          | enableVirtualModel            |
| POST   | /api/v1/clients/{client_id}/disable               | disableClient                 |
| POST   | /api/v1/clients/{client_id}/enable                | enableClient                  |
| POST   | /api/v1/virtual-models/{model_id}/verifications   | verifyVirtualModelDraft       |
| GET    | /health                                           | platformHealth                |
| GET    | /integration/status                               | readIntegrationStatus         |
| GET    | /branding/contract                                | readBrandingContract          |

PATCH endpoints принимают полное typed replacement settings body; это не JSON Patch.
POST create settings возвращает 201; inference возвращает 200. Async mutation —
202 + own Operation, state pending/succeeded/failed/unknown/cancelled, safe error.
DELETE connection отключает новый admission с сохранением history; DELETE credentials
отзывает own authorization/generation через durable operation, не удаляет facts.
Publish/rollback/cancel outcomes читаются по operation ID, а не слепо повторяются.
Disable/enable client/model — CAS операции без удаления history. Enable модели
повторно проверяет её active revision/current qualification, не восстанавливает
отозванную connection. Archive требует предварительного disable.
Provider verification проверяет connection/model, не требует существующего профиля.
Profile verification фиксирует exact draft version/config hash и ordered deployments;
publication принимает только profile_verification_id этого draft/generation/endpoint.
Draft хранит выбранные deployments без обязательного proof ID, поэтому его можно
создать и сохранить до verification. Proof не caller field внутри настроек.
Verification запускает bounded job с immutable own probe snapshot:
активация не требуется, но нужны собственный verification grant/budget и ledger.
Public launch выбирает существующий budget_policy_id из budgets API. Server
проверяет actor/project scope и выводит purpose-bound internal execution grant;
caller не создаёт signed grant и не назначает себе monetary authority в body.
S2 qualification только controlled synthetic server; платные live probes только S4
через уже реализованный S3 admission. Proof scope включает endpoint/environment.

## Compatible subset

[INFERENCE_V1](contracts/INFERENCE_V1.md) задаёт text/full/stream/tools/JSON subset.
GET models выдаёт только разрешённые published virtual slugs, не raw provider catalog.
Chat API usage — prompt/completion/total (+cache/reasoning details);
Responses usage — input/output/total. Internal TokenFacts не подменяет эти wire fields.
При incomplete/unknown usage native usage=null, детали доступны в own request trace.
Numbers не фабрикуются из длины строк. Tool arguments сохраняются байт/semantic-safe.

Не поддерживаются stateful previous_response_id/conversation, built-in remote tools,
background/Batch/WebSocket/media; unsupported fields →400 до I/O.
Response cache выключен. Store=false для stateless API; encrypted replay buffer —
отдельная private technical retention, не пользовательское persistent conversation.

## Statistics, expenses, export

/from,/to required, half-open range; grouping/filter enum и max range из ANALYTICS.
Responses выдаются одним authorized snapshot с as_of/watermark/lag и knownness.
CSV export имеет колонки request_id, attempt_id, occurred_at_utc, client_id, project,
virtual_slug, profile_revision, provider, requested_model, actual_model, status,
input_tokens,cached_input_tokens,output_tokens,currency,amount,confidence,source.
Money null — empty cell + unknown confidence. Values beginning =,+,-,@ в text cells
экранируются; numeric decimal amount column определяется schema, не string formula.

Prices immutable, generic API не принимает cost confidence как trusted input.
recordSubscription хранит metadata; paid status/receipt_ref требует independently
trusted receipt importer для confirmed ledger. requestLedgerCorrection принимает
original entry + trusted receipt reference/reason; сумма выводится importer,
caller не назначает произвольное confirmed списание. Duplicate receipt id deduped.

## Ошибки

Envelope {error:{code,message,request_id}}, optional Retry-After для 429.
400 invalid_payload/unsupported_parameter; 401 unauthenticated; 403 forbidden;
404 not_found; 409 idempotency_conflict/replay_expired/invalid_transition;
412 stale_revision; 422 capability_mismatch/context_limit/usage_invalid;
429 rate_limited/budget_exhausted; 502 provider_failure;
503 provider_unavailable/authorization_unknown/accounting_unavailable.
Post-dispatch ambiguous outcome хранится в request state unknown.
Upstream credential/body/error content маскируется; raw native failure не passthrough.
После начала SSE HTTP status не меняется: protocol error event + terminal state,
никакого fabricated completion. Error cases: TC-005/006/018/026/033.

## Полный design mapping

UI_SCREEN_SPEC/UI_FIELD_REFERENCE и design/design-contract.json перечисляют
all source/derived fields/actions. PriceInput.request_fee хранит nullable known fee,
не actual списание; DatasetInput case.tools фиксирует caller schemas для проверки.
Control one-time key replay при недоступном output:409 one_time_output_unavailable,
метаданные операции сохраняются; explicit rotation, no new implicit issue.

## Дополнительные target operations

| listModelContextPreferences | `GET /api/v1/connections/{connection_id}/model-contexts` |
| saveModelContextPreference | `PUT /api/v1/connections/{connection_id}/model-contexts` |
| listNamespaceBindings | `GET /api/v1/namespaces` |
| listNotifications | `GET /api/v1/notifications` |
| ackNotification | `POST /api/v1/notifications/{notification_id}/ack` |

[Версионирование](API_VERSIONING.md); [Namespace](contracts/NAMESPACE_V1.md). Query q/status/mode/action bounded и allowlisted. Namespace query pair validates grants, malformed input не сбрасывает фильтр. ACK actor-local и идемпотентен.

## Тарифы проектов и источники себестоимости

| Метод | Путь                    | Operation                   |
| ----- | ----------------------- | --------------------------- |
| GET   | /api/v1/pricing-sources | listPricingSources          |
| POST  | /api/v1/pricing-sources | createPricingSourceRevision |
| GET   | /api/v1/project-tariffs | listProjectTariffs          |
| POST  | /api/v1/project-tariffs | createProjectTariffRevision |
| GET   | /api/v1/project-charges | listProjectCharges          |

[PROJECT_TARIFFS](PROJECT_TARIFFS.md) определяет режимы, Namespace/CAS и decimal scale. Новые DTO входят в 0.5.0-design. Actual runtime not_run.

## Закрытие semantic review — 0.5.0-design

| Метод | Путь                                       | Operation               |
| ----- | ------------------------------------------ | ----------------------- |
| GET   | /api/v1/project-tariff-activations         | listTariffActivations   |
| POST  | /api/v1/project-tariff-activations         | activateProjectTariff   |
| GET   | /api/v1/evaluations/{evaluation_id}/scores | listManualScores        |
| POST  | /api/v1/evaluations/{evaluation_id}/scores | recordManualScore       |
| POST  | /internal/v1/service-inference             | serviceInference        |
| POST  | /internal/v1/delegations/revoke            | revokeServiceDelegation |

BudgetInput.scope_id — UUID; project scope обязательно имеет NamespaceRef, scope_id
равен namespace_id, Tracker target выводится из verified binding. Installation/profile
scope не принимает чужой Namespace; client scope выводится из exact client record.
NamespaceBinding содержит safe label/project_key; cursor обязателен в следующем page.
Exact GET model-contexts?model_id даёт ETag version или "0" при отсутствии; PUT
If-Match "0" создаёт только отсутствующий tuple, updates используют ETag; UUID
Idempotency-Key дедуплицирует side effect/proof invalidation. Cursor коллекции не ETag модели.
Manual scores проверяют существующий frozen candidate/case/repetition, own scope,
CAS и actor/time; финансы не изменяются. Новые typed errors входят в общий envelope.
Signed service wire: [SERVICE_ADAPTER_V1](contracts/SERVICE_ADAPTER_V1.md).

## Готовность реализации — 0.5.0-design

| Метод | Путь                                                      | Operation              |
| ----- | --------------------------------------------------------- | ---------------------- |
| POST  | /api/v1/project-tariff-activations/{activation_id}/cancel | cancelTariffActivation |

POST /api/v1/project-tariff-activations/{activation_id}/cancel — cancelTariffActivation,
control auth/UUID key/CAS/reason, sync 200 TariffActivation с cancellation witness.
Семантика 409/412/replay и timeline — [PROJECT_TARIFFS](PROJECT_TARIFFS.md).
ProjectCharge имеет cost_source_state и guarded nullable source UUID: unconfigured
не создаёт fake revision. StatisticsFilters общая для всех statistics/export paths,
с full dimensions/timezone и opaque IDs. ServiceInferenceInput discriminated по protocol;
ChatInput/ResponsesInput mismatch отклоняется до I/O. Runtime API пока отсутствует.
