# Выделение AI из Admin в AI Hub

Статус: Target approved для документации/design. Реальный cutover не выполнялся.
Источник Admin: `7c4240a2154478b4866661cbc45dbd06d3e855e2`, `frontend/src/pages/live/ai.tsx`, `shared/api/ai.ts`, AI_PROFILE_V1, миграции 0007/0009/0010 и backend/ai-runtime.
Канонический реестр: [admin-extraction-map](admin-extraction-map.json). Нынешние Admin ADR-0018/0020 продолжают действовать до accepted cutover.

## Карта экранов, полей и хранилищ

| ID      | Исходный раздел/поверхность                                       | Поля и данные                                                                                                   | Назначение Hub / operation                                                                            | Сохранение                                                                      |
| ------- | ----------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| UI-01   | Активный AI-профиль · `/ai`                                       | `profile.provider/model/context_window_tokens/revision`                                                         | models · `readVirtualModel`                                                                           | Неизменяемый снимок; начатый execution сохраняет старую версию                  |
| UI-02   | Бюджет платной приёмки · `/ai`                                    | `limit/settled/reserved/uncertain/available_microdollars; unsettled/uncertain_requests; blocked_reason`         | budgets/expenses · `readStatisticsExpenses`                                                           | Точный decimal; неизвестный резерв удерживается                                 |
| UI-03   | Подключения · `/ai?provider=chatgpt\|openrouter`                  | `provider/model/context_window_tokens; connected/runtime_available/capabilities_verified`                       | providers · `listProviders`                                                                           | OpenRouter остаётся явным preset; каталог не является proof                     |
| UI-04   | Подключение · `/ai`                                               | `OpenRouter credential; ChatGPT operation_id/login_id/user_code/verification_url/expires_at/status`             | provider authorisation · `writeCredential/startManagedLogin/cancelOperation`                          | Write-only; собственная авторизация; повторный вход по умолчанию                |
| UI-05   | Модель и контекст · `/ai`                                         | `catalog model id/name/context_limit_tokens/max_output_tokens; model_contexts`                                  | catalog/model preferences · `readModelCatalog/listModelContextPreferences/saveModelContextPreference` | Точный model ID; тысячи ×1000; null limit не заменяется 256K                    |
| UI-06   | Сохранить черновик · `/ai`                                        | `draft_revision; model/context_window_tokens; If-Match`                                                         | model draft · `updateVirtualModelDraft`                                                               | CAS; изменение draft инвалидирует proof, даже при возврате значений             |
| UI-07   | Проверить модель / Сделать активным · `/ai`                       | `verification_id; draft binding/generation/TTL; publication operation`                                          | model verify/publish · `verifyVirtualModelDraft/publishRevision/readOperation`                        | В текущем Admin UI недоступно; qualification не наследуется                     |
| UI-08   | Незавершённая операция · `/ai`                                    | `operation_id/provider/kind/status; pending login operation`                                                    | operation readback · `readOperation`                                                                  | Тот же ID; запись, logout и потенциально платный вызов не дублируются           |
| DB-01   | Provider settings · `0007_ai_profiles.sql`                        | `ai_provider_settings(workspace,provider,model,context_window_tokens,draft_revision,updated_at)`                | connections/drafts · `createConnection/updateVirtualModelDraft`                                       | Явный mapping workspace → installation                                          |
| DB-02   | Immutable publications · `0007_ai_profiles.sql`                   | `ai_profile_revisions(profile,credential_generation,operation_id,published_by,published_at); ai_active_profile` | profile history · `listRevisions`                                                                     | Исходная revision связана с новым собственным ID; активация после target proof  |
| DB-03   | Publication outbox · `0009_ai_publication_outbox.sql`             | `ai_publication_outbox; pending candidate; registration readback`                                               | control operation journal · `readOperation`                                                           | Исходный pending завершается или уточняется до cutover                          |
| DB-04   | Per-model budgets · `0010_ai_model_contexts.sql`                  | `ai_model_contexts(workspace,provider,model,context_window_tokens,updated_at)`                                  | context preferences · `saveModelContextPreference`                                                    | Уникальная source tuple; Namespace не ищется по имени                           |
| RT-01   | Encrypted vault/managed login · `backend/ai-runtime`              | `encrypted state; generation; auth checkpoint; own key/CODEX_HOME`                                              | own vault/managed adapter · `writeCredential/startManagedLogin`                                       | Global HOME не импортируется; ciphertext не включён в обычный export            |
| RT-02   | Acceptance ledger · `backend/ai-runtime/src/budget.rs`            | `limit/settled/reserved/uncertain counters; blocked reason; original charge/request identities`                 | financial import · `requestLedgerCorrection`                                                          | Агрегат не создаёт историю receipt                                              |
| RT-03   | Execution journal · `backend/ai-runtime/src/inference_journal.rs` | `intent; attempt; sequence/readback; scoped grant binding; sidecar retention`                                   | requests/attempts · `readRequest/readRequestResult`                                                   | Исходные IDs и dedupe; reasoning остаётся закрытым; повторного dispatch нет     |
| RT-04   | Provider transport/policies · `backend/ai-runtime`                | `OpenRouter endpoint/max_price; native binary/version/tool policy; cached/reasoning/tool continuation`          | qualified adapters · `startVerification`                                                              | Переиспользуются требования и fixtures; target adapter квалифицируется отдельно |
| KEEP-01 | Identity/PAT · `/users /tokens`                                   | `central identity/session/PAT grants`                                                                           | Central Auth/Admin · `retained`                                                                       | Hub не владеет паролями и central sessions                                      |
| KEEP-02 | Namespace registry · `/namespaces`                                | `Namespace lifecycle/owner ACK`                                                                                 | Admin · `retained`                                                                                    | Hub владеет только verified binding; нового ResourceKind нет                    |
| KEEP-03 | Branding/service catalogue · `/branding /services /runtime`       | `published branding/service metadata/integration health`                                                        | Admin · `retained`                                                                                    | После cutover только состояние и переход                                        |
| KEEP-04 | Agent roles/Task/tools · `Fleet/Tracker/Workflow`                 | `grant issuer/lease/agent role/Task transitions/tool executor`                                                  | original owners · `retained`                                                                          | Hub проверяет binding; Fleet grant не превращается в общий Hub key              |

## Преобразование и происхождение

Исходный workspace связывается с целевым installation UUID явным manifest mapping; не Namespace.
Provider/model prefs связываются с exact own connection/upstream ID. Context UI ×1000 выполняется целочисленно; сохранённые токены передаются без округления.
Настроенный budget не physical bound: null metadata требует qualification, даже при saved 256000.
OpenRouter остаётся именованным preset openai_compatible с собственной endpoint/pricing/tool qualification.
Legacy microdollars — uint64 decimal string. Exact amount = value / 1000000, включая 1→0.000001 и max uint64→18446744073709.551615.
Передача агрегатов без source facts допустима только как явно marked legacy opening projection; она не создаёт фиктивные receipts или confirmed history.
Каждая import identity содержит source instance/kind/record ID/digest и target mapping. Duplicate same digest идемпотентен; changed digest conflict.
Неизвестные attempts/reserves остаются у source owner до trusted reconciliation. Ни новый Hub, ни rollback не освобождают их молча.
Old proof хранится для истории; target credential generation/adapter/draft проверяются заново. Fleet grants не заменяются автоматически client key.

## Credentials и конфигурация runtime

Основной путь — повторная authorisation в Hub. Sealed migration требует отдельного owner scope/escrow/restore receipt; ключи не входят в ordinary export или screenshots.
AI_RUNTIME_WORKSPACE/LISTEN/STATE_DIR/KEY_FILE/CLIENTS_FILE/EXECUTION_TRUST_FILE/CODEX_BINARY/CODEX_VERSION/CODEX_HOME/CODEX_WORKDIR/EXTERNAL_CALLS принадлежат operator/deployment boundary.
Их Hub counterparts описываются в ENV как own installation/listen/vault/native config и allowlist. Значения путей, токены и accepted runtime файлы не копируются в документы.

## Последовательность переключения — future S7

1. Source consumer inventory и verified nonempty backup/vault/key receipts; зафиксировать old readback endpoints.
2. Hub own S1–S6 gates, target adapters и consumer compatibility; migration manifest с source digest, mappings и financial facts.
3. Reauthorise own accounts и получить target proof; staged import не активирует профиль.
4. Сверить settings/context, immutable revision history, дедупликацию, ledger per currency, unresolved reserves и scoped grants.
5. Подтвердить quiescence/reconciliation source writes и включить один opt-in consumer на exact Hub endpoint/profile/key.
6. После accepted receipt закрыть прежние AI writes в Admin; /ai направляет в Hub с validated Namespace URL pair, каталог хранит status и UI link.
7. Проверить bookmarks/SSO/deep links, no duplicate dispatch, financial readback; расширять consumers только по receipts.

## Stop и rollback

Unmapped source record, no backup, stale/unknown proof, unresolved billed operation, digest conflict или финансовое расхождение блокируют cutover.
Rollback возвращает consumer на сохранённый compatible old endpoint только после cancel/reconcile Hub attempts. Hub ledger/history сохраняется.
Admin users/PAT/Namespace/branding/service registry/audit остаются у прежних owners. После переключения нет второго редактора AI-конфигурации.
TC-036 проверяет varied IDs/names, exact units, all source rows mapped, digest replay/conflict и held reserve. Live acceptance остаётся not_run.
