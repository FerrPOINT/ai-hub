# Поля форм и payload mapping

Канонический schema — [OpenAPI](contracts/openapi.v1.json). Прототип использует demo IDs; реальные DTO используют opaque UUID. Derived поля не являются caller grants.

## connection — createConnection

Schema: ConnectionInput.

| Поле                | Тип    | Required | Ограничения                                                 |
| ------------------- | ------ | -------- | ----------------------------------------------------------- |
| display_name        | string | да       | `{"minLength": 1, "maxLength": 120}`                        |
| endpoint_policy_ref | string | да       | `{"minLength": 1, "maxLength": 120}`                        |
| billing_mode        | string | да       | `{"enum": ["metered", "subscription", "local", "unknown"]}` |

Derived: provider path kind.
Результат: Connection/draft; authorization separate.

## credential — writeCredential

Schema: CredentialInput.

| Поле   | Тип    | Required | Ограничения                                               |
| ------ | ------ | -------- | --------------------------------------------------------- |
| secret | string | да       | `{"minLength": 1, "maxLength": 16384, "writeOnly": true}` |

Derived: expected_generation from exact connection.
Результат: Operation; no secret readback.

## profile — createVirtualModel

Schema: ProfileInput.

| Поле                  | Тип                  | Required | Ограничения                                                                                                                                    |
| --------------------- | -------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| slug                  | string               | да       | `{"pattern": "^[a-z][a-z0-9-]{1,62}$"}`                                                                                                        |
| display_name          | string               | да       | `{"minLength": 1, "maxLength": 120}`                                                                                                           |
| mode                  | string               | да       | `{"enum": ["development", "pinned_test"]}`                                                                                                     |
| deployments           | array                | да       | `{"items": {"$ref": "#/components/schemas/DeploymentInput"}, "minItems": 1, "maxItems": 5}`                                                    |
| parameters            | GenerationParameters | да       | `{}`                                                                                                                                           |
| input_limit           | integer              | да       | `{"minimum": 1}`                                                                                                                               |
| output_limit          | integer              | да       | `{"minimum": 1}`                                                                                                                               |
| context_limit         | integer              | да       | `{"minimum": 1}`                                                                                                                               |
| required_capabilities | array                | да       | `{"items": {"type": "string"}}`                                                                                                                |
| allowed_overrides     | array                | нет      | `{"items": {"type": "string", "enum": ["temperature", "top_p", "max_output_tokens", "reasoning_effort"]}, "default": [], "uniqueItems": true}` |
| timeout_seconds       | integer              | да       | `{"minimum": 1, "maximum": 600, "default": 120}`                                                                                               |
| max_attempts          | integer              | да       | `{"minimum": 1, "maximum": 5, "default": 3}`                                                                                                   |

Derived: verified actor/operation metadata.
Результат: Draft; proof not required for create/save.

## profileProof — verifyVirtualModelDraft

Schema: ProfileVerificationInput.

| Поле             | Тип    | Required | Ограничения          |
| ---------------- | ------ | -------- | -------------------- |
| budget_policy_id | string | да       | `{"format": "uuid"}` |

Derived: expected_draft_version, If-Match.
Результат: Own profile_draft proof, exact hash/deployments.

## publish — publishRevision

Schema: PublishInput.

| Поле | Тип | Required | Ограничения |
| ---- | --- | -------- | ----------- |

Derived: expected_draft_version, profile_verification_id, If-Match.
Результат: Immutable active revision + audit.

## price — createPriceRevision

Schema: PriceInput.

| Поле            | Тип                | Required | Ограничения                                                                                                                 |
| --------------- | ------------------ | -------- | --------------------------------------------------------------------------------------------------------------------------- |
| connection_id   | string             | да       | `{"format": "uuid"}`                                                                                                        |
| model_id        | string             | да       | `{}`                                                                                                                        |
| tier            | string             | да       | `{}`                                                                                                                        |
| currency        | string             | да       | `{"pattern": "^[A-Z]{3}$"}`                                                                                                 |
| input_uncached  | string             | да       | `{"pattern": "^(0\|[1-9][0-9]{0,17})(\\.[0-9]{1,12})?$"}`                                                                   |
| input_cached    | oneOf              | да       | `{"oneOf": [{"type": "string", "pattern": "^(0\|[1-9][0-9]{0,17})(\\.[0-9]{1,12})?$"}, {"type": "null"}]}`                  |
| output_billable | string             | да       | `{"pattern": "^(0\|[1-9][0-9]{0,17})(\\.[0-9]{1,12})?$"}`                                                                   |
| request_fee     | oneOf              | нет      | `{"oneOf": [{"type": "string", "pattern": "^(0\|[1-9][0-9]{0,19})(\\.[0-9]{1,18})?$"}, {"type": "null"}], "default": null}` |
| effective_from  | string             | да       | `{"format": "date-time"}`                                                                                                   |
| effective_to    | ['string', 'null'] | да       | `{"format": "date-time"}`                                                                                                   |
| source          | string             | да       | `{}`                                                                                                                        |

Derived: unit=per_million_tokens.
Результат: Immutable new rates; historical facts not rewritten.

## subscription — recordSubscription

Schema: SubscriptionInput.

| Поле           | Тип                | Required | Ограничения                                               |
| -------------- | ------------------ | -------- | --------------------------------------------------------- |
| connection_id  | string             | да       | `{"format": "uuid"}`                                      |
| period_start   | string             | да       | `{"format": "date-time"}`                                 |
| period_end     | string             | да       | `{"format": "date-time"}`                                 |
| amount         | string             | да       | `{"pattern": "^(0\|[1-9][0-9]{0,19})(\\.[0-9]{1,18})?$"}` |
| currency       | string             | да       | `{"pattern": "^[A-Z]{3}$"}`                               |
| payment_status | string             | да       | `{"enum": ["planned", "pending", "paid", "unknown"]}`     |
| receipt_ref    | ['string', 'null'] | да       | `{}`                                                      |

Derived: verified actor/operation metadata.
Результат: Metadata pending trust; body cannot assign confirmed charge.

## correction — requestLedgerCorrection

Schema: CorrectionInput.

| Поле                | Тип    | Required | Ограничения                          |
| ------------------- | ------ | -------- | ------------------------------------ |
| original_entry_id   | string | да       | `{"format": "uuid"}`                 |
| trusted_receipt_ref | string | да       | `{}`                                 |
| reason              | string | да       | `{"minLength": 1, "maxLength": 240}` |

Derived: Amount/trust from importer, not user.
Результат: Append-only correction/settlement.

## budget — createBudget

Schema: BudgetInput.

| Поле               | Тип    | Required | Ограничения                                                                                                      |
| ------------------ | ------ | -------- | ---------------------------------------------------------------------------------------------------------------- |
| scope_type         | string | да       | `{"enum": ["installation", "project", "client", "profile"]}`                                                     |
| scope_id           | string | да       | `{}`                                                                                                             |
| currency           | string | да       | `{"pattern": "^[A-Z]{3}$"}`                                                                                      |
| period             | string | да       | `{"enum": ["utc_day", "utc_month"]}`                                                                             |
| hard_limit         | string | да       | `{"pattern": "^(0\|[1-9][0-9]{0,19})(\\.[0-9]{1,18})?$"}`                                                        |
| warning_thresholds | array  | да       | `{"items": {"type": "integer", "minimum": 1, "maximum": 99}, "minItems": 1, "maxItems": 5, "default": [80, 95]}` |

Derived: verified actor/operation metadata.
Результат: Policy + current canonical period.

## client — createClient

Schema: ClientInput.

| Поле                      | Тип     | Required | Ограничения                                                                                                                                                     |
| ------------------------- | ------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| application_key           | string  | да       | `{"minLength": 1, "maxLength": 120}`                                                                                                                            |
| project_binding           | string  | да       | `{}`                                                                                                                                                            |
| allowed_virtual_model_ids | array   | да       | `{"items": {"type": "string", "format": "uuid"}, "maxItems": 100}`                                                                                              |
| expires_at                | string  | да       | `{"format": "date-time"}`                                                                                                                                       |
| max_concurrency           | integer | да       | `{"minimum": 1, "maximum": 100}`                                                                                                                                |
| max_requests_per_minute   | integer | да       | `{"minimum": 1, "maximum": 10000}`                                                                                                                              |
| scopes                    | array   | нет      | `{"items": {"type": "string", "enum": ["infer", "read_own_usage", "read_result"]}, "minItems": 1, "uniqueItems": true, "default": ["infer", "read_own_usage"]}` |
| cost_policy               | string  | нет      | `{"enum": ["budget_guaranteed", "cost_unknown_allowed"], "default": "budget_guaranteed"}`                                                                       |
| namespace                 | oneOf   | нет      | `{"oneOf": [{"$ref": "#/components/schemas/NamespaceRef"}, {"type": "null"}], "default": null}`                                                                 |

Derived: verified actor/operation metadata.
Результат: Client metadata; issueClientKey explicit separate operation.

## dataset — createDatasetVersion

Schema: DatasetInput.

| Поле            | Тип    | Required | Ограничения                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --------------- | ------ | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| name            | string | да       | `{"minLength": 1, "maxLength": 120}`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| project_binding | string | да       | `{}`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| cases           | array  | да       | `{"minItems": 1, "maxItems": 1000, "items": {"type": "object", "additionalProperties": false, "properties": {"case_id": {"type": "string"}, "input": {"type": "string", "maxLength": 32768}, "assertion_kind": {"type": "string", "enum": ["exact_match", "json_schema", "tool_args", "manual"]}, "expected": {"type": "string", "maxLength": 32768}, "tools": {"type": "array", "maxItems": 64, "items": {"type": "object", "additionalProperties": false, "properties": {"type": {"type": "string", "const": "function"}, "function": {"$ref": "#/components/schemas/FunctionDefinition"}}, "required": ["type", "function"]}, "default": []}}, "required": ["case_id", "input", "assertion_kind", "expected"]}}` |
| namespace       | oneOf  | нет      | `{"oneOf": [{"$ref": "#/components/schemas/NamespaceRef"}, {"type": "null"}], "default": null}`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

Derived: hash/version generated by owner.
Результат: Immutable dataset.

## evaluation — startEvaluation

Schema: EvaluationInput.

| Поле                 | Тип     | Required | Ограничения                                                                      |
| -------------------- | ------- | -------- | -------------------------------------------------------------------------------- |
| dataset_version_id   | string  | да       | `{"format": "uuid"}`                                                             |
| profile_revision_ids | array   | да       | `{"items": {"type": "string", "format": "uuid"}, "minItems": 1, "maxItems": 10}` |
| repetitions          | integer | да       | `{"minimum": 1, "maximum": 10, "default": 1}`                                    |
| max_concurrency      | integer | да       | `{"minimum": 1, "maximum": 8, "default": 1}`                                     |
| budget_policy_id     | string  | да       | `{"format": "uuid"}`                                                             |
| scorer_revision      | string  | да       | `{}`                                                                             |

Derived: actor/validated internal execution grant.
Результат: Frozen evaluation and own expenses.

## modelContext — saveModelContextPreference

Schema: ModelContextPreferenceInput.

| Поле                  | Тип     | Required | Ограничения                             |
| --------------------- | ------- | -------- | --------------------------------------- |
| model_id              | string  | да       | `{"minLength": 1, "maxLength": 120}`    |
| context_window_tokens | integer | да       | `{"minimum": 1, "maximum": 4294967295}` |

Derived: connection_id, If-Match.
Результат: Saved exact connection/model preference; not physical limit.

## Семантическая проверка

Namespace/project binding сверяется с owner projection. Client Namespace сменяется только после отзыва прежнего key. Profiles — массив UUID, модельный бюджет — exact connection/model preference. Physical unknown не заменяется default. Money — строки decimal; параметры генерации отдельно от финансовой арифметики. Invalid fields связаны с errors и сохраняют ввод.
