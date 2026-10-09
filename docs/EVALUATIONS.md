# Разработка и сравнение моделей

## Dataset и run

Dataset хранит immutable version, synthetic inputs, caller tool schemas,
expected assertions, case labels и hashes. Не импортировать рабочие conversations
без отдельного разрешения, retention и доступа.
Run фиксирует dataset version, profile revisions, exact deployments/generations,
parameters, adapter versions, max concurrency, seed support, scorer version,
currency/budget cap и environment receipt.

Каждый вызов проходит обычный inference admission/ledger с evaluation_run_id.
Private runner grant не позволяет читать чужие проекты. Mode pinned_test:
один exact deployment; fallback/retry/cache off. Недоказанная provider model
substitution отмечает case incomparable, не successful pinned result.
Наличие seed не обещает детерминизм provider.

## Scorers и результаты

v1: exact match, normalized text comparison, JSON Schema validity, tool name/args,
user-reviewed score с actor/time. Auto LLM judge в v1 выключен: требует отдельного
профиля, бюджетируемых requests и disclosure.
Объективность результата: одинаковые dataset/version/config, repetitions и
raw sample counts, confidence limitations. Один удачный ответ не рейтинг модели.

Результаты: pass/fail/invalid/incomparable/skipped, actual model, tokens,
TTFT/latency, expenses/confidence, error/fallback count и artifacts hash.
Cost/quality/time сравниваются независимо. Missing usage не zero-cost win.
Partial run и cancelled run сохраняют потраченные attempts и unknown reserves.

## Лимиты и lifecycle

Default concurrency=1, cap=8; max cases=1000, repetitions<=10, candidates<=10.
Launch требует explicit capped budget либо declared cost-unknown/token caps.
States queued/running/completed/failed/cancelled; worker leases + fencing,
одна attempt на case/candidate/repetition/key. Restart читает request status,
не повторяет ambiguous external I/O.
Cancel закрывает новые admissions, текущие attempts сверяет adapter; partial cost
сохраняется. Export и content artifacts только владельцу/granted project.
TTL artifacts=7 дней default; run metadata/financial facts имеют независимый retention.

Manual scores: case_id/profile_revision_id/repetition из frozen config; `expected_version=0` создаёт absent score, update сравнивает CAS, 412 сохраняет ввод. Score 0..1 с шагом .001, reason, authenticated actor/time, append-only version history; ACK/score не меняет financial facts. [SERVICE_ADAPTER_V1](contracts/SERVICE_ADAPTER_V1.md) описывает отдельный service inference.

## Scorer v1: точные правила

`scorer-v1` выбирает проверку по assertion_kind. `manual-v1` оставляет каждый
случай pending review независимо от автоматической assertion; успех не выдумывается.
Неизвестный suite и duplicate candidate revision IDs отвергаются до запуска.

| Assertion       | Правило expected/output                                                                                                                                                                                                                           |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| exact_match     | Точное равенство decoded UTF-8 strings, включая whitespace/case.                                                                                                                                                                                  |
| normalized_text | NFC, CRLF/CR → LF, trim и collapse только ASCII whitespace `[ \t\n\r\f\v]` в один space. Case/punctuation/NBSP сохраняются.                                                                                                                       |
| json_schema     | expected — JSON Schema 2020-12 object/boolean; output — один valid JSON value, без удаления code fences. Proven validator, local refs only, no remote resolver.                                                                                   |
| tool_args       | expected JSON `{name,arguments}`; name совпадает с declared function, arguments — object. Ровно один complete returned tool call с этим name; JSON keys unordered, arrays ordered, numbers exact decimal semantic equality. Tools не исполняются. |
| manual          | Нужна attributed ManualScore с matching case/candidate/repetition; expected — пояснение для человека, не автоматически начисленный балл.                                                                                                          |

Некорректная assertion/tool schema отвергается при записи dataset. При inference
обрыв/incomplete tool arguments — invalid/incomparable, не pass и не повтор I/O.
JSON/schema depth ≤32, expected ≤32768 chars; numeric lexeme ≤128 chars/exponent
absolute ≤1000. После bounded parse сравнение JSON не использует f64. Scorer timeout
даёт invalid с reason; версия validator/Unicode normalization и конфигурационный
digest входят в frozen environment receipt. Одна revision не меняет алгоритм.
Fixtures: `"  A\r\n B "` normalizes to `"A B"`; `"a b"` does not match `"A B"`.
`{name:verify,arguments:{}}` без такого declared function — invalid dataset.
