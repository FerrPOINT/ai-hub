# Учёт расходов и бюджетов

## Денежные представления

Currency — трёхбуквенный код; amount — decimal string в API и NUMERIC(38,18)
в PostgreSQL; per-million rate — NUMERIC(30,12). Binary float запрещён.
Проверить диапазон/scale до сохранения, без implicit DB округления.
Financial atomic unit=10^-18; rate unit=10^-12 per million. Поэтому cost atomic units
= integer tokens × scaled rate units: division уже учтена в scale, per-request
округления нет. Rust checked i128/валидированный DB decimal codec, exact arithmetic
и overflow отказ до I/O; не hand-rolled floating price math.
Отображение округляется
half-even до minor units валюты только после суммирования, не по каждому запросу.
Different currencies выдаются отдельными totals. В v1 нет FX conversion.

Все цены synthetic в примерах. Реальные rates оператор вводит с evidence/source,
currency, unit=per_million_tokens, effective interval и immutable revision.
Подмена тарифов в code/defaults запрещена. Effective interval опубликованного source binding не перекрывается для
одного connection/model/current tier/currency. Immutable PriceRevision содержит allowed
window и может пересекаться с другими quotes. Timeline closing происходит по следующему
source effective_from или собственному effective_to, без UPDATE старой quote. Admission
фиксирует exact quote/source snapshot.

## Cost status

| Статус    | Доказательство                                                              |
| --------- | --------------------------------------------------------------------------- |
| confirmed | Provider charge/statement receipt с trusted ingestion и stable external ID  |
| estimated | Проверенные usage facts × exact price revision, без подтверждения списания  |
| unknown   | Нет полного usage/pricing/acceptance либо billing pending                   |
| zero      | Не статус: amount=0 с confirmed/estimated и evidence-free/quota explanation |

Просто usage и price дают estimated. User/manual correction не повышает confidence
до confirmed без independently trusted receipt. Missing cost не число 0.
Response tokens не означают реальное списание. Invoice confirmation может прийти
позже и создаёт settlement/correction; исходный факт не переписывается.

## Formula и примеры

Категории адаптера приводятся к непересекающимся input_uncached,
input_cached, cache_write, output_billable и explicit_other. Reasoning обычно
является частью output; отдельная цена допустима только при непересекающемся
provider billing contract. Все categories имеют provenance/knownness.

cost = Σ(category_tokens × price_per_million / 1000000) + known fixed request fees.
Если input_tokens включает cached: uncached=input_tokens-cached, только когда
adapter contract подтверждает такое включение. Отрицательная разность — typed
usage_invalid, не clamp/repair.

Synthetic: input 2000, из них cached 800; output 1000, из них reasoning 200.
Rates: uncached 2 USD/1M, cached 0.5, output 8.
Estimated = 1200×2/1M + 800×0.5/1M + 1000×8/1M = 0.010800000000 USD.
Reasoning второй раз не добавляется.
Предыдущая платная failed attempt 100 uncached tokens добавляет 0.000200000000:
один logical request, две attempts, estimated total 0.011000000000.
Machine example: [accounting-example.json](examples/accounting-example.json).
Precision case: 1 token при 0.000000000001/1M = 0.000000000000000001,
сохраняется в ledger без округления до нуля. Confirmation/correction никогда
не обрезается: out-of-range receipt сохраняет safe evidence/reason и блокирует
новые paid admissions до explicit reconciliation, не fabricated zero.

## Ledger и reconciliation

Append-only entries: reserve, release, charge, correction, subscription_charge.
Each fact имеет unique installation/source/external_event_id. Replay не дублирует
entry. Adjustment ссылается на original entry; отрицательный credit допустим только
для correction с причиной и evidence. Изменение price не пересчитывает историю
скрытно; explicit re-estimation создаёт revision/correction с сохранением прежнего.

Effective expense по одному charge key берёт confirmed settlement вместо associated
estimate, не суммирует их. Estimated и unknown counts показываются рядом с confirmed.
Fee каждой upstream attempt учитывается, включая failed/fallback/evaluation calls.

## Subscription и quota

Subscription charge хранит provider connection, amount/currency, service period,
payment status, receipt и optional allocation policy. Оплата месяца учитывается
один раз, а не на каждом token request. Период [start,end) UTC.
Quota: observation/source/as_of/reset_at/unit/limit/remaining; недоступная quota null.
Включённые tokens имеют zero incremental marginal cost только если это доказано.
Per-request allocated subscription cost — отдельная аналитическая оценка,
по умолчанию выключена; не прибавляется к cash expenses повторно.
One-month invoice не дробится автоматически в cash report; period allocation
в отдельной projection сохраняет policy version и partial-period coverage.

## Budget admission

Budgets с currency и scope installation/project/client/profile; период UTC day
или calendar month; warning thresholds default 80%/95% и hard_limit.
Одна request должна удовлетворить всем applicable budgets одной транзакцией:
locks в стабильном scope/id порядке, usage/rate bounds + existing reserved/charges.
Reservation строится на upper bound input + capped output + bounded attempt fees.
Fallback резервирует возможную следующую attempt до I/O; нельзя «умножить» квоту
через несколько инстансов. Unknown outcome удерживает соответствующий reserve.

Known cost overrun фиксируется как fact и блокирует новые paid admissions;
действительные расходы не урезаются до лимита. Unknown/unbounded price не допускается
под hard monetary budget; оператор может дать explicit cost-unknown grant с
token/request/concurrency caps, и UI обязан показать отсутствие денежной гарантии.
Subscription fee учитывается budget по его scope; не смешивать usage quota и cash.

## Фиксированный сбор

PriceInput.request_fee — известный fixed per-attempt amount38/18 либо null unknown.
Отсутствие поля не означает zero-fee. Trusted receipt может подтвердить fee даже
при квалифицированном no-acceptance rejection; tokens при этом не выдумываются.
Эта fee snapshot включается в upper bound/reservation и per-attempt ledger.
Допустимость fallback определяется независимой acceptance certainty из ROUTING.

## Цена проекта

[PROJECT_TARIFFS](PROJECT_TARIFFS.md) отделяет account expense от project charge и margin. Default markup=2000 bps; manually allocated subscription token cost не создаёт provider receipt. Project amounts NUMERIC(50,24); неизвестный basis не обнуляет reserve. Existing monetary budgets используют provider cost.

## Отсутствующая цена: допустимый unknown path

При explicit cost_unknown_allowed и bounded token/request/concurrency grants admission
может зафиксировать отсутствие pricing source. Frozen snapshot содержит
cost_source_state=unconfigured и cost_source_revision_id=null; ID/ставка не создаются
для заполнения отчёта. ProjectCharge: basis_confidence=unknown, cost_basis=null,
margin_amount=null. Default markup даёт project_amount=null/status=pending.
Custom rates могут дать известный project_amount только из qualified disjoint usage
в явно разрешённой валюте; margin остаётся null до доверенного basis. Без usage — pending/null.
Expired/unavailable source, если он существовал, сохраняет настоящий snapshot ID и
state=configured: наличие настройки не повышает confidence. Новая ручная цена не
переоценивает старый unconfigured snapshot. Trusted late receipt даёт append-only
correction с прежним tariff; receipt provenance хранится отдельно от pricing configuration.
Нет денежного upper bound — нет fabricated zero reserve: отсутствие monetary guarantee
явно показано, token/concurrency caps действуют. Ранее созданные uncertain reserves удерживаются.

Для unconfigured source валюта берётся из independently qualified account billing_currency с statement/receipt provenance (runtime_qualifications), не из имени провайдера, UI default или caller metadata. Нет квалифицированной валюты — currency_unqualified до I/O. cost_unknown_allowed не отменяет применимые hard monetary budgets: без upper bound такой budget блокирует dispatch; разрешённый unknown path требует отсутствия guaranteed monetary bounds и явно ограниченного token/request/concurrency grant.

Configuration presence и financial trust независимы: basis_source=unavailable/price_estimate/provider_receipt. Первый unconfigured event имеет null/unknown basis; поздний trusted receipt может подтвердить basis, сохраняя cost_source_state=unconfigured и null configuration ID. Append-only supersedes event использует исходный tariff и настоящий receipt event identity; новую price row не создавать. Effective sums заменяют previous pending/provisional, не прибавляют его второй раз.

Provider NUMERIC(38,18) может использовать checked i128 scaled units; project NUMERIC(50,24) требует bounded arbitrary-precision decimal/integer codec, не reused i128. Limits/overflow проверяются до I/O и DB cast. Manual score JSON numeric lexeme также проверяется exact decimal для scale .001, не по float remainder.
