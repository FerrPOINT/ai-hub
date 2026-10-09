# Закрытие замечаний полного ревью

Дата: 2026-10-09. Объём: документационные контракты и исполнимый дизайн-прототип.
Исходное ревью: AI Hub 7a2b53c8798270710959d121e2163f13aa5ec72e.
Решения ниже реализованы в проектном пакете; actual application/runtime acceptance не выполнялась.

| ID  | Граница                                     | Исправление                                                                                                                                   | Проверка                                                                                                |
| --- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| R01 | Namespace запусков и datasets               | Stable run/dataset ID и явная Namespace identity вместо поиска по тексту.                                                                     | same-name-cannot-cross-namespace; dataset-panel-namespace-isolation                                     |
| R02 | Dirty/history/proof                         | Back/Forward удерживает ввод; proof выдаётся только сохранённой версии.                                                                       | native-back-draft-guard; unsaved-proof-cannot-bypass-history                                            |
| R03 | Dataset/Tracker mapping                     | Tracker project derived из проверенной Namespace UUID-пары; отдельного выбора нет.                                                            | dataset-project-derived-from-namespace; check_semantics                                                 |
| R04 | Proof/generation/rollback                   | Proof привязан к config, generation, endpoint, adapter и context; rollback сверяет snapshot bindings.                                         | endpoint-requalification-not-profile-proof; profile-rollback                                            |
| R05 | Project budget identity                     | scope_id UUID + NamespaceRef, exact mapping, client scope derived отдельно.                                                                   | check_semantics; validate_contract positive/negative BudgetInput                                        |
| R06 | Полные тарифные формы                       | Точное connection/model/currency, несколько профилей, UTC intervals, собственные ставки.                                                      | source-price-model-neighbor; currency-rate-exact-tuple-no-fx                                            |
| R07 | Активация/интервалы                         | Immutable draft и отдельная CAS activation timeline; будущий тариф не меняет старые charges.                                                  | tariff-save-not-activation; tariff-scheduled-not-early; tariff-history-immutable                        |
| R08 | Pagination/labels                           | Bounded limit/cursor у Namespace/model-context lists, label и Tracker metadata в binding.                                                     | check_semantics; mutation-negative unit tests                                                           |
| R09 | Первый context PUT                          | Exact model ETag "0" для create, CAS для update, Idempotency-Key и original readback.                                                         | check_semantics; model-context-saved                                                                    |
| R10 | Cases/scorer/manual scores                  | tool_args совпадает с API; typed expected; frozen scorer/budget/revisions; attributed CAS manual scores.                                      | schema-tool-args-accepted; run-scorer-budget-and-identities-frozen; manual-score-attributed-and-visible |
| R11 | История и URL tabs                          | Archived/unavailable сохраняют разрешённую историю; admission закрыт; auth/expense tabs в URL.                                                | archive-history-readable; archive-new-admission-closed; expense-tabs-reload; managed-login-no-duplicate |
| R12 | Canonical export                            | Raw decimal/UTC/null и полный typed filter_echo из тех же facts; группировки не дублируют CI.                                                 | export-canonical-decimal; export-full-scope-utc-echo; Statistics example schema                         |
| R13 | Consumer revision identity                  | В примере opaque revision UUID, ordinal r12 только display label.                                                                             | check_semantics; service-contract negative ordinal vector                                               |
| R14 | Fleet/Forge transport и publication cohorts | Signed internal adapter + Hub key, raw body binding, lease/fencing/revocation и recovery. Docs branches отделены от main/SDK/operator cohort. | check_service_contract 12 offline vectors; SERVICE_ADAPTER_V1; runtime/main integration planned         |
| R15 | Метаданные/готовность                       | OpenAPI 0.5.0-design, 47 таблиц/420 полей, 16 operational + login; свежий source-bound evidence.                                              | check_docs; check_alignment; check_design; validate_contract                                            |

Финальная IAB матрица: 230 geometry / 179 states / 146 flow assertions; 74 native PNG, пять просмотрены непосредственно.
[Evidence](design/evidence.json), [QA](design/QA.md), [OpenAPI](contracts/openapi.v1.json).
Полный исходный script из IAB совпадает с локальным SHA 69ee74bbadef2f2ac26969c02f85b8762e9d2a369673829dfe6ab4f64e30c9b8.

## Встраивание в экосистему

Base владеет SDK/UI/Auth contracts; Admin — branding/service catalog/Namespace registry;
Central Auth — identity/PAT; Tracker — Task; Fleet — grants/agents/tools; Forge — repositories/workspaces.
Hub не выдаёт Fleet grants и не исполняет tools. Public SDK не требует Task/run.
Сервисный путь отдельно проверяет подписанный ExecutionContextV2 и пределы делегирования.
Provider cost, расчётная цена подписки и начисление проекту остаются разными фактами.
Default 20% и ставки за 1M не создают выдуманный provider receipt.

## Отдельные будущие gates

Документационные branches Base/Admin содержат Target approved и будущий переход /ai.
Это не интеграция этих branches в main и не runtime registration. Их Namespace cohort
также ещё не принят main; нельзя молча включать его вместе с Hub docs или менять SDK pin.
Правила нового signed adapter проектные; actual Rust verifier, trusted issuers, live revocation,
SSO, PostgreSQL и финансовая сверка требуют S1–S7. Dispatch/runtime_ready не включаются по offline vector.
Реальный cutover требует backup, mappings, квалификации Hub/consumers, financial reconciliation
и rollback; действующий Admin /ai сохраняется до этой вехи. После cutover ровно один config editor.

## Финальное readiness closure

[READINESS_AUDIT](READINESS_AUDIT.md) фиксирует READY-01–05: early financial prerequisites, unconfigured/late-receipt distinction, cancellation/CAS, canonical filters и protocol union. Числа и evidence текущего результата — [QA](design/QA.md). Actual application execution remains not_run.
