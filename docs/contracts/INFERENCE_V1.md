# Inference contract v1

Статус design-only; источник typed shapes — [OpenAPI](openapi.v1.json).
Контракт действует между consumer и AI Hub; provider adapter имеет собственный
qualification receipt и не выдаётся за универсальный SDK.

## Client flow

Client имеет scoped Hub key, project binding и allowlist virtual models.
Выбирает virtual slug или explicit allowed published revision header,
задаёт caller input/tools и allowed bounded parameters.
Hub admission создаёт request context и durable intent/reserve до I/O.
Ответ выдаёт request ID/revision; own request readback возвращает trace/usage/money.
Client инструменты выполняет сам и передаёт results новым request с новым key;
Hub не начинает hidden agent loop.

## Streaming

Chat SSE data — chat.completion.chunk с stable response id/model, choice index0,
delta content либо tool_calls[index/id/function.name/arguments], finish_reason
stop/length/tool_calls/content_filter. Chunk usage только в отдельном terminal chunk
если counts complete, затем data: [DONE]. Fragment arguments не считаются complete
tool args до terminal tool_calls; сохранить ordering и точные bytes.

Responses SSE event type — response.created, response.output_text.delta,
response.function_call_arguments.delta, response.function_call_arguments.done,
response.completed/failed/incomplete и errors. Events sequence monotonic;
function call IDs стабильны. Unknown/cancelled interruption не completed.
Final output требует finished message/function shape и known usage либо null.
No fallback после любого content/tool fragment; heartbeat не content.
Proxy/loadbalancer buffering выключено; backpressure bounded и client disconnect
триггерит cancel intent, а не доказанное отсутствие bill.

## Idempotency и recovery

Scope=(installation,client,key), binding включает principal/project/profile revision,
semantic payload и override headers. Key созданный самим Hub не dedupe будущий
новый HTTP attempt клиента; надёжный клиент присылает UUID key заранее.
Request ID возвращается в headers до streaming body.
Idempotent replay после ambiguous outcome даёт existing status/metadata,
не повтор provider. Full stored safe reply до <=24h; expired payload → replay_expired.
Nonstream terminal reply replay возвращает original reply. Stream replay и
in-progress/unknown duplicate не создают SSE заново: 409 с original request ID.
GET /api/v1/requests/{request_id}/result возвращает terminal typed reply по own
read_result grant до TTL; финансовая trace read permission не даёт это право.
Network failure до получения ID: повтор с тем же key и request readback recovery,
не новый key. Control GET requests filters by own key/request only authorized.

После restart intended/dispatched-but-unconfirmed → unknown, held reservation
не освобождается. Adapter reconcile использует status/receipt queries,
не новый inference. Cancel до dispatch terminal без fee; после dispatch
cancel_requested до подтверждения; unknown billing всё ещё reserved.

## Limits/capabilities

Function tool JSON Schema только local refs/bounded depth/size, no remote resolver.
Tool continuation полные пары; malformed/incomplete history rejected до I/O.
Client tools не native shell/browser tools. JSON output schema при unsupported
candidate не silently relaxed. Context/output bounds checked до dispatch по
qualified token accounting, estimates отмечены и safety bound обязателен.
No automatic truncation, summarization/compaction или transformation into new model.
Если tokenizer/upper-bound не qualified, budget-guaranteed admission fail closed.

Probe и evaluation calls имеют own kind/frozen snapshot, grant и financial accounting;
не обходят limits, даже если не обычные user requests.
