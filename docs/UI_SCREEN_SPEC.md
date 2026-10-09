# Спецификация экранов

Каноническая композиция: [design-contract](design/design-contract.json). Общий shell следует Base; продукт владеет содержанием. Источники API — typed draft, реализация отсутствует.

## Обзор — /

Цель: Что использует модели, сколько стоит и что требует внимания.

Класс: operational. Layout: wide.
Операции: exportStatistics, readStatisticsSummary, listRequests, listNotifications.
Содержимое: Запросы → Подтверждённые расходы → Оценочные расходы → Unknown outcomes → Тренд → Качество → Последние запросы.
Фильтры/URL: Namespace в шапке; Период; Валюта.

## Провайдеры — /providers

Цель: Состояние подключения отдельно от доступа модели.

Класс: operational. Layout: wide.
Операции: createConnection, readConnection, listProviders.
Содержимое: Название → Adapter kind → Authorization → Quota freshness → Каталог → Capabilities.
Фильтры/URL: Поиск подключения; Состояние.

## Подключение — /providers/:id

Цель: Own authorization и квалификация exact account/model/endpoint.

Класс: operational. Layout: detail-with-aside.
Операции: updateConnection, writeCredential, startManagedLogin, revokeConnectionAuthorization, startVerification, disableConnection, listModelContextPreferences, saveModelContextPreference, readOperation.
Содержимое: Connection generation → Write-only credential → Managed login → Model metadata → Qualified capabilities → Proof history → Safe configuration → Сохранённый контекст каждой модели → Readback login/connection operation.
Фильтры/URL: Каталог / Авторизация / Проверки.

## Модели — /models

Цель: Имена API и версии профилей без скрытой подмены.

Класс: operational. Layout: wide.
Операции: createVirtualModel, readVirtualModel, listVirtualModels.
Содержимое: Slug → Display name → Mode → Active revision → Traffic → Expense confidence.
Фильтры/URL: Поиск профиля; Режим; Статус.

## Профиль — /models/:id

Цель: Save draft → verify exact draft → publish; ongoing requests frozen.

Класс: operational. Layout: reading.
Операции: updateVirtualModelDraft, verifyVirtualModelDraft, publishRevision, listRevisions, rollbackRevision, disableVirtualModel, enableVirtualModel, archiveVirtualModel.
Содержимое: Identity → Ordered deployments → Generation parameters → Bounds → Required capabilities → Allowed overrides → Deadline/max attempts → Proof → Publication → История и откат → Disable/enable/archive.
Фильтры/URL: Маршрут / Параметры / История.

## Запросы — /requests

Цель: Logical request и attempts не одно число.

Класс: operational. Layout: wide.
Операции: listRequests, readRequest.
Содержимое: ID/time → Virtual profile → Client → Actual model → Outcome → Cost knownness → Attempt count.
Фильтры/URL: Namespace в шапке; Период; Статус.

## Запрос — /requests/:id

Цель: Нельзя отправлять unknown request заново ради восстановления.

Класс: operational. Layout: detail-with-aside.
Операции: readRequest, readRequestResult, cancelRequest.
Содержимое: Frozen binding → Timeline → Actual model confidence → Usage category provenance → Confirmed/estimated/unknown → Held reserve → Receipt → Read result grant.
Фильтры/URL: Ход / Attempts / Usage / Учёт.

## Статистика — /statistics

Цель: Сопоставимые знаменатели и явная полнота данных.

Класс: operational. Layout: wide.
Операции: readStatisticsSummary, readStatisticsTimeseries, readStatisticsBreakdown, exportStatistics.
Содержимое: Known tokens → Rates/denominators → Latency/TTFT samples → Fallback → Availability probes → Quota freshness → Projection watermark → Breakdown.
Фильтры/URL: Namespace в шапке; Период; Валюта; Разрез.

## Расходы — /expenses

Цель: Не складывать валюты, estimate с заменившим receipt или allocation с cash fee.

Класс: operational. Layout: wide.
Операции: readStatisticsExpenses, createPriceRevision, listPrices, recordSubscription, listSubscriptions, requestLedgerCorrection, exportStatistics.
Содержимое: Confirmed → Estimated → Unknown count → Reserves → Currency groups → Receipts/corrections → Rate snapshots → Subscription period.
Фильтры/URL: Namespace в шапке; Период; Валюта; Списания / Тарифы / Подписки.

## Бюджеты — /budgets

Цель: Лимит проверяется до I/O; unknown reserve не свободный остаток.

Класс: operational. Layout: wide.
Операции: createBudget, updateBudget, listBudgets.
Содержимое: Policy → Used → Held → Available → 80/95 thresholds → Hard cap → UTC reset → Cost-unknown disclaimer.
Фильтры/URL: Scope; Currency; Period.

## Приложения — /clients

Цель: Сохранение доступа не выдаёт новый ключ автоматически.

Класс: operational. Layout: wide.
Операции: createClient, updateClient, issueClientKey, revokeClientKey, disableClient, enableClient.
Содержимое: Application identity → Project binding → Allowed profiles → Scopes → Expiry → RPM/concurrency → Cost policy → Keys → Несколько разрешённых профилей → Привязка Namespace → Выдача ключа отдельно.
Фильтры/URL: Namespace в шапке.

## Тестирование — /evaluations

Цель: Одинаковые datasets и frozen configurations.

Класс: operational. Layout: wide.
Операции: createDatasetVersion, listDatasetVersions, startEvaluation, listEvaluations.
Содержимое: Datasets → Version/hash → Runs → Pinned config → Quality/time/cost → Unknown/incomparable.
Фильтры/URL: Dataset/version; Profile revisions; Outcome.

## Результат теста — /evaluations/:id

Цель: Partial/unknown не successful comparison и не zero-cost победа.

Класс: operational. Layout: detail-with-aside.
Операции: readEvaluation, cancelEvaluation.
Содержимое: Frozen dataset/scorer → Exact profiles/parameters → Samples → Latency/token/cost → Incomparable → Artifacts access → Cancel/readback.
Фильтры/URL: Кандидат; Case; Outcome.

## Аудит — /audit

Цель: Immutable safe audit, без тела credentials.

Класс: operational. Layout: wide.
Операции: listAudit.
Содержимое: Actor/action → Object/revision → Operation/outcome → Reason → UTC timestamp.
Фильтры/URL: Namespace в шапке; Объект; Действие.

## Настройки — /settings

Цель: Параметры read-only; ничего похожего на env dump.

Класс: operational. Layout: reading.
Операции: readEffectiveSettings.
Содержимое: Own installation → Effective bounds → Auth/integration → Retention → Source/Base/config/schema identity.
Фильтры/URL: Раздел конфигурации.

## Вход в платформу — /login

Цель: Central SSO entry; Hub does not own passwords.

Класс: auth. Layout: reading.
Операции: Central Auth SSO entry; local password API отсутствует.
Содержимое: Нейтральный SSO entry → Возврат в целевой продукт.
Фильтры/URL: Нет.

## Общие условия

- Scoped страницы берут verified NamespaceRef; общие provider/profile настройки обозначены как уровень установки.
- Ошибка контекста не сбрасывает выбор на all; unbound показан отдельно. URL принадлежит вкладке.
- Loading/empty/error/403/404/partial/stale различаются. Dirty navigation/reload/close, validation/pending/412 сохраняют ввод.
- Mutation требует confirmed operation/readback; unknown не success. Key issue — отдельное действие.
- Prototype моделирует действия на безопасных fixtures; реальные rights, provider/billing и SSO acceptance not_run.

## Тарифы — /tariffs

Operational, wide. Tabs «Проекты» / «Себестоимость подключений» в URL. NamespacePicker общий; all показывает сравнение, exact active Namespace разрешает scoped edit. Default 20%; формы переключают markup/custom rates и auto/manual source, input/output/cached за 1M, currency/version/effective interval. Таблица показывает cost basis, начисление и margin отдельно. Operations: listPricingSources/createPricingSourceRevision/listProjectTariffs/createProjectTariffRevision/listPrices/createPriceRevision/listProjectCharges. Состояния pending/412/unknown сохраняют ввод и резерв.

## Полный pricing и scoring flow

/tariffs показывает все текущие connections/model/currency, shared PriceRevision history, profile/currency/UTC selectors и отдельную draft/activate timeline. /expenses ссылается на тот же справочник ставок; второго editor/store нет. Dataset/run читает frozen IDs/scorer/budget; readEvaluation и manual score показывают actor/time/case/candidate. Archive history сохраняет immutable records и не предлагает новые admissions.
