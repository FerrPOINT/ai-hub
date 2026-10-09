# Тарифы провайдеров и проектов

Статус: Target approved, документация и синтетический прототип. Реальные начисления отсутствуют.

## Две независимые цены

Себестоимость принадлежит точному подключению, upstream model, billing tier и валюте.
Цена проекта принадлежит проверенному NamespaceRef и виртуальному профилю Hub;
название проекта не идентичность. Подключения остаются общими для установки.
В разделе «Тарифы» доступны «Проекты» и «Себестоимость подключений».
Вход и выход задаются отдельно за **1 000 000 токенов**; cached input — отдельная
непересекающаяся категория, если её поддерживает квалифицированный adapter.

## Источник себестоимости

| Режим         | Подключение                                            | Правило                                                                                |
| ------------- | ------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| provider_auto | OpenRouter                                             | Каталог даёт quote ставок; ответ даёт usage и фактическую стоимость                    |
| manual        | Ollama Online, ChatGPT по подписке, другие подключения | Оператор вводит input/output/cached ставки, валюту и evidence; это расчётная стоимость |

Переключатель «Автоматически от провайдера / Ввести вручную» задаётся для точной
пары connection/model. Auto включается только у adapter с qualified pricing reader.
У Ollama Online и ChatGPT по подписке в v1 auto выключен; нельзя подставить нулевую
или выдуманную цену. Изменение режима создаёт новую revision и не меняет историю.
Ручной режим требует существующую immutable PriceRevision. Название модели само
по себе не выбирает цену другого подключения. Для subscription charge остаётся
один cash payment; расчёт по токенам — отдельная allocation, не второе списание.

## OpenRouter

Проверено по официальным источникам 2026-10-09:
[usage accounting](https://openrouter.ai/docs/cookbook/administration/usage-accounting),
[model catalog](https://openrouter.ai/docs/api/api-reference/models/list-all-models-and-their-properties).
OpenRouter возвращает prompt/completion usage и cost; в stream usage приходит в
последнем SSE событии. Отдельные include_usage параметры больше не нужны.
`usage.cost` — стоимость для аккаунта OpenRouter; `cost_details.upstream_inference_cost`
описывает другой уровень и не прибавляется повторно. Currency/credit conversion
должны быть подтверждены adapter receipt; валюту не угадывать по имени провайдера.
Catalog pricing нормализуется из per-token в per-million exact decimal, с source/time/tier.
Каталог не подтверждает списание. Trusted generation-ID readback разрешён для
reconciliation; повторный inference не является способом получить стоимость.
Отсутствующий cost остаётся unknown либо estimated по qualified usage и price snapshot.
Для BYOK adapter отдельно учитывает подтверждённые источники списаний без двойного
суммирования cost и upstream cost. Эта разновидность требует отдельной квалификации.

## Цена проекта

По умолчанию действует режим default_markup: **20% наценки на себестоимость**
(markup_bps=2000), а не 20% прибыли от выручки. Проект может выбрать собственную
наценку либо custom_rates: свои input/output/cached ставки за миллион токенов.
Тариф задаётся по Namespace/profile/currency; profile=null — общий тариф проекта.
Приоритет: exact Namespace/profile → общий Namespace → установка, default 20%.
Перекрывающиеся активные интервалы одного ключа отвергаются; currency не конвертируется.

- Default: project charge = cost basis × (10000 + markup_bps) / 10000.
- Custom: project charge = Σ(disjoint billable tokens × project rate / 1000000).
- Margin amount = project charge − cost basis; отрицательная маржа допустима и видима.
- Для OpenRouter default basis — подтверждённый account charge всех attempts,
  включая fallback и неуспешные платные попытки. Оценка даёт provisional charge.
- Для ручной себестоимости basis — token allocation; статус не provider-confirmed.
- Custom rate charge требует verified usage. Без него сумма pending/null, не ноль.
- Unknown receipt удерживает прежний provider reserve; tariff не освобождает резерв.
- Existing budget policies продолжают ограничивать provider cost. Цена проекта
  отдельно показывается в учёте; project-charge budgets требуют будущего явного контракта.

Admission фиксирует tariff revision, cost-source revision, profile revision, Namespace,
валюту и категории. Future tariff не переписывает старый request; late receipt
считает корректировку по прежнему snapshot. История append-only и идемпотентна по
attempt/source receipt; период суммируется из canonical charges, не строк таблицы.
Изменение тарифов доступно ai-hub:write в разрешённом scope; Namespace archived/unavailable
закрывает новые revisions, но сохраняет чтение authorised history.

## Точность и пример

Provider cost сохраняет NUMERIC(38,18); rates — NUMERIC(30,12).
Project charge/margin используют NUMERIC(50,24), чтобы multiplication наценки
не округляла каждый request. API передаёт decimal strings; округление half-even
до minor currency unit выполняется для отображения итогов. Binary float запрещён.

Синтетический пример: input=1000000, output=1000000, cost rates=2/8 USD.
Basis=10 USD; default project charge=12 USD; margin=2 USD.
Custom project rates=3/10 USD дают charge=13 USD и margin=3 USD.
Это отдельные факты от subscription cash fee и provider receipt.

## Контракты и проверка

OpenAPI: listPricingSources, createPricingSourceRevision, listProjectTariffs,
createProjectTariffRevision, listProjectCharges. POST требует Idempotency-Key и
expected_version для CAS по logical key; initial version=0, stale → 412.
Default 20% — продуктовая policy, реальные ставки не вшиваются в приложение.
UI формы показывают source, currency, unit, effective interval и revision.
TC-038 проверяет decimal precision, automatic/manual, отсутствующий usage/cost,
exact Namespace identity, custom rates, immutable historical snapshot и dedupe.
