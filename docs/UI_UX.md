# Карта UI/UX AI Hub

Статус: Target approved для приложения; HTML — интерактивный prototype. Общий UI/UX Standard принадлежит Base.

| Route | Класс | Layout | Доступ | Fixture |
| --- | --- | --- | --- |
| / | operational | wide | central_verified_with_product_grants | design/prototype.html#/ |
| /providers | operational | wide | central_verified_with_product_grants | design/prototype.html#/providers |
| /providers/:id | operational | detail-with-aside | central_verified_with_product_grants | design/prototype.html#/providers/api-a |
| /models | operational | wide | central_verified_with_product_grants | design/prototype.html#/models |
| /models/:id | operational | reading | central_verified_with_product_grants | design/prototype.html#/models/main-dev |
| /requests | operational | wide | central_verified_with_product_grants | design/prototype.html#/requests |
| /requests/:id | operational | detail-with-aside | central_verified_with_product_grants | design/prototype.html#/requests/a17d |
| /statistics | operational | wide | central_verified_with_product_grants | design/prototype.html#/statistics |
| /expenses | operational | wide | central_verified_with_product_grants | design/prototype.html#/expenses |
| /budgets | operational | wide | central_verified_with_product_grants | design/prototype.html#/budgets |
| /clients | operational | wide | central_verified_with_product_grants | design/prototype.html#/clients |
| /evaluations | operational | wide | central_verified_with_product_grants | design/prototype.html#/evaluations |
| /evaluations/:id | operational | detail-with-aside | central_verified_with_product_grants | design/prototype.html#/evaluations/run-24 |
| /audit | operational | wide | central_verified_with_product_grants | design/prototype.html#/audit |
| /settings | operational | reading | central_verified_with_product_grants | design/prototype.html#/settings |
| /tariffs | operational | wide | central_verified_with_product_grants | design/prototype.html#/tariffs |
| /login | auth | reading | anonymous | design/prototype.html#/login |

## Namespace и сохранение состояния

Picker содержит all/active/unbound и unavailable/archived/invalid. Выбор хранится UUID-парой URL и не влияет на соседнюю вкладку. В выбранном Namespace нет второго project selector. Provider/model настройки общие для установки; client/dataset/request/expense/project budget scoped.
Фильтры и tabs восстанавливаются reload/Back/Forward. Черновики не попадают в URL/localStorage. Dialog close/Escape и смена контекста требуют явного discard при dirty. Pending исключает duplicate submit; 412 сохраняет draft.
Синтетические факты и prototype evidence не доказывают live authorisation, capability или billing. Полная приёмка по [DESIGN_ACCEPTANCE](DESIGN_ACCEPTANCE.md) и [TESTING](TESTING.md).

## Раздел тарифов

/tariffs — operational wide page. Pricing tab хранится в allowlisted URL tariff_tab. При selected Namespace нет второго project selector. Currency/rates/source/version и input/output units видимы. Auto без qualified источника закрыт; manual empty не бесплатный тариф. Unknown и subscription allocation имеют самостоятельные подписи.

## Состояние и контекст

Все tab families provider/expense/tariff используют allowlisted URL и native history. Несохранённые model fields/version живут отдельно от saved snapshot; native Back/Forward получает stay/discard guard. Namespace disabled state запрещает writes/admissions, сохраняет authorised history. Dataset project выводится из Namespace readback; второго editable selector нет. Rows/cursors/queries используют resource ID, никогда display text.
