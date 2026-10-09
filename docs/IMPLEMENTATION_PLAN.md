# План реализации

Все пути ниже — запланированные компоненты, ещё не созданный код.
Требования/тесты traceability planned; завершение этапа требует observable boundary,
не одного слоя adapters/config.

## Вертикальные этапы

### S1 — Own foundation и trusted access

Зависимости: нет; ready frontier после documentation gate.
Компоненты: `backend/crates/{api,application,domain,infrastructure}`, `frontend/src/{app,shared/api}`, `backend/migrations`.

Результат: Изолированное приложение принимает trusted principal, имеет own schema/audit и живые readiness/SSO проверки.
Scope: FR-018, FR-019, FR-023, NFR-004.
Проверки: TC-018, TC-019, TC-023, TC-030.
Создать actual manifests/lockfiles/SQLx migrations и own isolation. Документировать реальные команды; unsafe .env/credential defaults отсутствуют.

### S2 — Connection и virtual model lifecycle

Зависимости: S1.
Компоненты: `backend/crates/application/{connections,profiles}`, `frontend/src/pages/{providers,models}`.

Результат: Ранние financial primitives учитывают bounded synthetic probes; затем connection/draft публикует immutable revision через UI/API.
Scope: FR-001, FR-002, FR-003, FR-004, FR-020, FR-024.
Проверки: TC-001, TC-002, TC-003, TC-004, TC-020, TC-024.
Каждая slice включает contract/schema/UI state changes и targeted checks; старые acceptance receipts не переносятся.

### S3 — Inference с financial ledger

Зависимости: S2.
Компоненты: `backend/crates/application/{admission,routing,accounting}`, `backend/crates/infrastructure/adapters`, `frontend/src/pages/requests`.

Результат: Реальный controlled upstream отвечает full/stream/tools через scoped Hub key, retries/fallback/budget/restart доказаны.
Scope: FR-005, FR-006, FR-007, FR-009, FR-010, FR-012, FR-026, NFR-001, NFR-002.
Проверки: TC-005, TC-006, TC-007, TC-009, TC-010, TC-012, TC-026, TC-027, TC-028.
Каждая slice включает contract/schema/UI state changes и targeted checks; старые acceptance receipts не переносятся.

### S4 — Нужные внешние providers и subscriptions

Зависимости: S3.
Компоненты: `backend/crates/infrastructure/adapters/{ollama,zai,chatgpt_managed}`, `frontend/src/pages/{providers,expenses}`.

Результат: Каждый выбранный account/adapter qualified live; own auth и subscription/quota receipts проверены.
Scope: FR-008, FR-011.
Проверки: TC-008, TC-011.
Каждая slice включает contract/schema/UI state changes и targeted checks; старые acceptance receipts не переносятся.

### S5 — Статистика, расходы и operational UI

Зависимости: S3, S4.
Компоненты: `backend/crates/application/{analytics,exports}`, `frontend/src/pages/{dashboard,statistics,expenses,budgets}`.

Результат: Сводки/разрезы/export совпадают с ledger; late facts, partial failures, multi-currency и thresholds проверены.
Scope: FR-013, FR-014, FR-015, FR-016, FR-025, NFR-005, NFR-006.
Проверки: TC-013, TC-014, TC-015, TC-016, TC-025, TC-031, TC-032.
Каждая slice включает contract/schema/UI state changes и targeted checks; старые acceptance receipts не переносятся.

### S6 — Воспроизводимые model evaluations

Зависимости: S4, S5.
Компоненты: `backend/crates/application/evaluations`, `frontend/src/pages/evaluations`.

Результат: Одинаковый dataset на pinned profiles даёт проверяемые результаты, реальные расходы и cancel/recovery.
Scope: FR-017.
Проверки: TC-017.
Каждая slice включает contract/schema/UI state changes и targeted checks; старые acceptance receipts не переносятся.

### S7 — Integration, restore и полный release gate

Зависимости: S6.
Компоненты: `deploy`, `docs/contracts`, `backend/crates/application/recovery`.

Результат: Выбранный consumer проходит exact-target полный путь; own restore, rollout/rollback и итоговая приёмка доказаны.
Scope: FR-021, FR-022, NFR-003, NFR-007, NFR-008.
Проверки: TC-021, TC-022, TC-029, TC-033, TC-034.
Каждая slice включает contract/schema/UI state changes и targeted checks; старые acceptance receipts не переносятся.

## Cadence

Мелкие implementation slices — scoped tests/typecheck по changed boundary.
Не запускать full build/hosted CI/deploy после каждого файла/пакета.
Integrated S3 gate проверяет inference/ledger/recovery, integrated S5 — full
operational statistics UI, S7 — единый полный release gate с container/DB/provider/
consumer/browser/restore. Earlier full gate только если safe progress impossible.
S4 qualifying accounts выполняется только с budget и explicit own authorization.
Ни один этап не заканчивается «adapter-only» или NOT_WIRED вместо своего результата.

## Migration, зависимости и handoff

Central scopes/service declaration — S1 owner packet; qualification/vault native
transport — S4. Existing Admin consumer migration — S7 и отдельный owner approval
для concrete cutover. Новые schema changes per stage expand/compatibility path.
Code prep ready не требует действующей подписки/production access для S1–S3;
S4/S7 actual access/account receipts являются обязательными future gates.
Evidence: exact source/Base/config/schema/image, tests/DB, served identity/URL,
provider and browser outcomes, limits/known residual risks.

## S2a → S2b: обязательный порядок

S2a создаёт price/source, budget/period/reservation и request/attempt/usage/ledger/replay
records до первого probe. Минимальные control API/form для бюджета и цены работают
в S2; нет unlimited seed и bypass финансового учёта. Один transaction/admission/settlement
engine применяется к verification и затем обычному inference. S2b реализует qualified
connection → saved draft → proof → publication поверх этих primitives.
S3 добавляет public full/stream/tools/fallback и project charge/tariff/service records;
не переносит уже созданные tables и не создаёт второй financial engine. Прежние
proof calls сохраняют provider ledger; начисления проектам вводятся явно в S3 для
новых запросов, история не дополняется выдуманными начислениями задним числом.
