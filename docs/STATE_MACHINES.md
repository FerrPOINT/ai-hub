# Состояния и атомарные границы

Wire states принадлежат OpenAPI; storage fields — DATA_DICTIONARY.
Это проектные transitions, не реализованный state engine.

## Control operation

| From → To                  | Guard                                     | Durable result                              |
| -------------------------- | ----------------------------------------- | ------------------------------------------- |
| New → pending              | Valid principal/key/body/expected version | Operation binding + intent                  |
| pending → succeeded        | Own commit/result receipt                 | Safe metadata + audit atomic                |
| pending → failed           | Definitive rejection                      | Safe reason; no invented success            |
| pending → unknown          | External side effect ambiguous            | Retain intent/generation; no resend         |
| unknown → succeeded/failed | Trusted readback same operation           | Original ID/history, no second side effect  |
| pending → cancelled        | Cancellation before dispatch              | Known terminal without external side effect |

Unknown login/logout prevents fresh authorization operation until readback.
Safe_result never contains raw provider/client keys. One-time client key lost:
409 one_time_output_unavailable + safe key metadata → explicit rotation.

## Profile

Draft create/save uses full validated settings and CAS, no proof prerequisite.
Verify snapshots draft version/hash/ordered targets + budget purpose; no activation.
Publish requires exact fresh profile proof and current generations/eligibility.
Transaction creates immutable revision + CAS active pointer + audit.202 operation
может быть already succeeded; worker polling не дополнительная remote activation.
Disable blocks new admissions; enable validates current qualified receipt. Archive
requires disable, history retained. Runtime qualification invalidation отдельна от
15-minute publication freshness. Started requests retain frozen revision.

## Request и attempt

| Transition                                        | Rule / financial effect                                           |
| ------------------------------------------------- | ----------------------------------------------------------------- |
| Admission → intended                              | Frozen context + unique key + all budget locks/reserves committed |
| intended → dispatched                             | Stable attempt ID; external I/O outside transaction               |
| dispatched → streaming                            | First content timestamp; fallback permanently closed              |
| dispatched/streaming → completed/failed/cancelled | Trusted terminal facts + settlement atomic                        |
| dispatched → unknown                              | Accepted/billing unknown, hold reserve, no replay                 |
| unknown → terminal                                | Reconcile same provider ID/receipt; never fresh inference         |
| cancellation requested                            | Separate durable flag; response cancellation не charge=0          |
| Duplicate terminal event                          | Dedupe → no second charge/audit delta                             |

Lost DB after dispatch: transport may fail, record remains intent/unknown on recovery;
не «successful persisted response». Caller delivery result TTL независим от billing.
Known rejected before acceptance может иметь qualified fixed request fee: она
учитывается отдельно и не превращается в token usage. Retry/fallback eligibility
определяет trusted adapter, не HTTP status alone.

## Reservation / ledger

held → settled/released only trusted usage/cost/cancel reconciliation. Unknown holds.
Per-currency atomic units exact; late confirmed charge replaces linked estimate.
Refund/correction adds entry, no update old amounts. Overrun preserved and blocks
new paid admission. Aggregates rebuild canonical facts, never change admission.

## Evaluation

queued → running → completed/failed/cancelled. Each candidate/case/repetition unique
request key and worker fence. Unknown attempt yields unknown/incomparable result,
no automatic rerun. Cancel prevents new cases; spent/held values retained.
New run has new config/ID, old results immutable.

## Сохранённый draft и тариф

Local edit сохраняет private UI fields отдельно от server saved version; native
Back/Forward не превращает ввод в committed draft. Оператор может stay/discard.
Proof связывает saved version/config hash и ordered connection/model/generation/
endpoint/adapter snapshots; новый credential/endpoint инвалидирует его. Повторная
connection qualification не восстанавливает profile proof. TTL publication не
заменяет runtime qualification. Rollback требует current qualification старого snapshot.

Tariff draft → scheduled activation → active → superseded; revision неизменна.
Policy CAS и unique activation time сериализуют конкурентов. Новые запросы фиксируют
active tuple; старые и reconciliation не читают сегодняшнюю цену.

## Cancellation и unconfigured witness

scheduled → cancelled только отдельным own cancellation fact до activate_at, с CAS,
scope, actor/reason и original replay result. Current status derived; cancelled time
не переиспользуется и не supersedes predecessor. Draft publication и activation status
различны. Unconfigured source — explicit null FK/unknown basis; настройка поздней цены
не меняет старое admission. Protocol/body mismatch rejected до dispatch intent.
