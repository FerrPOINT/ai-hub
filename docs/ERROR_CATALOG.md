# Ошибки и действия пользователя

Канонический product envelope/status — API; поле message безопасно, request_id UUID.
Error/unknown не пустой success, raw provider payload/credentials не раскрываются.

| Code / HTTP                    | Где возникает                      | Что показывает UI                              | Retry / данные / деньги           |
| ------------------------------ | ---------------------------------- | ---------------------------------------------- | --------------------------------- |
| invalid_payload /400           | DTO/field/range parse              | Field errors + preserved draft                 | Исправить ввод, no I/O            |
| unsupported_parameter /400     | Unqualified protocol field         | Неподдерживаемая возможность                   | No silent drop/downgrade          |
| unauthenticated /401           | Caller auth/session/key            | SSO или key auth required                      | No credential fallback            |
| forbidden /403                 | Product scope/grant                | «Нет доступа»                                  | Не logout, не model retry         |
| not_found /404                 | Exact resource lookup              | «Запись не найдена»                            | No first-record fallback/CRUD     |
| stale_revision /412            | CAS config/draft                   | Версия изменилась, draft retained              | Read current; explicit reconcile  |
| idempotency_conflict /409      | Same key/different binding         | Конфликт запроса                               | New key только для new intent     |
| request_in_progress /409       | Duplicate active request           | Ход/readback original ID                       | No repeat provider                |
| request_unknown /409           | Duplicate ambiguous dispatch       | Неизвестный исход, reserve held                | Trusted status/receipt only       |
| replay_expired /409            | Content TTL passed                 | Buffer expired, metadata retained              | No automatic new inference        |
| stream_replay_unavailable /409 | Repeated stream                    | Result readback if permitted                   | No fabricated second stream       |
| capability_mismatch /422       | Params/tools/JSON/proof            | Требуется compatible profile                   | Deny before I/O                   |
| context_limit /422             | Qualified tokenizer/bounds         | Limits mismatch                                | No silent truncate/compact        |
| usage_invalid /422             | Impossible provider categories     | Недостоверный usage                            | Don't clamp false values          |
| rate_limited /429              | Own bounded limiter                | Retry-After/count scope                        | New intent only under safe policy |
| budget_exhausted /429          | Atomic reserve                     | Scope/remaining/reset/held                     | Denied before I/O                 |
| provider_failure /502          | Known upstream typed failure       | Safe cause and own request trace               | No own-user logout                |
| provider_unavailable /503      | No eligible qualified deployments  | Статус подключения                             | No bypass Hub/paid retries        |
| authorization_unknown /503     | Own connection authorization state | Нужен status/relogin owner                     | Unknown auth not a new login loop |
| accounting_unavailable /503    | Own ledger DB/admission            | Вызов не отправлен / outcome unknown after I/O | Keep intent/reserve               |

SSE после первого content/tool fragment: safe protocol failure/close, original
request ID и unknown/cancel status; no false finish_reason=stop/completed.
Cancellation подтверждается adapter; cost unknown может остаться после terminal cancel.
Notifications дают user action, не автоматическую credential/budget policy смену.
