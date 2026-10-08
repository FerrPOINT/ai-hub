# Наблюдаемость

## Health

Target GET /health/live — process; /health/ready — own DB/schema/key/config/worker.
Не делать платный inference внутри health или restart loop.
Platform health.read использует GET /health alias собственного readiness;
fixed integration/branding contract endpoints возвращают только safe metadata.
Readonly build/info содержит source/Base/schema/config revisions без private paths.

## Metrics

Counters logical requests/admission_denied/attempts/outcomes/fallback, token category
known/unknown, ledger settlement/unknown, reservation held, worker/reconciliation,
projection lag. Histograms own overhead/request latency/TTFT/DB latency.
Labels bounded provider kind/outcome/capability/confidence; request/user/project/
model arbitrary IDs не Prometheus labels, они authorized statistics dimensions.

Logs structured, safe event + request/attempt/correlation ID, redaction before sinks.
No payload/credential/native auth code/raw upstream errors. Audit — DB domain
event, не заменяется logging. Telemetry drop не разрешает ledger drop/retry.

## Alerts и тесты

Projection lag>60s sustained 5min, unknown attempts>0 for 10min, pool saturation,
vault/readiness errors, hard budget denial/overrun и provider rejection trend.
Пороговые значения рабочие defaults и effective config; zero-traffic no availability.
Backend/adapter/consumer timestamps различаются; latency attributed correctly.
TC-016 проверяет outages/redaction, TC-014 — sample/denominator semantics.
