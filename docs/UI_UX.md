# Карта интерфейса AI Hub

Карта сценариев — проектный baseline, не скриншоты работающего UI.
Общий shell и theme правила принадлежат Base; Hub использует wide,
reading (760px), detail-with-aside и общие PageHeader/ListPage/FormPage.
Все authenticated routes видимы по capabilities из /auth/me; backend проверяет
те же права отдельно. Публичный login — нейтральный «Вход в платформу» /
«Войти через SSO»; callback технически невидим и возвращает на target.

## Экраны

| Route            | Layout / источник                            | Сценарий и actions                                                                                            |
| ---------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| /                | wide; statistics summary                     | Range/project/currency filters, known/unknown money, health/latency/alerts                                    |
| /providers       | wide; providers                              | Каталог connections, состояние/quota/freshness, создать connection                                            |
| /providers/:id   | detail-with-aside; connection/catalog/proofs | Write-only credentials или managed login, verify, model/capability view, disable                              |
| /models          | wide; virtual models                         | Slug/name/status/revision, create, archive                                                                    |
| /models/:id      | reading; draft/revisions                     | Deployments order, params/bounds/mode, CAS save, verify exact draft, publish/rollback, disable/enable/archive |
| /requests        | wide; authorized request list                | Filter period/client/project/model/status, attempts/expenses confidence                                       |
| /requests/:id    | detail-with-aside; trace/ledger              | Revision/actual model/attempt timeline, cost basis, cancellation/readback                                     |
| /statistics      | wide; statistics endpoints                   | Tokens/counters/percentiles, dimensions, sample count/lag, bounded export                                     |
| /expenses        | wide; expense/prices/subscriptions           | Per-currency amounts, estimates/unknown, billing sources/period, corrections                                  |
| /budgets         | wide; policies/periods/alerts                | Scope/currency/period limits, spent/reserved, warning/hard-limit policy                                       |
| /clients         | wide; client/grants                          | Create scoped access, show-once key, expiry/rotation/revoke, own usage                                        |
| /evaluations     | wide; datasets/runs                          | Dataset version, candidate revisions, bounded launch, results/export                                          |
| /evaluations/:id | detail-with-aside; run/cases                 | Compare quality/time/cost, incomparable states, cancel, evidence provenance                                   |
| /audit           | wide; audit events                           | Object/actor/time filters, immutable safe changes                                                             |
| /settings        | reading; effective config                    | Read-only deployment limits/retention/auth/integration identity                                               |

Enable модели проверяет current qualification; archive доступен после disable.
Управление client state/scopes/cost-policy находится в /clients. Result recovery
в request detail только по own read_result grant; financial trace не даёт content.

## Общие состояния

Каждый route: loading, confirmed empty, error, permission denied, not found,
partial failure и stale data. Loading не показывает fake zero expenses.
При saved draft+failed provider read форму не очищать. Pending submit защищает handler
и кнопку. 412 сохраняет draft и предлагает explicit reconcile.
Hard budget deny объясняет scope/currency, unknown reserve и ближайший reset.
Actual model unknown отображается текстом, не незаметным icon.

## Взаимодействие

Cards отвечают на «кто использует», «сколько стоит», «почему ошибка», «что проверить».
Статусы confidence не только цвет. Dates/currency/amount доступны screen reader.
Filters/range/timezone direct-load/Back/Forward сохраняются в URL; refresh сохраняет
last data с freshness error banner. Таблицы имеют local horizontal scroll.
Dialogs focus trap/Escape/return-focus, mobile actions доступны.
375/1440/1920/2560 и все Base темы: no body overflow, long slugs/unknown labels,
zero/large/negative correction amounts и partial state.
Synthetic fixtures не содержат реальных keys/prompts/account names.

## Приёмка

[ui-routes.json](ui-routes.json) — route/operation/test mapping, status planned.
TC-024 проверяет сценарии, TC-032 — geometry/a11y. Реальные screenshots и served
source identity собираются после S2/S5, не выдумываются для design baseline.
