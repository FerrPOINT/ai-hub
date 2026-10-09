# Готовность документации и дизайна перед реализацией

Дата: 2026-10-09. Scope: product/API/data/UI handoff; работающий backend/runtime
не создаётся этой поставкой. Исходное readiness review — `45508cb`.

## Закрытие READY-01–05

| Замечание                                     | Решение                                                                                                                                                    | Авторитетное доказательство                                                                                   |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| READY-01 — S2 зависел от financial S3         | S2a вводит request/attempt/usage/price/ledger/budget/replay до S2b proof; S3 переиспользует engine                                                         | DD phases/FK, check_readiness, negative later-stage price/budget tests, IMPLEMENTATION_PACKETS/MIGRATION_PLAN |
| READY-02 — fake source для отсутствующей цены | Source state/config FK отделены от basis_source/confidence. Initial unavailable → pending/null; trusted late receipt может подтвердить basis без price row | ProjectCharge conditional schema и positive/negative/late receipt fixtures; own SQL CHECK/FK future S3        |
| READY-03 — нельзя отменить расписание         | Idempotent authorised CAS cancellation + immutable witness; original time/revision retained, predecessor interval не закрывается                           | cancelTariffActivation, cancellation DD/form, IAB state/expiry/CAS/replay/namespace/occupied-time flows       |
| READY-04 — echo терял фильтры                 | StatisticsFilters общий для query/echo/cursor/export, x-filter-field refs; UI model → virtual_model, timezone URL/reload; CSV сохраняет full echo          | check_readiness, independent schema fixtures и JSON/CSV IAB downloads с одинаковыми model buckets             |
| READY-05 — protocol/body противоречили        | ServiceInferenceInput — protocol-discriminated union                                                                                                       | Positive chat/responses и оба mismatched negative payloads; signature/body/namespace/fence vectors            |

Наличие pricing configuration не является подтверждением расхода. Сначала
неизвестный basis, позднее trusted receipt correction с прежним tariff/scope;
current price не применяют к старому request. Нет qualified currency — отказ до I/O.
Unknown-cost grant не обходит active hard monetary budget. Public SDK unbound path
отделён от SDLC service path, где все три Namespace identity обязательны и равны.

## Дальнейшие решения, замкнутые в этом пакете

- Stable policy CAS растёт на draft/activation/cancellation; response policy_version
  отличается от immutable revision sequence. Original replay сохраняет свою as_of/version.
- PriceRevision — immutable quote allowed window; effective source timeline nonoverlapping,
  без редактирования старой quote. Current billing tier/currency/window проверяются.
- Fixed Tracker fixture identity переживает reload; runtime использует проверенный owner ID.
- Scorer-v1/manual-v1 имеют точные правила в [EVALUATIONS](EVALUATIONS.md); malformed
  assertions/remote refs отвергаются, literal `$ref` в данных не принимается за resolver.
- Provider numeric38/18 и project numeric50/24 используют подходящие bounded exact codecs;
  score .001 проверяется по JSON decimal lexeme. No f64 money math.

## Что доказывают gates

check_docs/check_alignment проверяют структуру, API/route/form/requirement mapping и
20 source handoff rows. check_readiness отвергает future-stage FK, несвязанные filters,
missing cancellation authority/CAS/durability и permissive protocol. check_design
требует fresh source-bound IAB geometry/states/flows/screenshots. Независимый OpenAPI
validator проверяет schemas и payload counterexamples; mutation tests доказывают,
что verifiers отвергают поломку, а не просто подтверждают исходный файл.
Финальные результаты/числа: [QA](design/QA.md), [evidence](design/evidence.json).

## Следующий implementation frontier

S1: manifests/locks, isolated schema/initializer, central auth/service declaration,
generated DTO transport и Base UI shell. Затем S2a → S2b → S3…S7 по [плану](IMPLEMENTATION_PLAN.md).
Разработчик не выбирает заново billing/proof/Namespace/scorer/cancellation semantics.
Routine library/schema implementation остаётся в packet gate и не считается выполненным кодом.

Base SDK baseline, Namespace target cohort и operator tooling остаются независимыми.
Namespace SDK/owner readers нужно квалифицировать при S1 integration. Документация
Base/Admin опубликована в named docs branches; main integration не выполнялась.
Target approved и этот gate не означают runtime registration или accepted cutover.

## Отдельные execution gates

Actual PostgreSQL constraints/concurrency/recovery, SSO/logout/PAT, provider accounts,
native retries/tools isolation, actual generated clients, signed consumer, restore/load
и release/cutover acceptance — not_run. S7 требует nonempty backup/mapping, финансовую
сверку, consumer qualification и rollback. Действующий Admin /ai сохраняется до cutover;
после него один AI config editor. Эти будущие gates не подменены prototype/crypto PASS.
