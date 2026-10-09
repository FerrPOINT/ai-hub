# Реестр актуализации документов и дизайна

Дата: 2026-10-09. Статусы ниже относятся к документам и prototype. Runtime acceptance not_run.

| Граница / источник                                | Прежний пробел                                             | Решение / owner                                                                | Проверка                                                       |
| ------------------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| Admin /ai, AI_PROFILE_V1, миграции 0007/0009/0010 | Укрупнённый handoff без полей и хранилищ                   | 20 mapping rows в ADMIN_HANDOFF_V1 и admin-extraction-map; Hub/Admin           | check_alignment: missing row/unsafe policy/unknown operation   |
| Base NamespaceRef / resource_context              | Project string без registry identity                       | NamespaceRef + verified local projection, client/dataset/request bindings; Hub | UUID identity и соседний registry; IAB URL/tab/unbound/foreign |
| Base UI/UX Standard                               | Route class/fixture/auth отсутствовали                     | 15 operational + auth /login, ссылка на normative standard; Hub                | metadata gate и 219 geometry / 168 states                      |
| Провайдеры/модели/аудит                           | UI filters отсутствовали в API                             | Bounded q/status/mode/action и Namespace query pair; Hub                       | check_alignment + IAB combined filters                         |
| Admin model_contexts                              | Сохранённый context budget не описан в Hub                 | Own exact connection/model preference + CAS; Hub                               | IAB switch/save/unknown physical bounds                        |
| Notification outbox / UI                          | Чтение и actor ACK не описаны                              | Polling control API, notifications и actor acknowledgements; Hub               | IAB ACK сохраняет финансовые значения                          |
| Profile/client lifecycle                          | Макет не моделировал rollback/archive и несколько профилей | Immutable snapshots, guarded lifecycle и array grants; Hub                     | IAB old request/revision/rollback/archive/key revocation       |
| Consumer / versioning                             | Общий сценарий без recovery/examples                       | NAMESPACE_V1/CONSUMER_V1/API_VERSIONING, synthetic example; Hub                | OpenAPI/schema и planned TC-033/035                            |
| Budget/import/operations                          | Exact conversion и runbook недостаточно конкретны          | uint64 microdollars → decimal, provenance/reconcile/drain/stop gates; Hub      | positive/negative conversion tests; runtime drill future S7    |
| Base documentation discovery                      | ai-hub не зарегистрирован                                  | Explicit onboarding одного owner, catalog/inventory/compatibility; Base        | 18 sync tests; штатный mirror после owner commit               |

Трассировка требований: [TRACEABILITY](TRACEABILITY.md). Отдельные execution gates:
SSO, own PostgreSQL, adapters, financial ledger, actual consumer, restore/load и
реальный cutover не закрываются документационными и prototype проверками.
