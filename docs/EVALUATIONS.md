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
