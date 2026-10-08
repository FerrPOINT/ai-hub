# Модель угроз

| Угроза                                     | Boundary / контроль                                                  | Проверка |
| ------------------------------------------ | -------------------------------------------------------------------- | -------- |
| Подмена client/project/user                | Verified principal + immutable context; metadata не grants           | TC-018   |
| Доступ к чужим requests/expenses/artifacts | Server scoped queries, readback/export same policy                   | TC-018   |
| Утечка provider authorization              | Own vault/write-only/redaction и browser memory policy               | TC-002   |
| SSRF через base URL/model redirect         | Fixed validated endpoint allowlist, DNS/IP, redirects off            | TC-020   |
| Replay/double billing                      | Unique request/attempt/evidence IDs, unknown reserve, reconciliation | TC-006   |
| Concurrent budget overspend                | Atomic sorted row locks/reserve before I/O                           | TC-012   |
| Native hidden retry/side call              | Adapter qualification/disabled tools/attempt accounting              | TC-007   |
| Prompt-injected control/tool action        | Split scopes; Hub не tool executor                                   | TC-020   |
| Stream output смешан от разных моделей     | No fallback after content/tool fragment                              | TC-005   |
| Alias скрывает upstream downgrade          | Exact receipt, unverified actual model visible                       | TC-009   |
| Poisoning usage/price confidence           | Trusted ingestion, immutable price snapshots, audit corrections      | TC-010   |
| Duplicate invoice/subscription accounting  | Unique external source ID; settlement replaces estimate              | TC-011   |
| Formula injection в CSV                    | Escape cells, authorized snapshot                                    | TC-015   |
| Loss of vault/key/ledger после restore     | Separate key escrow, own restore drill, no external calls            | TC-022   |
| High-cardinality metrics/data exhaustion   | Bounded requests, filters, export и metric labels                    | TC-016   |
| Revoked subscription/central session       | Current generation/session recheck, no credential fallback           | TC-019   |

## Residual risks

External provider billing/alias semantics могут быть неполными; confidence остаётся
unknown/estimated до evidence. Provider account/API policy drift требует повторной
qualification. Сетевое разделение и зашифрованные backups проверяются на target,
не выводятся из source. Публичный multi-tenant hosting вне v1.
