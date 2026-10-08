# Спецификация экранов

[design-contract](design/design-contract.json) — canonical product page composition.
Общий Base shell нормативен; существующие UI/UX документы не копируются.
Технические opaque IDs для API не подменяются display names; demo IDs prototype
не являются production route contract.

## Обзор — /

Цель: Что использует модели, сколько стоит и что требует внимания.
Layout: wide. Источники/операции: exportStatistics.
Порядок содержимого: Запросы → Подтверждённые расходы → Оценочные расходы → Unknown outcomes → Тренд → Качество → Последние запросы.
URL state/filters: Период; Проект; Валюта.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Провайдеры — /providers

Цель: Состояние подключения отдельно от доступа модели.
Layout: wide. Источники/операции: createConnection, readConnection.
Порядок содержимого: Название → Adapter kind → Authorization → Quota freshness → Каталог → Capabilities.
URL state/filters: Поиск/состояние подключения.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Подключение — /providers/:id

Цель: Own authorization и квалификация exact account/model/endpoint.
Layout: detail-with-aside. Источники/операции: updateConnection, writeCredential, startManagedLogin, revokeConnectionAuthorization, startVerification, disableConnection.
Порядок содержимого: Connection generation → Write-only credential → Managed login → Model metadata → Qualified capabilities → Proof history → Safe configuration.
URL state/filters: Каталог / Авторизация / Проверки.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Модели — /models

Цель: Имена API и версии профилей без скрытой подмены.
Layout: wide. Источники/операции: createVirtualModel, readVirtualModel.
Порядок содержимого: Slug → Display name → Mode → Active revision → Traffic → Expense confidence.
URL state/filters: Поиск; Mode; Active/draft.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Профиль — /models/:id

Цель: Save draft → verify exact draft → publish; ongoing requests frozen.
Layout: reading. Источники/операции: updateVirtualModelDraft, verifyVirtualModelDraft, publishRevision, listRevisions, rollbackRevision, disableVirtualModel, enableVirtualModel, archiveVirtualModel.
Порядок содержимого: Identity → Ordered deployments → Generation parameters → Bounds → Required capabilities → Allowed overrides → Deadline/max attempts → Proof → Publication.
URL state/filters: Маршрут / Параметры / История.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Запросы — /requests

Цель: Logical request и attempts не одно число.
Layout: wide. Источники/операции: listRequests, readRequest.
Порядок содержимого: ID/time → Virtual profile → Client → Actual model → Outcome → Cost knownness → Attempt count.
URL state/filters: Период; Project/client; Profile/revision; Status; Currency; Purpose.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Запрос — /requests/:id

Цель: Нельзя отправлять unknown request заново ради восстановления.
Layout: detail-with-aside. Источники/операции: readRequest, readRequestResult, cancelRequest.
Порядок содержимого: Frozen binding → Timeline → Actual model confidence → Usage category provenance → Confirmed/estimated/unknown → Held reserve → Receipt → Read result grant.
URL state/filters: Ход / Attempts / Usage / Учёт.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Статистика — /statistics

Цель: Сопоставимые знаменатели и явная полнота данных.
Layout: wide. Источники/операции: readStatisticsSummary, readStatisticsTimeseries, readStatisticsBreakdown, exportStatistics.
Порядок содержимого: Known tokens → Rates/denominators → Latency/TTFT samples → Fallback → Availability probes → Quota freshness → Projection watermark → Breakdown.
URL state/filters: From/to; Timezone; Project/client/user; Profile/revision; Provider/actual model; Purpose; Dimension.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Расходы — /expenses

Цель: Не складывать валюты, estimate с заменившим receipt или allocation с cash fee.
Layout: wide. Источники/операции: readStatisticsExpenses, createPriceRevision, listPrices, recordSubscription, listSubscriptions, requestLedgerCorrection, exportStatistics.
Порядок содержимого: Confirmed → Estimated → Unknown count → Reserves → Currency groups → Receipts/corrections → Rate snapshots → Subscription period.
URL state/filters: Cash range; Currency; Project/client; Provider; Confidence; Ledger/Prices/Subscriptions.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Бюджеты — /budgets

Цель: Лимит проверяется до I/O; unknown reserve не свободный остаток.
Layout: wide. Источники/операции: createBudget, updateBudget, listBudgets.
Порядок содержимого: Policy → Used → Held → Available → 80/95 thresholds → Hard cap → UTC reset → Cost-unknown disclaimer.
URL state/filters: Scope; Currency; Period.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Приложения — /clients

Цель: Сохранение доступа не выдаёт новый ключ автоматически.
Layout: wide. Источники/операции: createClient, updateClient, issueClientKey, revokeClientKey, disableClient, enableClient.
Порядок содержимого: Application identity → Project binding → Allowed profiles → Scopes → Expiry → RPM/concurrency → Cost policy → Keys.
URL state/filters: Project; Status.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Тестирование — /evaluations

Цель: Одинаковые datasets и frozen configurations.
Layout: wide. Источники/операции: createDatasetVersion, listDatasetVersions, startEvaluation, listEvaluations.
Порядок содержимого: Datasets → Version/hash → Runs → Pinned config → Quality/time/cost → Unknown/incomparable.
URL state/filters: Dataset/version; Profile revisions; Outcome.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Результат теста — /evaluations/:id

Цель: Partial/unknown не successful comparison и не zero-cost победа.
Layout: detail-with-aside. Источники/операции: readEvaluation, cancelEvaluation.
Порядок содержимого: Frozen dataset/scorer → Exact profiles/parameters → Samples → Latency/token/cost → Incomparable → Artifacts access → Cancel/readback.
URL state/filters: Кандидат; Case; Outcome.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Аудит — /audit

Цель: Immutable safe audit, без тела credentials.
Layout: wide. Источники/операции: listAudit.
Порядок содержимого: Actor/action → Object/revision → Operation/outcome → Reason → UTC timestamp.
URL state/filters: Object; Actor; Action; Time.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Настройки — /settings

Цель: Параметры read-only; ничего похожего на env dump.
Layout: reading. Источники/операции: readEffectiveSettings.
Порядок содержимого: Own installation → Effective bounds → Auth/integration → Retention → Source/Base/config/schema identity.
URL state/filters: Раздел конфигурации.

Каждая action имеет source payload/permission/receipt из API и Field reference.
Успех mutation подтверждается operation/readback; error/unknown не становится success.
Loading/empty/error/403/404/partial/stale обязательны; для editable form также
dirty, validation,412/pending. Detail lookup missing ID даёт404, не first-record fallback.

## Состояния

| State            | Экран                           | Контракт                                                       |
| ---------------- | ------------------------------- | -------------------------------------------------------------- |
| ready            | Данные загружены                | Содержимое с as_of, confidence и count                         |
| loading          | Загрузка                        | Не отображать успешные нулевые деньги/пустой каталог           |
| empty            | Подтверждённый пустой результат | Scope/filters сохранены; contextual action                     |
| error            | Ошибка чтения                   | Сохранить draft/last values; безопасный retry чтения           |
| forbidden        | Нет доступа                     | Не logout; без CRUD/повторного model вызова                    |
| not_found        | Нет объекта                     | Не fallback на первый объект; никаких CRUD                     |
| partial          | Часть источников недоступна     | Known/unknown раздельно; успешно загруженное сохранено         |
| stale            | Устаревший snapshot             | as_of/lag и обновление, не fake live                           |
| conflict         | CAS конфликт 412                | Черновик не теряется; read current revision/explicit reconcile |
| pending          | Операция в процессе             | No double submit; durable operation readback                   |
| budget_exhausted | Недостаточно бюджета            | Denied before external I/O; explain held reserve/scope         |
| long             | Длинные значения                | Реальные переносы, читаемые IDs, без body overflow             |

## Действия без потери состояния

Изменение filter сбрасывает cursor history, сохраняет корректный URL state.
Route navigation с dirty form даёт stay/discard; publish требует saved exact draft.
Background error сохраняет поля/last-success data. Key выдача — отдельное явное
действие, one-time result/replay semantics не «Сохранить права».
Подтверждение destructive action описывает effect и preservation history.
Signed actor, generation, proof ID, ETag и idempotency key не редактируются как user fields.
