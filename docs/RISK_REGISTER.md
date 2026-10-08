# Риски и зависимости

| ID   | Риск                                         | Контроль / checkpoint / owner                                            |
| ---- | -------------------------------------------- | ------------------------------------------------------------------------ |
| R-01 | Metadata выдаётся за actual provider access  | Proof exact model/account/generation + live S4; AI Hub                   |
| R-02 | Unknown paid I/O повторяется                 | Intent/reserve + readback/reconciliation S3; AI Hub                      |
| R-03 | Два владельца AI config после выделения      | Owner-coordinated ADMIN_HANDOFF S7; Hub/Admin                            |
| R-04 | Subscription price считается token cash fee  | Separate period charge/allocation, receipt basis S4/S5; AI Hub           |
| R-05 | Cached/reasoning tokens двойной счёт         | Adapter categories/evidence и decimal sample S3; AI Hub                  |
| R-06 | Provider alias silently changes model        | Actual model confidence + pinned qualification S4/S6; AI Hub             |
| R-07 | Несовместимые Responses/tools conversion     | Exact wire/semantic fixtures и live account tests S3/S4; AI Hub          |
| R-08 | Existing SDK/toolchain drift                 | Exact .base-revision/locked source/compatibility gate S1; Hub/Base       |
| R-09 | Нельзя получить реальные quota/charge facts  | Unknown/freshness и declared cost limits, not fabricated 0 S4/S5; AI Hub |
| R-10 | Budget bypass race/multiple scopes           | Atomic sorted locks and unknown holds S3; AI Hub                         |
| R-11 | Потеря ledger/vault/key после rollback       | Nonempty isolated restore rehearsal и schema compatibility S7; operator  |
| R-12 | Нельзя readback target или in-app browser    | Stage acceptance remains unverified; recover approved surface S7         |
| R-13 | Экспорт/telemetry раскрывает private content | Allowlist grants/redaction/retention tests S5; AI Hub                    |
| R-14 | Statistics projection отстаёт/ошибочна       | Watermark/knownness, canonical sum, exact rebuild S5; AI Hub             |
| R-15 | Порты/БД соседей используются по шаблону     | Inventory and own resources before S1 runtime; operator                  |

## Не блокирует старт разработки

Actual provider credentials/price/account policy, chosen live URL/ports, consumer
cutover и capacity receipts нужны соответствующим future checkpoints.
S1–S3 выполняются с synthetic data и controlled upstream.
Неподтверждённые native capability/financial guarantees не принимаются «по аналогии».
Действительных неразрешённых design blockers после baseline audit нет;
future access/runtime gates открыты и не объявляются PASS.
