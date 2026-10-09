# Документация AI Hub

Начать с TZ/PRODUCT_REQUIREMENTS/CURRENT_STATE, затем IMPLEMENTATION_PLAN.
Документы describe planned v1 и sufficient development inputs; runtime не реализован.

| Документ                                                             | Назначение                                    |
| -------------------------------------------------------------------- | --------------------------------------------- |
| [PRODUCT_REQUIREMENTS.md](PRODUCT_REQUIREMENTS.md)                   | FR/NFR и criteria                             |
| [TZ.md](TZ.md)                                                       | назначение и user scope                       |
| [CURRENT_STATE.md](CURRENT_STATE.md)                                 | фактический статус                            |
| [PRE_DEVELOPMENT_GATE.md](PRE_DEVELOPMENT_GATE.md)                   | проверка готовности начать S1                 |
| [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md)                     | компоненты/зависимости/вертикальные stages    |
| [ROADMAP.md](ROADMAP.md)                                             | milestones и future scope                     |
| [DOMAIN_MODEL.md](DOMAIN_MODEL.md)                                   | термины/инварианты                            |
| [ARCHITECTURE.md](ARCHITECTURE.md)                                   | поток/owners/transaction boundaries           |
| [DATA_MODEL.md](DATA_MODEL.md)                                       | таблицы/constraints/индексы                   |
| [MIGRATIONS.md](MIGRATIONS.md)                                       | schema lifecycle                              |
| [API.md](API.md)                                                     | operations/errors/versioning                  |
| [contracts/openapi.v1.json](contracts/openapi.v1.json)               | typed draft, not runtime-generated            |
| [contracts/INFERENCE_V1.md](contracts/INFERENCE_V1.md)               | wire/stream/tools/unknown contract            |
| [PROVIDERS.md](PROVIDERS.md)                                         | adapter/account qualification                 |
| [ROUTING.md](ROUTING.md)                                             | draft/revision/fallback/pinned                |
| [ACCOUNTING.md](ACCOUNTING.md)                                       | денежные facts/rates/subscriptions/budgets    |
| [ANALYTICS.md](ANALYTICS.md)                                         | метрики/snapshot/filters/export               |
| [EVALUATIONS.md](EVALUATIONS.md)                                     | dataset/run/comparison                        |
| [UI_UX.md](UI_UX.md)                                                 | scenarios/states/actions                      |
| [ui-routes.json](ui-routes.json)                                     | routes/operation mapping                      |
| [FRONTEND_ARCHITECTURE.md](FRONTEND_ARCHITECTURE.md)                 | UI state/client boundaries                    |
| [BASE_INTEGRATION.md](BASE_INTEGRATION.md)                           | SDK/auth/service integration                  |
| [contracts/ADMIN_HANDOFF_V1.md](contracts/ADMIN_HANDOFF_V1.md)       | owner переход без runtime mutations           |
| [SECURITY.md](SECURITY.md)                                           | principal/vault/retention                     |
| [THREAT_MODEL.md](THREAT_MODEL.md)                                   | контроль/negative tests                       |
| [ENV.md](ENV.md)                                                     | target configuration                          |
| [LOCAL_SETUP.md](LOCAL_SETUP.md)                                     | source/dev и будущий actual bootstrap         |
| [DEPLOYMENT.md](DEPLOYMENT.md)                                       | candidate/app-only rollout/rollback           |
| [BACKUP_RESTORE.md](BACKUP_RESTORE.md)                               | own nonempty rehearsal                        |
| [OPERATIONS.md](OPERATIONS.md)                                       | diagnosis/reconciliation/notification actions |
| [MONITORING.md](MONITORING.md)                                       | health/metrics/alerts                         |
| [STANDARDS.md](STANDARDS.md)                                         | backend/API/DB/frontend/review conventions    |
| [CI_CD.md](CI_CD.md)                                                 | docs-first и future workflow                  |
| [TESTING.md](TESTING.md)                                             | behavioral oracles                            |
| [QUALITY_GATE.md](QUALITY_GATE.md)                                   | local/build/DB/runtime/evidence gates         |
| [TRACEABILITY.md](TRACEABILITY.md)                                   | requirements/test/phase links                 |
| [traceability.json](traceability.json)                               | machine-readable coverage                     |
| [RISK_REGISTER.md](RISK_REGISTER.md)                                 | dependency/risk checkpoints                   |
| [ADR_INDEX.md](ADR_INDEX.md)                                         | принятые project decisions                    |
| [SOURCE_AUDIT.md](SOURCE_AUDIT.md)                                   | точные source/official references             |
| [source-audit.json](source-audit.json)                               | source metadata/hashes                        |
| [RELEASE.md](RELEASE.md)                                             | первый полный integrated acceptance           |
| [examples/accounting-example.json](examples/accounting-example.json) | synthetic decimal oracle                      |

## Соответствие категориям остальных сервисов

Base root matrix полностью сохранена; reference fleet docs map сгруппирован без
дублирования rules: API/version/error → API+INFERENCE; DB/index → DATA_MODEL+
MIGRATIONS; frontend standards → FRONTEND_ARCHITECTURE+STANDARDS;
system admin/ops/resilience → DEPLOYMENT+OPERATIONS+BACKUP_RESTORE;
testing/review/performance → TESTING+QUALITY_GATE+STANDARDS.
Screenshots/real builds добавляются после actual app, не placeholders.
Single owner specification каждой нормы указан в TRACEABILITY.

## Полный design/development handoff

- [DEVELOP_READY](DEVELOP_READY.md)
- [Дизайн и прототип](design/README.md)
- [Система UI](DESIGN_SYSTEM.md), [экраны](UI_SCREEN_SPEC.md), [поля](UI_FIELD_REFERENCE.md)
- [Use cases](USE_CASES.md), [доступ](ACCESS_MATRIX.md), [ошибки](ERROR_CATALOG.md)
- [Состояния](STATE_MACHINES.md), [reconciliation](RECONCILIATION.md)
- [Typed data dictionary](DATA_DICTIONARY.md), [migration plan](MIGRATION_PLAN.md)
- [Implementation packets](IMPLEMENTATION_PACKETS.md), [design acceptance](DESIGN_ACCEPTANCE.md)
- [Rendered QA](design/QA.md), [галерея](design/gallery.html)

## Актуализация и выделение из Admin

[Namespace](contracts/NAMESPACE_V1.md), [Consumer](contracts/CONSUMER_V1.md), [версии API](API_VERSIONING.md), [руководство](USER_GUIDE.md). Typed OpenAPI, DD и UI обновляются вместе; operational acceptance остаётся not_run.

[Реестр актуализации](ALIGNMENT_REGISTER.md) связывает источники, устранённые пробелы и проверки.

## Тарифы проектов

[PROJECT_TARIFFS](PROJECT_TARIFFS.md): per-million цены, OpenRouter auto usage/cost, ручные Ollama Online/ChatGPT rates, default 20%, Namespace overrides и immutable начисления.

[SERVICE_ADAPTER_V1](contracts/SERVICE_ADAPTER_V1.md) — target signed wire Fleet/Forge, а не расширение public metadata или permission на runtime starts.

- [Закрытие полного ревью](REVIEW_RESOLUTION.md) — R01–R15, проверенный prototype и отдельные main/runtime gates.

- [ADR-0009 — readiness contract](adr/0009-implementation-readiness.md) — закрытие READY-01–05 перед реализацией.

- [Готовность перед реализацией](READINESS_AUDIT.md) — завершённый requirements/contract/data/design audit и future execution boundaries.
