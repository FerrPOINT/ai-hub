# Фактическая реализация AI Hub v1

Дата: 2026-10-10. Ветка: `feat/ai-hub-v1-implementation-20261010`.
Источник задачи: полный S1–S7 implementation; production cutover исключён.
Baseline Hub: `b266e158b33e6cc70e02a9df3ba1d21c74b983af`.

## Владельцы и решения

Hub владеет schema/domain/application/API/UI. Central Auth остаётся владельцем
SSO/session/PAT; Admin — Namespace registry; Tracker/Fleet/Forge — исходным
resource/execution context. Base/Admin documentation branches не вливаются целиком.
Remote refs обновлены; оба owner packet SHA совпадают с заданными baseline.

Выбран native Rust application boundary с SQLx transactions и Base auth/UI.
Generic proxy или второй policy engine отклонены: они теряют durable intent,
accounting и qualification provenance. Миграции выполняет явная команда,
а serve проверяет сохранённые schema/installation/key без initialization.

SDK `875cac2edf1a18c3a8a59e2f67256d02a8fc04e4` сохраняется в `.base-revision`.
Технические Namespace types и controlled UI берутся отдельно из cohort
`81decf7d9edd2c4218d8625a96e2e25c0617e9f1` по `.namespace-base-revision`.
Оба checkout материализованы независимо и проверяются до Cargo/consumer checks.
Operator tooling остаётся pinned workspace dependency; его acceptance не
выводится из проверки двух SDK checkout.

Grant получает `namespace_binding_id` с own composite FK. Project ID/name сами
по себе не предоставляют доступ к Namespace другой registry. Это уточнение
DB-01 не меняет владельцев или public NamespaceRef wire.

## Последовательность и открытые этапы

| Этап | Фактический статус | Следующий результат |
| --- | --- | --- |
| S1 | В реализации: Cargo/pnpm manifests, DB-01, vault/auth ports, control reads, generated API и Base shell | PostgreSQL/SSO/grants/readers/locked consumer evidence и own runtime |
| S2a | В реализации: financial SQL/source guard, control API, basic budget UI, protected results и wired maintenance worker | Price/source forms и milestone runtime/provider evidence |
| S2b | В реализации: connection/credentials, exact OpenRouter metadata decoder и generation-bound cached catalog | Live metadata reader, context, disable/archive, draft/proof/publication и UI |
| S3 | Не реализован | Scoped public/signed inference и общий financial engine |
| S4 | Не реализован | Own provider accounts с explicit budget и live receipts |
| S5 | Не реализован | Полный operational UI и authorized snapshot/export |
| S6 | Не реализован | Immutable datasets, fenced pinned comparisons |
| S7 | Не реализован | Consumer/restore/load/rollout candidate и final evidence |

Source реализация не означает закрытие соответствующих FR/TC. Существующая
design traceability сохраняет actual acceptance `not_run` до полного сценария.
Результаты scoped checks дополняются после исполнения. Ни SDK pin, ни prototype
evidence не считается доказательством живой интеграции.

## Проверки и запуск

В source workspace PDLC3 доступны `enter-dev.ps1` и exact installed Rust image.
`scripts/rust.ps1` проверяет обе зависимости и запускает только указанную Cargo
команду с собственным cache/target, ограниченными CPU/RAM и `--rm`.
`scripts/test_foundation_pg.ps1` создаёт временный PostgreSQL17 с own role/DB,
без опубликованных портов и named volumes, затем удаляет только свой контейнер.

Реальный постоянный runtime добавляется через штатного owner/operator после
проверки image/schema/SSO и ресурсов. Ни existing PDLC groups, ни common bundle,
ни текущий Admin `/ai` не переключаются подготовкой source packet.

Provider I/O выполнено: **0**. OpenRouter dev account разрешён пользователем;
проверки ограничены пятью короткими вызовами и общим budget 0.10 USD.
Ключ на Dev подтверждён без вывода/переноса. Account/currency/model qualification,
financial admission и остальные S4/S7 gates ещё не закрыты.

## Scoped evidence

- `cargo check -p aihub-api`: pass для foundation source.
- `cargo test -p aihub-domain --lib`: 6 pass, включая four financial rules.
- Own PostgreSQL17: 1 integration scenario pass, 0 ignored при explicit harness.
  Проверены rollback partial initialize, concurrent initialize, wrong installation/key,
  immutable audit, own composite FKs, nonnil identities, exact Namespace grants и
  cursor snapshot/actor/query/revocation boundaries. Temporary DB удалена runner.
- Frontend Namespace/transport: 5 pass; URL pair, late previous-session 401,
  current revocation, 403 preservation и отсутствие bearer в profile storage.
- Documentation: structural/readiness/semantics pass; Python regression 39 pass.
- Rust OpenAPI export и generated TypeScript consumer/typecheck pass.
- API auth/Namespace boundary tests: 2 pass (controlled ports, не live SSO).
- Infrastructure unit checks: 3 pass, vault AAD/wrong key, trusted origins и
  Namespace identity/confirmed/archived transitions. Reader ещё не подключён к
  живому owner; его observation не выдаёт grant и не заменяет Tracker readback.
- Fresh PostgreSQL check после `0003_financial_prerequisites`: все три миграции
  применились, foundation scenario pass.
- Fresh PostgreSQL17 после `0004`–`0006`: один financial scenario pass, 0 ignored.
  Два независимых PgStore конкурируют за последний reserve и один dispatch claim;
  foreign grant/model/generation, отсутствующая цена под hard budget, revoked grant,
  смена connection generation и forged fence отвергаются. Expired sender recovery
  сохраняет original attempt/held reserve без redispatch. Unknown fact не нулевой;
  duplicate receipt не меняет totals; confirmed заменяет estimate, refund append-only,
  actual overrun не обрезается и блокирует новое admission. Forced final audit failures
  откатывают admission и settlement целиком. Account/model/currency в fixture
  синтетические; это не live provider qualification и не полный TC-028.
- Live SSO, owner readers, served UI и CI ещё pending.

## Control prices — текущий срез

Добавлены `GET/POST /api/v1/prices`: immutable quote, обязательный UUID
Idempotency-Key, HMAC canonical payload собственным vault key, safe operation
readback и atomic audit. Write-only PAT может создать quote без config read.
Rates/currency/effective window/source типизированы; nullable cached/effective_to
явные, отсутствие request_fee сохраняется unknown. Overlapping quote windows
разрешены; effective pricing-source timeline реализуется отдельно, поэтому
создание quote не публикует inference config.

Own PostgreSQL17 control fixture: concurrent same-key create возвращает одну
quote/operation/audit, changed binding конфликтует, чужой actor не читает operation,
cursor привязан к actor/limit и не включает новые quotes; final audit failure
откатывает quote и operation. Это scoped SQL evidence, не browser/live API acceptance.
HTTP access: 3 tests pass, включая wrong PAT scope/nil key/decimal number/negative fee.
Required nullable fields и unknown fee: domain unit pass. Rust-generated OpenAPI
и generated TypeScript consumer/typecheck pass для этого actual subset.
Price forms и подключение этих quotes к S2b proof ещё впереди.

## Control budgets — текущий срез

Добавлены `GET/POST /api/v1/budgets` и `PATCH /api/v1/budgets/{budget_id}`.
Exact decimal cap и warning thresholds сохраняются вместе с operation/readback/audit.
Изменение требует strong quoted `If-Match` и UUID Idempotency-Key; currency,
period и scope immutable. Понижение cap не переписывает charged/reserved balances;
remaining может быть отрицательным после overrun или понижения лимита.

Project scope разрешается по точной registry/Namespace UUID-паре в verified binding.
Client scope получает Namespace из сохранённой client record, без caller override.
Installation/project/client policies требуют актуальный product grant; profile
policy выключен до создания owner table S2b. Свой installation UUID не является
универсальным разрешением читать проектные бюджеты.

`0007_budget_identity` исправляет own uniqueness: одинаковый Tracker UUID в двух
registry не объединяет policies; existing policy IDs/balances сохраняются.
Own PostgreSQL17 fixture: concurrent create один результат, concurrent CAS один
победитель, исходный replay после следующего update, сохранение расходов/резервов,
negative remaining, two-registry neighbor, derived client Namespace, current grant
revocation on frozen cursor, UTC day/calendar month и rollback final audit — pass.
Финансовые balances для control fixture синтетические; реальные reserve/settle
проверены отдельным financial fixture. Полный TC и live UI acceptance остаются открыты.

Rust-generated OpenAPI/TypeScript consumer/typecheck pass; HTTP access 4 tests pass.
Shared operation helper сохранил price-control regression: own PostgreSQL fixture pass.
Base `/budgets` показывает authorized pages и exact string totals, создаёт budget
для установки/выбранного проекта и редактирует существующий cap/thresholds.
Creation client/profile picker остаётся на owner APIs S2b/S3. Form имеет dirty guard,
pending lock, strong CAS и сохраняемый во вкладке non-secret mutation intent.
Unknown reply/reload/последующий 403 сохраняют исходные key/body до readback;
412 оставляет пользовательские значения и требует явной загрузки текущей версии.
Три controlled transport UI tests pass, включая decimal beyond floating precision.
Это не IAB или live SSO evidence. Browser/runtime acceptance выполняется на milestone.

`0003_financial_prerequisites` вводит connection/generation/catalog/price/probe,
requests/attempts/usage/replay, immutable ledger и budget/reservation таблицы.
`0004` добавляет exact bounded grant/account/currency authority, `0005` —
неповторяемый dispatch claim и expiry, `0006` — append-only settlement facts и
effective expense pointer; `0008` — frozen wire mode и encrypted result receipts.
Application financial port реализован для internal
verification; provider transport ещё не подключён. Price forms,
maintenance worker подключён к serve; provider execution worker ещё впереди. Public/evaluation admission выключен до
S3/S6. FK profile revision присоединяется S2b до public inference.

## Protected results — текущий срез

Добавлен internal `ResultDelivery` port: settlement и encrypted JSON result
сохраняются одной SQL transaction. AES-GCM AAD связывает installation/request,
protocol и HTTP status; keyed binding сохраняет exact bytes. Body не имеет
Debug/Serialize, хранится отдельно от audit/operations/ledger и ограничен 2 MiB.
Формат и stream flag заморожены в admission/probe; legacy wire mode не угадывается.
Protocol-specific response normalization, HTTP result endpoint и worker wiring — S3.

Отдельный current `read_result` grant требует exact principal/client/project/
Namespace binding и enabled client scope. Metadata/verification grant не даёт
content. Unknown/in-progress/stream возвращают metadata outcome без reply.
Payload TTL максимум 24h; immutable replay receipt остаётся после bounded purge,
поэтому duplicate/late settlement не продлевает TTL и не пересоздаёт content.

Fresh PostgreSQL17 financial fixture после `0008`: 1 scenario pass, 0 ignored,
8.41s. Сохранены прежние admission/concurrency/recovery/receipt/correction oracles.
Новые assertions: immutable wire mode, wrong protocol/TTL, atomic final audit
rollback без reply или expense, exact byte replay, changed body conflict, separate
scope/grant, foreign client, revocation, unknown hold, no plaintext in safe fields,
expiry/purge и запрет resurrection. Provider accounts/body синтетические;
это partial S2a/TC evidence, не live provider/result API или полный TC-028.

Полные TC остаются `not_run`, когда исполнена только часть oracle. Partial mapping
и источник каждого результата — [implementation-evidence.json](implementation-evidence.json).

## Cancellation и unclaimed intent — текущий срез

Internal owned cancellation сохраняет durable cancel intent с audit до terminal
settlement. Новые claims заблокированы сразу. Если финальный accounting/audit
недоступен, reserve остаётся held и retry продолжает исходную отмену.
До единственного verification dispatch common settlement engine подтверждает
zero expense по отдельному local no-send authority, без поддельного claim или
provider usage. Даже ненулевой frozen request fee не начисляется до I/O.
После dispatch/unknown отмена остаётся intent; она не доказывает отсутствие bill.

`0009_intent_lifecycle` фиксирует queue TTL 5–120s, ограниченный также сроком grant.
Deadline и TTL immutable; истёкший/legacy intent не получает новый send claim.
Bounded recovery переводит истёкшие unclaimed intents в unknown с held reserve,
без redispatch. Live neighbor не затрагивается. Producer authority остаётся
verified internal context; public/signed cancel endpoint и scheduler — S3.
Multi-attempt fallback cancellation/aggregation также квалифицируется в S3.

Fresh PostgreSQL17 financial fixture после `0009`: 1 scenario pass, 0 ignored,
13.46s. Проверены fixed-fee no-send cancellation, wrong client, durable intent
при final settlement audit failure, запрет claim, retry после grant revoke,
concurrent cancellation без double release, after-dispatch hold, forged no-send
rejection, cancel/claim race, immutable deadline, expiry/no redispatch и untouched
live neighbor с прежним request ID. Прежние settlement/replay assertions сохранены.
`cargo check --locked -p aihub-api --tests` pass. Actual process crash, transport
cancel, public authentication и runtime scheduler остаются `not_run`.

## Pricing sources — текущий срез

Добавлены `GET/POST /api/v1/pricing-sources`, typed required nullable fields,
Idempotency-Key/HMAC operation replay, CAS по connection/model/currency и atomic audit.
Immutable revisions имеют unique start и half-open intervals. Resolver выбирает
последнюю started revision; expired revision сохраняет ID с unknown quote, без
older fallback. Future source меняет policy version, но не текущий effective ID.
Manual binding проверяет exact quote tuple, current generation tier и allowed window;
несоответствие — 422. Уже созданная несвязанная quote не удаляется при failed binding.

Auto требует structural catalog origin и current qualified pricing reader/native
currency/account generation/adapter/endpoint. Свободный текст price.source не даёт
этих прав. Catalog observation и data status показываются отдельно от settings.
Существующий legacy billing tier не угадывается из имени или billing_mode.

Fresh PostgreSQL17 после `0010`: 1 scenario pass, 0 ignored, 0.90s. Assertions:
concurrent operation replay/CAS, exact connection/currency neighbors, expiry no
fallback, original readback после новых revisions, unique start, immutable rows,
final audit rollback и auto rejection/qualification/invalidation. Catalog/account
fixtures синтетические; ledger entries 0, provider calls 0. Five API access tests
pass (controlled authentication ports). Полный TC-038/S2a остаётся открытым до
forms и live adapter/runtime evidence.

## Financial source binding — текущий срез

`0011` добавляет own source FK и immutable policy version в request. Legacy
version не угадывается. Admission разрешает quote только по current resolver,
замораживает ID/version/tier/quote/as_of и source data status в attempt snapshot.
Unconfigured resolution имеет null source ID и не создаёт placeholder policy.
Configured, но expired source сохраняет ID и unknown quote; hard budget не
подменяет missing price нулём.

Dispatch повторно проверяет source/policy/quote/tier перед one-send claim.
Stable advisory lock покрывает и отсутствие policy row; source writer не может
вставить новую policy между проверкой и commit claim. Future revision также
меняет policy version и инвалидирует queued proof. Same-key replay возвращает
старый request; попытка заменить его source context конфликтует.
Settlement использует original price snapshot, а не current resolver.

Fresh PostgreSQL17 financial fixture после `0011`: 1 scenario pass, 0 ignored,
14.85s. Сохранены прежние money/replay/cancel/recovery assertions. Дополнительно
проверены no-placeholder unconfigured hard-budget denial, future source version
change при той же effective quote, denied old claim, original request replay,
immutable source version, fresh proof и late usage по прежним 2/8 rates после
смены на 20/80. Currency/account/proof синтетические; provider calls 0.

## Maintenance worker — текущий срез

Serve после readiness и bind запускает installation-owned periodic maintenance.
Сначала обрабатывается durable no-send cancel, затем expired intent/dispatch recovery,
encrypted payload TTL и read snapshot/cursor cleanup. Каждая pass ограничена 100
записями; сбой одного task не отменяет независимые следующие tasks. SQL/actor/body
не печатаются в worker logs; только task/counts. Worker не имеет provider transport.

`AIHUB_MAINTENANCE_TICK_SECONDS`: default 5, допустимо 1–60. Missed ticks skip,
Ctrl-C/SIGTERM останавливают worker; join ограничен 5s. При timeout task abort
не превращает held reserve в release. Source wiring не считается served acceptance.

Fresh PostgreSQL17 financial fixture: 1 scenario pass, 0 ignored, 13.78s.
Проверены retry после persistent final audit failure, worker/foreground cancel race,
scheduled expired-intent recovery, expired snapshot/cursor cascade с сохранением
live neighbor, реальный timer loop cleanup и завершение по watch signal.
Прежние source/money/replay assertions сохранены. Locked API check pass.
Docker signal, HTTP graceful drain, actual process crash/restart и installed-image
provenance остаются S3/S7 runtime gates. External provider calls 0.

## Connection metadata control — текущий срез

Добавлены connection list/read/create/update. GET возвращает ETag, PATCH требует
strong If-Match и full typed settings; mutation использует HMAC/keyed operation
readback и atomic audit. Идентичность не выводится из display name. Label-only
update сохраняет generation/authorization; endpoint или billing settings создают
новую generation с absent authorization и invalidated qualification. Disabled
state не включается настройками. Старые generation snapshots и credentials сохраняются.

Endpoint policy устанавливает operator через `configure-endpoints` и bounded
absolute `AIHUB_ENDPOINT_POLICIES_FILE`. Public input содержит только policy ref,
не URL/headers/proxy. Policy bytes/hash immutable; изменение требует новой ref.
Generation хранит endpoint snapshot. Sample OpenRouter origin проверен по
[официальному API](https://openrouter.ai/docs/api/api-reference/chat/send-chat-completion-request).
Sample — operator configuration, не native account/billing qualification.
DNS/egress pinning при настоящем HTTP выполняется в adapter slice S4.

Fresh PostgreSQL17 после `0012`: 1 scenario pass, 0 ignored, 0.54s. Проверены
endpoint ref/provider matching, same-key create, stale/concurrent CAS, two same-name
connections, rename without deauthorization, material generation change, pinned
original readback, immutable old snapshot и final audit rollback. Six HTTP access
tests pass; raw URL/secret fields rejected без secret echo. Credentials/quota
отсутствуют в metadata fixture, новый connection status authorization_unknown.
Disable/enable/archive, model catalog и live verification ещё не реализованы.

## Write-only credentials — текущий срез

Добавлены PUT/DELETE credentials с 202 Operation. PUT требует own expected_generation,
DELETE — fresh strong If-Match/version. Principal scope и key проверяются до seal.
Secret DTO — zeroizing, без Debug/Serialize; API schema writeOnly. Keyed canonical
operation binding включает secret digest, но digest/plaintext не сохраняются.
Ciphertext AES-GCM связан AAD с installation/connection/new generation/purpose.

Новая generation prepared: наличие ключа не подтверждает account/currency/capability.
Settings и operator endpoint snapshot копируются, qualified tier/authorization не
наследуются. Старые credential versions становятся revoked с сохранением ciphertext;
qualification инвалидируется. Revoke создаёт own revoked generation и сохраняет
disabled state. Managed provider отвергает API-key body и требует own login.

Fresh PostgreSQL17 после `0013`: 1 scenario pass, 0 ignored, 0.54s. Actual Foundation
crypto/store boundary проверяет concurrent same-key write, stale generation, changed
secret conflict, wrong key/generation AAD, отсутствие canary в operations/audit/
Connection/Operation JSON, final audit rollback, revoke и original replay без
восстановления authorization. Immutable ciphertext/delete constraints pass.
Seven API access tests pass, error responses не echo secret. Accounts/secret
синтетические; live Dev key не перенесён, provider I/O 0. Whole-app log/browser/export
canary, active adapter secret context и managed login остаются S3/S4/S7 gates.

## Catalog decoding и cache — текущий срез

OpenRouter decoder сверён с official [models](https://openrouter.ai/docs/api/api-reference/models/list-all-models-and-their-properties)
и [key metadata](https://openrouter.ai/docs/api/api-reference/api-keys/get-current-api-key).
Money number lexemes читаются RawValue без f64; per-token pricing strings переводятся
в per-million exact decimal. Negative/missing/sub-scale/overflow price остаётся
unknown; excessive exponent ограничен до BigDecimal allocation. Account label,
credential fragments и creator identity не сохраняются в DTO. Numeric account
observations не присваивают currency/tier и не создают paid receipt.

Catalog limits/capabilities — advertised и unverified, не physical/inference proof.
Internal store проверяет own connection generation и endpoint hash, создаёт
immutable snapshot. Removed membership не удаляет historical upstream rows.
GET models читает только stored snapshot, поддерживает q/opaque cursor/limit;
новая generation не наследует старый catalog. HTTP refresh/operation и pricing
quote materialization ещё не подключены: нет скрытого external I/O в GET.

Scoped decoder: 3 unit tests pass, включая 20 integer/18 fractional digits,
exponents, unsupported scale, unknown fee/cache, duplicate model/invalid limits.
Fresh PostgreSQL17 после `0014`: 1 scenario pass, 0 ignored, 0.54s. Search,
actor/query cursor, old page после refresh, immutable snapshot, generation/hash
mismatch и отсутствие qualification/ledger rows проверены. Eight controlled-port
HTTP access tests pass. Source cache writer ещё внутренний; actual HTTP fetch,
durable refresh operation/audit, currency witness, valid secret context и live
account qualification остаются обязательными следующими шагами до model probe.
