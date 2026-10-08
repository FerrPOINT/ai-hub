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
