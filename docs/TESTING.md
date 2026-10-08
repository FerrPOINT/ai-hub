# Проверки и сценарии

Все строки ниже — planned behavioral tests, не исполненные результаты.
Документационный gate не является реализацией этих сценариев.

| Test   | Требование | Поведение/negative/recovery oracle                                                                                                                                                     |
| ------ | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TC-001 | FR-001     | Создать connection с новым display name/ID; permitted endpoint succeeds; неизвестный adapter и private redirect denied; disable сохраняет history.                                     |
| TC-002 | FR-002     | Записать/сменить/revoke synthetic credential; искать canary secret в API, logs, traces, browser storage, exports; wrong-key restore denied.                                            |
| TC-003 | FR-003     | Catalog metadata без proof publication denied; exact proof succeeds; смена generation/model/draft/adapter или expired proof вновь denied.                                              |
| TC-004 | FR-004     | Два concurrent drafts: один CAS succeeds, второй 412 с сохранённым draft; pointer rollback не меняет frozen request; duplicate slug denied.                                            |
| TC-005 | FR-005     | Controlled primary 429/no acceptance и reserve secondary дают две attempts; connect-before-send fallback; timeout/5xx-after-send и output fragment не fallback.                        |
| TC-006 | FR-006     | Один key/payload повторить до/после restart; dispatch count=1; different payload/principal conflict/denied; expired payload readback не resend.                                        |
| TC-007 | FR-007     | Public chat/Responses normal/stream/tools/continuation/JSON через controlled server; case-sensitive args retained; unsupported field/native tool/stateful request rejected before I/O. |
| TC-008 | FR-008     | Own subscription login/cancel/logout generations; invalidated authorization не возобновляется; quota absent null; recurring payment не множится по calls.                              |
| TC-009 | FR-009     | Provider actual model отличается от requested alias: показать both/verification; pinned run incomparable; virtual response slug remains caller name.                                   |
| TC-010 | FR-010     | Вычислить synthetic cached/reasoning example exact decimal; unknown category/rate дает unknown; invalid negative counts rejected; stale price snapshot not rewritten.                  |
| TC-011 | FR-011     | Duplicate trusted receipt и subscription payment дают одну charge; late confirmation replaces estimate; refund creates correction; forged amount/status not accepted.                  |
| TC-012 | FR-012     | Два API instances параллельно reserving последний бюджет: ровно один dispatch; multiple scopes/currencies; warning dedupe; unknown holds; overrun visible.                             |
| TC-013 | FR-013     | Same authorized snapshot overview/breakdown/timeseries согласованы; late facts rebuild; timezone/DST/half-open boundary, empty confirmed vs source error различаются.                  |
| TC-014 | FR-014     | Dataset с completed/failed/cancelled/unknown/inprogress и zero traffic: exact denominators/sample counts/percentiles/TTFT, probes separate from user calls.                            |
| TC-015 | FR-015     | Client A пытается export B через прямой query/ID/cursor; deny. Decimal CSV value/null match JSON; formula text escaped; oversized export bounded.                                      |
| TC-016 | FR-016     | Down ledger DB отказ до I/O; metrics collector failure не duplicate dispatch; lag/unknown alerts без high-cardinality labels и private bodies.                                         |
| TC-017 | FR-017     | Одинаковые frozen cases/config на двух qualified pinned models, no fallback/cache; seed disclaimer; partial cancel/restart no repeated case calls, cost per actual attempt.            |
| TC-018 | FR-018     | Human session, PAT read/write и inference key матрица; project binding spoofed body ignored/denied; protected direct route/details/cursors enforce backend grants.                     |
| TC-019 | FR-019     | Revoke/expire session/PAT/key/connection перед admission/next attempt и во время UI; no new I/O; no unexpected re-login/global credential fallback.                                    |
| TC-020 | FR-020     | Allowlist local Ollama exact target работает; SSRF/DNS rebinding/redirect/proxy env denied; prompt tool call возвращён caller без local side effect.                                   |
| TC-021 | FR-021     | Consumer на old endpoint и explicit opt-in Hub key; exact revision/generation mapped; cutover/rollback no duplicate inference or shared DB writes.                                     |
| TC-022 | FR-022     | Backup synthetic DB+vault отделён от key; own isolated restore external_calls off; dedupe/unknown reserves/audit links equal; existing data target rejected.                           |
| TC-023 | FR-023     | Mutation и audit fail атомарно при DB failure; repeated idempotent operation не audit duplicates; no secret body/raw provider error in record.                                         |
| TC-024 | FR-024     | Все routes loading/empty/error/403/404/partial и dirty/pending/412; screenshot + real API scenario; unavailable provider не empty success.                                             |
| TC-025 | FR-025     | USD/EUR/zero/unknown + estimate then confirmation + fixed subscription: per-currency totals exact; no combined mixed currency or double allocation.                                    |
| TC-026 | FR-026     | Context/output/schema deadline mismatch до I/O; 429 safe Retry-After; provider error redacted and distinct from invalid input; stream terminal not false success.                      |
| TC-027 | NFR-001    | DB concurrency/dedupe/correction unique constraints и compare canonical amounts до/после aggregate rebuild; transaction rollback on partial write.                                     |
| TC-028 | NFR-002    | Crash before intent, after intent, after dispatch, before terminal commit; stable request IDs/reserves; reconcile without redispatch; cancellation not free unknown.                   |
| TC-029 | NFR-003    | Exact-source isolated load fixture/hardware receipt, 20 concurrency, 100k rows: measure own overhead/statistics limits; slow provider separated; memory bounded.                       |
| TC-030 | NFR-004    | Materialize pinned Base, verify clean revision; frozen dependency build/readback hashes; CI/local images use same toolchain, changes to lock fail frozen gate.                         |
| TC-031 | NFR-005    | Advance TTL clock: replay payload/artifacts removed, dedupe/ledger keys retained; content denied after expiry, billing readback remains; no provider calls by TTL worker.              |
| TC-032 | NFR-006    | 375/1440/1920/2560 across supported themes: keyboard tab/Escape/return focus, no horizontal body overflow; unknown expense accessible label and readable table.                        |
| TC-033 | NFR-007    | Old supported client payloads retained, contract draft→generated parity, reject unsupported semantic fields; OpenAPI/schema compatibility and migration previous schema.               |
| TC-034 | NFR-008    | Final merged source/Base/image artifact hashes match served target; authenticate UI + actual provider/consumer checks; old screenshot/probe receipt rejected as current.               |

## Уровни доказательств

Unit: pure routing/decimal/state rules. Contract: typed DTO/wire/SSE/tool semantics.
Own PostgreSQL: migration/locks/dedupe/CAS/reconciliation/projection rebuild.
Controlled HTTP server: actual dispatch counts/errors/backpressure/cancel.
Live provider: account/access/actual model/tokens/capabilities/billing evidence.
Consumer: real configured application/service grant and continuation.
Browser: authenticated served app through project-approved in-app browser,
screenshots with source/viewport/theme/state provenance.
Restore/load: own isolated state and receipts, without writes to protected neighbors.

Mocks можно использовать внутри unit tests, но они не закрывают boundary gates.
No test coupled только к исходным имёнам/IDs; varied names/IDs/grants/state.
Financial tests include near-zero/large amounts, cache/reasoning overlap,
partial usage, ambiguous failures, duplicate invoice, refund, period boundaries,
multi-currency и concurrency across processes.
Цена synthetic в offline example проверяет арифметику, не provider bill.
