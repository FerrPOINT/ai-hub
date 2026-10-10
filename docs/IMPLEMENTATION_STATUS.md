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
| S2a | В реализации: financial SQL, control prices/budgets и basic budget UI | Price forms, pricing-source bindings, payload replay и lifecycle integration |
| S2b | Не реализован | Connection/catalog/draft/CAS/proof/publication UI/API |
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
effective expense pointer. Application financial port реализован для internal
verification; provider transport ещё не подключён. Price forms,
pricing-source timeline, protected response buffer, cancel-before-dispatch и
recovery unclaimed intent ещё впереди. Public/evaluation admission выключен до
S3/S6. FK profile revision присоединяется S2b до public inference.

Полные TC остаются `not_run`, когда исполнена только часть oracle. Partial mapping
и источник каждого результата — [implementation-evidence.json](implementation-evidence.json).
