# Архитектура frontend

Целевой React/TypeScript продукт на @sdlc/ui, без собственной копии shell.
App owns routing/session bootstrap; pages/features own scenarios; shared/api —
generated schema + thin operations + error mapping. Server state — React Query,
UI draft — local form state; глобальный store вводится только для реального shared
UI состояния. Финансовая арифметика/authorization не принадлежат frontend.

## API слой

После S1 DTO/OpenAPI генерируется из Rust handlers. Draft schema заменяется
generated spec с parity gate; React types не hand-edited.
One lifecycle transport из Base, product error envelope и typed query keys.
Не смешивать secure inference transport с automatic retry/refresh обычного GET:
mutation/idempotency readback явно; модельный запрос не повторяется browser retries.

Ключи queries включают installation/project/filter/range/currency/as_of basis.
Mutation invalidates affected detail/list/summary, не всю платформу.
Partial provider failure сохраняет draft/last data и explicit freshness.
401 заканчивает active session только по accepted central flow; 403/429 не logout.

## Forms и суммы

Fields/bounds и server contracts согласованы; CAS conflict показывает исходный
draft и новый version, не перезаписывает пользователя.
Money — decimal strings; числовые вычисления на сервере. UI форматирует amount
с currency/confidence; tooltip содержит price/usage basis и timestamp.
Фильтры/таб/tab/page в URL allowlist; secrets/prompt/private draft в URL или
localStorage не попадают. Credential field очищается после confirmed operation.

## Verification

Typed DTO/schema drift, React Query stale-data/invalidations, focus/pending/dirty
draft и keyboard, supported themes и реальные SSO/logout flows.
UI fixture не proof capability/billing. Компоненты sidebar/header не дублируются;
геометрия из Base UI Shell, route-specific exception отсутствует.

## Канонический UI контракт

[UI/UX Standard](https://github.com/FerrPOINT/services-base/blob/81decf7d9edd2c4218d8625a96e2e25c0617e9f1/docs/platform/UI_UX_STANDARD.md) и [UI Shell](https://github.com/FerrPOINT/services-base/blob/81decf7d9edd2c4218d8625a96e2e25c0617e9f1/docs/platform/UI_SHELL_STANDARD.md) — нормативные источники. NamespacePicker controlled, URL state tab-local. Общие provider/profile страницы отмечены «Установка». Notification polling: 30s только видимая active authenticated page, один in-flight request, остановка при logout/hidden; ошибка сохраняет last-success. Это target app behavior, не новый worker прототипа.

StatisticsFilters используется generated client/query keys/echo/export без второй копии dimensions. UI labels model → wire virtual_model; timezone только display. Tariff cancellation использует current policy CAS и отдельный confirmation/reason; pending/412 сохраняет ввод, retry сохраняет original body/key. Current timeline status и draft publication отображаются отдельно.
