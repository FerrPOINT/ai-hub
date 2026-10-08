# Матрица доступа

Это product authorization, не новая система global roles. Central Auth хранит
identity/session/PAT, Hub — own clients/grants. Все active browser users имеют
configuration capabilities по принятой платформенной policy; project/installation
scope дополнительных данных проверяется Hub. Explicit allow, default deny.

| Principal                    | Config read/write                               | Inference                                             | Own metadata                  | Result/content                                 | Credentials                                   |
| ---------------------------- | ----------------------------------------------- | ----------------------------------------------------- | ----------------------------- | ---------------------------------------------- | --------------------------------------------- |
| Anonymous                    | Нет                                             | Нет                                                   | Нет                           | Нет                                            | Нет                                           |
| Active central browser       | В границах installation и policy                | Только через validated scoped launch/playground grant | Granted projects              | Exact own/delegated read_result/artifact grant | Write-only config operation                   |
| PAT ai-hub:read              | Read, не mutation                               | Не автоматическое infer право                         | Granted scope                 | Scope read не read_result                      | Нет                                           |
| PAT ai-hub:write             | Allowed config mutation                         | Не automatic admin inference bypass                   | Не implied cross-project read | Нет без отдельного grant                       | Scoped write-only                             |
| Hub client infer             | Нет                                             | Allowed profiles/project/bounds                       | Только при read_own_usage     | Только read_result + exact owner/TTL           | Нет                                           |
| Revoked/expired principal    | Нет                                             | Нет                                                   | Нет protected new reads       | Нет                                            | Нет                                           |
| Own verification/eval worker | Только frozen internal context                  | Bounded purpose-specific grant                        | Own operation/run             | Approved artifacts                             | Secret adapter context, never public readback |
| Registry public reader       | Only safe /health/integration/branding contract | Нет                                                   | Нет                           | Нет                                            | Нет                                           |

## Sources и проверки

Browser session/PAT current status проверяется Base bridge. Client key hash/expiry/
revocation связывает exact application/project; body user/project label не доверенные
claims. Owner-derived grants проверяются до lookup/dispatch/export и перед next attempt.
UI capability hiding не authorization.403 не logout,401 действует только caller
auth boundary; provider auth failure не отзывает пользователя из платформы.

Readback/cursor/result/exports защищены той же policy, что оригинальный request.
Creator configuration access не implicit permission на transcript любого приложения.
Prototype не содержит real auth; denied scenes демонстрируют доступ, не доказывают security.
Tests: TC-018/019/020/031 и будущие API/SSO/client boundary tests.
