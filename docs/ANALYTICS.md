# Статистика

Canonical source — requests/attempts/usage facts/financial ledger; aggregates
перестраиваемы. UI не складывает несовместимые provider ответы и не считает
statistic card источником денег.

## Измерения и фильтры

UTC half-open [from,to), input ISO8601 с offset нормализуется в UTC.
UI timezone явно выбран пользователем; raw export содержит UTC и timezone metadata.
Default range=7 дней, max online=90 дней; export <=366 дней, rows<=100000.
Filters: installation, project, client, delegated_user, virtual_slug,
profile_revision, provider, actual_model, status, evaluation_run, currency.
Все IDs проходят server authorization. Project/client выбираются по grant,
request body user/metadata не trusted identity.
Breakdown by ограничен allowlist; лимит групп 100, remainder отдельным Other bucket.

## Определения метрик

| Метрика        | Правило                                                                       |
| -------------- | ----------------------------------------------------------------------------- |
| requests_total | Число logical admitted requests; pre-admission rejects отдельно               |
| attempts_total | Все durable provider attempts, probe/eval marked separate                     |
| success_rate   | completed / terminal requests; unknown/in-progress отдельно, не в denominator |
| error_rate     | failed / terminal; cancelled отдельно, denominator явно выдан                 |
| fallback_rate  | requests с >1 attempted deployments / admitted requests                       |
| latency        | finished_at-admitted_at; percentiles terminal samples, sample_count           |
| TTFT           | first_content_at-dispatched_at; only content events, null при отсутствии      |
| token totals   | Sum known category facts; unknown_usage_attempts отдельно                     |
| expenses       | Per-currency confirmed/estimated/unknown + held reserves; без двойного счёта  |
| availability   | Explicit bounded probes; probe count/window, не inferred из пустого трафика   |
| quota          | Latest own account observation + freshness; не выдуманный remaining           |
| throughput     | requests/seconds окна; started/finished series различаются                    |

Request-level статистика группируется по admitted_at; attempt usage — dispatched_at;
cash expenses — charge occurred_at; subscription service period отдельный filter.
Не называть эти наборы одним и тем же периодом без metadata.
Пустой подтверждённый набор даёт 0/count=0; источник недоступен — error,
не нулевой успешный dashboard.

## Consistency

Response: as_of, projection_watermark, lag_seconds, data_status=complete|partial|stale,
known/unknown counts, filter echo и source basis. Fact ingestion exactly-once via
dedupe; late fact обновляет нужные buckets через idempotent rebuild.
Totals и breakdown из одного repeatable-read snapshot. No approximate money.
Для запросов p50/p95 вычисляются exact percentile по bounded window на v1;
не суммировать percentile bucket values.

Partial analytics сохраняет доступные cards, показывает failed sections и
last successful values с timestamp. Export использует тот же authorized snapshot;
CSV strings защищены от formula injection, decimals и null semantics сохранены.
Projection rebuild никогда не запускает model calls.

## Актуализация 2026-10-09

Namespace UUID pair и binding=unbound входят в filter echo и cursor identity. Конфликт pair/unbound — ошибка, не all fallback. Budget period — текущий UTC месяц/день; диаграмма периода показывает cash occurrence, reserve остаётся отдельным current snapshot.

## Начисления проектам

В /tariffs отдельно показаны cost basis/confidence, project charge, margin и tariff revision. Суммы из project_charge_events не прибавляются к provider cash expenses. Currency/Namespace/range/snapshot identity общие для списка и экспорта; pending/provisional не confirmed expense. [Тарифы](PROJECT_TARIFFS.md).

## Canonical export и echo

Statistics.filter_echo включает all/unbound/namespace, exact UUID pair, range/currency/group_by и все фильтры dimensions/status. CSV/JSON сериализуют canonical statistics snapshot; display strings, валютные glyph и локальные даты не источник данных. Sample request/receipt таблицы явно неполные; агрегаты не превращаются в выдуманную историю. Разрезы profile/client используют одну synthetic matrix.

## Единственный filter contract

StatisticsFilters — canonical resolved shape для summary/breakdown/export/cursor.
Все поддерживаемые query selectors имеют x-filter-field и $ref на его property.
provider → provider_id; connection_id отдельный scope; actual_model и evaluation_run_id
не теряются. project → verified project_binding, client/model/revision → opaque UUID,
delegated_user → user_subject. Namespace UUID pair flatten только на HTTP boundary.
Missing selector — explicit null в echo; при query serialization null опускается.
Timezone — validated IANA label zone, UTC half-open filter/bucket boundaries сохраняются.
UI group label model маппится в API virtual_model; display label не wire enum.
Unsupported/malformed/foreign selectors отвергаются, не расширяют выборку.
