# Основные сценарии

## UC-01 — Подключить нейронку

Actor: active central browser user с configuration access.
Providers → create connection с approved endpoint reference → собственный API key/
managed authorization → catalog → bounded account/model verification.
Credential response metadata only, generation advances; catalog не proof.
Если credential operation unknown: readOperation, не повтор с новым key.
Квота/physical limits неизвестны → explicit unknown. No provider fallback at login.

## UC-02 — Сохранить и опубликовать профиль

Models → create draft с known connection/model IDs, bounds/params/mode →
save CAS → verify exact config с existing budget_policy → publish exact proof.
Draft save не требует proof. Input+output≤qualified context.
Change params/order/generation инвалидирует publication proof. Stale ETag сохраняет
draft; read current version и explicit reconcile. Для pinned один deployment/
max_attempts1; provider rejections не подбирают другие модели.

## UC-03 — Выдать доступ приложению

Clients → create/update trusted project binding, allowed profile IDs, scopes,
expiry/RPM/concurrency/cost policy → подтвердить saved configuration →
отдельно issueClientKey → show-once secret. Provider key не передаётся.
Key lost/unknown receipt → read operation; explicit rotation, не новый issue retry.
Revoke/disable: новые admissions denied, history/charges retained.

## UC-04 — Обычный и stream вызов

Consumer получает каталог только allowed published slugs → request с optional
caller UUID idempotency key → exact frozen revision/actor/project/budget →
intent/reserve → provider → full/SSE/tools/JSON. Caller сам выполняет function tools.
Fallback только safe qualified nonacceptance до output; каждый attempt/fee recorded.
Client увидит request ID/revision до stream. Implicit SDK retries запрещены.

## UC-05 — Unknown outcome и recovery

После timeout/обрыва read request/status; unknown не resend/zero expense.
Повтор с тем же key возвращает old ID/status. Stream не переигрывается автоматически.
Если terminal reply доступен: own read_result grant и ≤24h buffer → retrieve result.
Provider receipt reconcile уточняет state/cost; cancellation не proof free billing.

## UC-06 — Понять расходы

Range/currency/project → confirmed/estimated/unknown/reserved отдельно →
request attempts и exact rate/receipt → trusted late settlement заменяет associated
estimate → correction history сохраняется.
Подписка учитывается один раз в cash occurred_at, service period отдельно;
allocation не второй платёж. Monthly budget включает charges всего месяца,
даже если текущий dashboard показывает последние7 дней.

## UC-07 — Управлять лимитом

Budget scope/currency/UTC day/month, hard limit и80/95 thresholds → saved CAS.
Admission удовлетворяет всем scopes атомарно. Unknown reserve удерживается.
Hard deny происходит до I/O; user узнаёт scope/reset/held.
Выдача cost_unknown_allowed требует explicit caps и явного отсутствия денежной гарантии.

## UC-08 — Сравнить модели

Dataset immutable version (synthetic/authorized cases/tools/assertions) →
frozen pinned profile revisions → own budget_policy/scorer/repetitions/concurrency →
run. Per-candidate/case/repetition unique key; no retry/cache/fallback.
Compare pass rates, samples, latency, known expense и incomparable/unknown.
Cancel stops new cases; ambiguous current I/O retained. New run не mutation old run.

## UC-09 — Статистика и экспорт

Authorized query/snapshot с explicit denominator/time basis. Partial/stale сохраняет
known facts; missing source не empty-success. Export тот же scope/as_of, decimal/
null/currency + CSV formula safety. UI settings не приписывают чужого пользователя
по caller user label. Reconstruction не запускает provider.

## UC-10 — Privacy, restore и cutover

Own replay/artifact TTL не стирает financial facts/dedupe. Archived receipts имеют
source/retention digest. Own key escrow и nonempty isolated restore external_calls=false.
Admin/Octo transition требует отдельного owner plan; old data не автоматически
переносится из-за появления UI Hub. Documentation/design baseline не deployment.
