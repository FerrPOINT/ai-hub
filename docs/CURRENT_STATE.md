# Текущее состояние

Дата: 2026-10-11. Stage: IMPLEMENTING, S1/S2a/S2b в работе.

Полная реализация S1–S7 поручена пользователем. В PDLC3 создана отдельная ветка
`feat/ai-hub-v1-implementation-20261010` на baseline `b266e15`. Добавляются actual
Cargo/pnpm manifests, own DB-01, central-auth/vault ports, control reads, Rust OpenAPI
и Base React shell. Фактические результаты и незакрытые gates:
[IMPLEMENTATION_STATUS](IMPLEMENTATION_STATUS.md). Ни один полный FR/TC пока не
объявлен принятым. Установленный PDLC runtime и Admin `/ai` сохранены.

## Проверенный исходный документационный baseline

- READY-01–05 закрыты в [READINESS_AUDIT](READINESS_AUDIT.md); R01–R15 предыдущего ревью сохранены в [реестре](REVIEW_RESOLUTION.md).
- OpenAPI 0.5.0-design, 47 таблиц/420 полей; 38 requirements/40 planned actual-app tests; 16 operational routes и /login.
- S2a вводит financial primitives до S2b proof, S3 переиспользует engine. Future-stage FK запрещён gate.
- Unconfigured pricing source не создаёт fake row/ID. Financial basis/confidence отдельны; late receipt подтверждает расход с прежним tariff.
- Scheduled cancellation авторизована/idempotent/CAS и append-only; current status/time, original readback, occupied time и history согласованы.
- StatisticsFilters одна на query/echo/cursor/JSON/CSV; dimension/timezone URL и exact model buckets проверены. Service protocol строго выбирает invocation shape, unbound service key не wildcard.
- Final IAB: 230 geometry / 179 states / 146 flow assertions, 77 native PNG; 7 просмотрены непосредственно, 15 contrast checks, 0 JS errors/provider calls.
- Documentation/alignment/readiness/semantic/design, independent OpenAPI/schema и mutation/regression tests приняты по финальному пакету.
- SDK baseline/Namespace cohort/operator tooling независимы; Base/Admin docs branches не main/runtime integration.
- Backend/API/SQL migrations/SSO/live adapters/consumer/restore/cutover не реализованы, execution acceptance not_run.

Финансовая SQL-граница S2a проверена отдельно: bounded admission, one-send claim,
unknown hold/restart, atomic receipts/corrections и protected encrypted result buffer.
Добавлены owned cancel и expired unclaimed recovery ports; maintenance worker подключён к serve.
Pricing-source control/CAS/resolver и mandatory financial source guard проверены
на isolated SQL fixtures; forms и served runtime/provider evidence ещё pending.
Начат S2b connection control/operator allowlist и write-only credential lifecycle;
exact decoder, cached catalog и durable metadata reader добавлены;
live account/currency/managed auth/qualification ещё pending.
Начаты Base provider list/detail, write-only key/revoke и catalog UI; operation
lookup/no-send close поддерживают recovery после reload без хранения secret.
Saved model context/CAS/history реализованы; financial snapshot и claim проверяют
context revision, legacy unqualified proof не допускается к новому dispatch.
Connection disable/API/UI подтверждение/readback реализованы; credentials и
generation сохраняются, свежий admission/queued claim заблокирован.
Control prices/budgets API
и basic budget UI реализованы; price forms и live provider path ещё впереди.
[Partial evidence](implementation-evidence.json)
не закрывает целиком FR/TC. Текущая веха: S1 + S2a, затем S2b → S3…S7.
Действующий Admin /ai, данные, pins и установленный runtime сохраняются до accepted cutover.

## Повторное ревью 2026-10-10

[Отчёт и evidence](reviews/2026-10-10/README.md) проверяют тот же prototype/source
`28098a5`: 204 сочетания route/viewport/theme и 146 flow assertions повторены в IAB,
0 JS errors/provider calls. Документационные/schema gates, 39 regression tests,
13 signed vectors и 15 Base mirror/onboarding tests — PASS. Новых блокирующих
замечаний к документации и дизайну нет. Ранее собранные 179 states не объявлены
повторно выполненными этим ревью. Реализация и main/runtime integration остаются
отдельными gates.

В S2b добавлена собственная граница virtual model drafts: typed config,
immutable draft history, ordered targets, CAS/idempotency и Base формы
/models и /models/:id. Публикация и live proof остаются незакрытыми; actual
результаты среза перечислены в IMPLEMENTATION_STATUS/implementation-evidence.
