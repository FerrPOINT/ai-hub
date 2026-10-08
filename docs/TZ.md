# Техническое задание AI Hub

## Назначение

Пользователь хочет отдельный сервис, через который приложения и агенты обращаются
к нейронкам. Можно создавать собственные виртуальные модели для разработки и
тестирования, как в существующем Octo gateway. В обязательный объём входят разные
виды статистики и расходы. Технологии соответствуют Rust/React продуктам Base.

## Сценарии

1. Оператор подключает provider account, проверяет точные capabilities и лимиты.
2. Создаёт main-dev с основным upstream и разрешёнными резервами; публикует revision.
3. Выдаёт приложению scoped access. Приложение выбирает стабильное имя профиля,
   не знает provider credentials, получает stream или полный ответ.
4. В request detail видно revision, выбранный и фактически ответивший upstream,
   все attempts, tokens, latency и confidence денежных величин.
5. Для app-test создаётся pinned revision без fallback; один dataset выполняется
   на нескольких профилях с одной конфигурацией теста и отдельным бюджетом.
6. Оператор видит расходы по дням, клиентам/проектам/providers/models, задержки,
   ошибки, ограничения, quota и приближение к лимитам.
7. При неизвестном исходе внешнего вызова сохраняются evidence и reservation,
   повтор не создаёт второй платный вызов.

## Scope v1

Text inference, streaming, function tools, structured outputs при фактической
поддержке модели; provider API-key и managed subscription adapters;
statistics/financial ledger, budgets, tests, UI и integration contracts.

В будущем: media/embeddings, model training, chat/RAG portal, автооптимизация
маршрутов, публичная коммерческая тарификация. Эти задачи не нужны для начала v1.
User-defined model означает виртуальный профиль существующих upstreams.

## Authority

Идентификаторы FR/NFR и критерии — только
[PRODUCT_REQUIREMENTS](PRODUCT_REQUIREMENTS.md).
Правила денег — [ACCOUNTING](ACCOUNTING.md), API — [API](API.md), UI — [UI_UX](UI_UX.md).
Порядок реализации — [IMPLEMENTATION_PLAN](IMPLEMENTATION_PLAN.md).
В документировании нет permission на перенос secrets или изменение runtime соседей.
