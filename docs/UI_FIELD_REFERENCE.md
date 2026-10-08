# Поля форм и payload mapping

Canonical schema — draft OpenAPI; этот reference экспортирован из design contract.
Derived fields не редактируются пользователем. Runtime DTO/validation ещё не реализованы.
Native prototype constraints иллюстрируют UX; server authorization/validation обязательны.

## connection — createConnection

Body schema: ConnectionInput.

| Поле                | Тип    | Required | Constraints / default                                       |
| ------------------- | ------ | -------- | ----------------------------------------------------------- |
| display_name        | string | да       | `{"minLength": 1, "maxLength": 120}`                        |
| endpoint_policy_ref | string | да       | `{"minLength": 1, "maxLength": 120}`                        |
| billing_mode        | string | да       | `{"enum": ["metered", "subscription", "local", "unknown"]}` |

Derived: provider path kind
Result: Connection/draft; authorization separate.

## credential — writeCredential

Body schema: CredentialInput.

| Поле   | Тип    | Required | Constraints / default                  |
| ------ | ------ | -------- | -------------------------------------- |
| secret | string | да       | `{"minLength": 1, "maxLength": 16384}` |

Derived: expected_generation from exact connection
Result: Operation; no secret readback.

## profile — createVirtualModel

Body schema: ProfileInput.

| Поле                  | Тип                  | Required | Constraints / default                            |
| --------------------- | -------------------- | -------- | ------------------------------------------------ |
| slug                  | string               | да       | `{"pattern": "^[a-z][a-z0-9-]{1,62}$"}`          |
| display_name          | string               | да       | `{"minLength": 1, "maxLength": 120}`             |
| mode                  | string               | да       | `{"enum": ["development", "pinned_test"]}`       |
| deployments           | array                | да       | `{"minItems": 1, "maxItems": 5}`                 |
| parameters            | GenerationParameters | да       | `{}`                                             |
| input_limit           | integer              | да       | `{"minimum": 1}`                                 |
| output_limit          | integer              | да       | `{"minimum": 1}`                                 |
| context_limit         | integer              | да       | `{"minimum": 1}`                                 |
| required_capabilities | array                | да       | `{}`                                             |
| allowed_overrides     | array                | нет      | `{"default": []}`                                |
| timeout_seconds       | integer              | да       | `{"minimum": 1, "maximum": 600, "default": 120}` |
| max_attempts          | integer              | да       | `{"minimum": 1, "maximum": 5, "default": 3}`     |

Derived: только actor/operation metadata.
Result: Draft; proof not required for create/save.

## profileProof — verifyVirtualModelDraft

Body schema: ProfileVerificationInput.

| Поле             | Тип    | Required | Constraints / default |
| ---------------- | ------ | -------- | --------------------- |
| budget_policy_id | string | да       | `{"format": "uuid"}`  |

Derived: expected_draft_version; If-Match
Result: Own profile_draft proof, exact hash/deployments.

## publish — publishRevision

Body schema: PublishInput.

| Поле | Тип | Required | Constraints / default |
| ---- | --- | -------- | --------------------- |

Derived: expected_draft_version; profile_verification_id; If-Match
Result: Immutable active revision + audit.

## price — createPriceRevision

Body schema: PriceInput.

| Поле            | Тип           | Required | Constraints / default                                     |
| --------------- | ------------- | -------- | --------------------------------------------------------- |
| connection_id   | string        | да       | `{"format": "uuid"}`                                      |
| model_id        | string        | да       | `{}`                                                      |
| tier            | string        | да       | `{}`                                                      |
| currency        | string        | да       | `{"pattern": "^[A-Z]{3}$"}`                               |
| input_uncached  | string        | да       | `{"pattern": "^(0\|[1-9][0-9]{0,17})(\\.[0-9]{1,12})?$"}` |
| input_cached    | string / null | да       | `{}`                                                      |
| output_billable | string        | да       | `{"pattern": "^(0\|[1-9][0-9]{0,17})(\\.[0-9]{1,12})?$"}` |
| request_fee     | string / null | нет      | `{"default": null}`                                       |
| effective_from  | string        | да       | `{"format": "date-time"}`                                 |
| effective_to    | string / null | да       | `{"format": "date-time"}`                                 |
| source          | string        | да       | `{}`                                                      |

Derived: unit=per_million_tokens
Result: Immutable new rates; historical facts not rewritten.

## subscription — recordSubscription

Body schema: SubscriptionInput.

| Поле           | Тип           | Required | Constraints / default                                     |
| -------------- | ------------- | -------- | --------------------------------------------------------- |
| connection_id  | string        | да       | `{"format": "uuid"}`                                      |
| period_start   | string        | да       | `{"format": "date-time"}`                                 |
| period_end     | string        | да       | `{"format": "date-time"}`                                 |
| amount         | string        | да       | `{"pattern": "^(0\|[1-9][0-9]{0,19})(\\.[0-9]{1,18})?$"}` |
| currency       | string        | да       | `{"pattern": "^[A-Z]{3}$"}`                               |
| payment_status | string        | да       | `{"enum": ["planned", "pending", "paid", "unknown"]}`     |
| receipt_ref    | string / null | да       | `{}`                                                      |

Derived: только actor/operation metadata.
Result: Metadata pending trust; body cannot assign confirmed charge.

## correction — requestLedgerCorrection

Body schema: CorrectionInput.

| Поле                | Тип    | Required | Constraints / default                |
| ------------------- | ------ | -------- | ------------------------------------ |
| original_entry_id   | string | да       | `{"format": "uuid"}`                 |
| trusted_receipt_ref | string | да       | `{}`                                 |
| reason              | string | да       | `{"minLength": 1, "maxLength": 240}` |

Derived: Amount/trust from importer, not user
Result: Append-only correction/settlement.

## budget — createBudget

Body schema: BudgetInput.

| Поле               | Тип    | Required | Constraints / default                                        |
| ------------------ | ------ | -------- | ------------------------------------------------------------ |
| scope_type         | string | да       | `{"enum": ["installation", "project", "client", "profile"]}` |
| scope_id           | string | да       | `{}`                                                         |
| currency           | string | да       | `{"pattern": "^[A-Z]{3}$"}`                                  |
| period             | string | да       | `{"enum": ["utc_day", "utc_month"]}`                         |
| hard_limit         | string | да       | `{"pattern": "^(0\|[1-9][0-9]{0,19})(\\.[0-9]{1,18})?$"}`    |
| warning_thresholds | array  | да       | `{"minItems": 1, "maxItems": 5, "default": [80, 95]}`        |

Derived: только actor/operation metadata.
Result: Policy + current canonical period.

## client — createClient

Body schema: ClientInput.

| Поле                      | Тип     | Required | Constraints / default                                                                     |
| ------------------------- | ------- | -------- | ----------------------------------------------------------------------------------------- |
| application_key           | string  | да       | `{"minLength": 1, "maxLength": 120}`                                                      |
| project_binding           | string  | да       | `{}`                                                                                      |
| allowed_virtual_model_ids | array   | да       | `{"maxItems": 100}`                                                                       |
| expires_at                | string  | да       | `{"format": "date-time"}`                                                                 |
| max_concurrency           | integer | да       | `{"minimum": 1, "maximum": 100}`                                                          |
| max_requests_per_minute   | integer | да       | `{"minimum": 1, "maximum": 10000}`                                                        |
| scopes                    | array   | нет      | `{"minItems": 1, "default": ["infer", "read_own_usage"]}`                                 |
| cost_policy               | string  | нет      | `{"enum": ["budget_guaranteed", "cost_unknown_allowed"], "default": "budget_guaranteed"}` |

Derived: только actor/operation metadata.
Result: Client metadata; issueClientKey explicit separate operation.

## dataset — createDatasetVersion

Body schema: DatasetInput.

| Поле            | Тип    | Required | Constraints / default                |
| --------------- | ------ | -------- | ------------------------------------ |
| name            | string | да       | `{"minLength": 1, "maxLength": 120}` |
| project_binding | string | да       | `{}`                                 |
| cases           | array  | да       | `{"minItems": 1, "maxItems": 1000}`  |

Derived: hash/version generated by owner
Result: Immutable dataset.

## evaluation — startEvaluation

Body schema: EvaluationInput.

| Поле                 | Тип     | Required | Constraints / default                         |
| -------------------- | ------- | -------- | --------------------------------------------- |
| dataset_version_id   | string  | да       | `{"format": "uuid"}`                          |
| profile_revision_ids | array   | да       | `{"minItems": 1, "maxItems": 10}`             |
| repetitions          | integer | да       | `{"minimum": 1, "maximum": 10, "default": 1}` |
| max_concurrency      | integer | да       | `{"minimum": 1, "maximum": 8, "default": 1}`  |
| budget_policy_id     | string  | да       | `{"format": "uuid"}`                          |
| scorer_revision      | string  | да       | `{}`                                          |

Derived: actor/validated internal execution grant
Result: Frozen evaluation and own expenses.

## Header и field-error conventions

Idempotency-Key — UUID на новый user intent, stable при readback; ETag/If-Match
берётся из exact config response. Principal/project provenance не caller свободный ввод.
Decimal amounts strings: financial18 decimals/rates12; no browser float arithmetic.
UTC intervals [from,to), nullable unknown не empty zero. Bounds/capabilities конфликтуют
до external I/O. Invalid fields aria-invalid + associated error; input retained.
One-time key generation — отдельная explicit operation, no hidden issue after save.
