# Архитектура AI Hub

Статус: проектный baseline v1. Работающие процессы не созданы.

## Компоненты и поток

| Компонент         | Вход                                       | Результат / владелец                                       |
| ----------------- | ------------------------------------------ | ---------------------------------------------------------- |
| Control API       | Verified central principal                 | Settings, revisions, clients, prices, subscriptions, audit |
| Inference API     | Hub client key или trusted service grant   | Admission и frozen request context                         |
| Routing service   | Frozen profile, capabilities, quota/budget | Ordered eligible deployment; explicit attempt              |
| Provider adapter  | Exact normalized invocation                | Typed events, usage facts и acceptance certainty           |
| Accounting        | Canonical request/attempt/provider facts   | Ledger, reservations, settlements и corrections            |
| Statistics API    | Authorized filter и as_of                  | Canonical sums/derived buckets и freshness                 |
| Evaluation runner | Frozen run config и own grant              | Те же requests/ledger, scores без tool execution           |
| React web         | Control API и central auth                 | Presentation, drafts, filters, безопасные actions          |

Зависимости: transport вызывает application, application опирается на domain
и port interfaces; infrastructure реализует ports. Ни SQL в handlers, ни provider
политики в UI. Один lifecycle-managed HTTP client pool на adapter.
Первый deploy — один Rust API process с bounded own recovery/evaluation workers;
разделение процессов только при доказанной нагрузке, не отдельный orchestrator.

## Request context

Admission один раз разрешает identity, client/project grants, frozen profile,
required capabilities, limits, applicable budget rows и price snapshots.
Для verification используется отдельный immutable probe snapshot, не published
revision: own verification grant разрешает только bounded proof operation.
Обычный inference client не может выбрать этот kind и обойти publication.
Verification/evaluation используют own internal clients с reserved application keys,
explicit scoped grant и actor attribution. Они создаются как own installation records
и не получают default unlimited budget, credentials или права соседних сервисов.
Контекст передаётся слоям, не собирается повторными запросами в каждом adapter.
Caller не может заменить connection или подменить grants через metadata.

## Транзакционные границы

1. До external I/O транзакция создаёт request/idempotency binding,
   attempt dispatch intent и reservations по всем применимым budget rows.
2. Adapter dispatch имеет stable attempt ID. Внешний I/O не входит в DB transaction.
3. Terminal facts и settlement/audit записываются атомарно. Duplicate terminal
   event не изменяет totals. Unknown сохраняет резерв и запрещает автоматический resend.
4. Aggregate projections строятся после canonical commit; projection failure
   не отменяет facts и не разрешает повторный provider dispatch.

Recovery работает только по durable own state. DB unavailable до intent — отказ
до I/O. После dispatch при потере БД результат не объявляется сохранённым;
attempt после restart unknown до reconciliation.

## Выбор реализации

Native Rust adapters дают единые grants, admission и бухгалтерскую границу.
LiteLLM/Octo остаются reference и возможным явным upstream compatibility adapter;
они не являются вторым владельцем Hub config/accounting.
Если upstream скрывает фактические attempts или pricing, confidence не повышается,
pinned evaluation через такой upstream не qualified.
Повторная generic proxy реализация и копирование соседней vault/DB отклонены.

## Что требуется до live

Accepted capability receipts конкретных adapters, service grant integration,
readback фактических models, central auth/SSO, exact-image target deployment,
recovery/restore и user scenarios. Source docs закрывают только design gate.
