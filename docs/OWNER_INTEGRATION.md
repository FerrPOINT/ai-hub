# Owner integration — фактические границы

## Central Auth

Live readback PDLC3 2026-10-10: `pdlc3-auth-1` содержит OIDC clients
admin-panel/ci-cd/task-tracker/wiki/fleet-control/project-workflow; `ai-hub` отсутствует.
Для живого SSO owner добавляет ai-hub → http://127.0.0.3:8192/sso/callback
и service scopes ai-hub:read/ai-hub:write через штатный installation bundle.
Текущий Auth/common не перезапускался и не изменялся. Прямой POST login или
чужой OIDC client не подменяет acceptance собственного browser path.

## Admin и Tracker

Admin documentation packet `2fb9720d368b0baba1673f89de4d5c0982934c84`
задаёт registry read `/api/v1/namespaces/{id}/context`. Hub использует только
fixed origin и отдельный private reader credential; user PAT не пересылается.
Projection не registry CRUD и не grant. Active binding требует actual owner
identity/readback; собственные SQL fixtures проверяют изоляцию, а не owner acceptance.
Declaration packet находится в deploy/service-declaration.json; ui.render pending.
Документационные owner branches целиком не вливаются.

## OpenRouter

Пользователь разрешил существующий OpenRouter dev account и дешёвые малые checks.
Проверены DNS 212.95.34.10 и ED25519 host fingerprint по DEV_SERVER.md,
наличие ключа подтверждено без вывода значения. Ключ ещё не переносился.
Qualification policy: максимум пять коротких вызовов, общий hard budget 0.10 USD;
валюта/аккаунт/model/rates проверяются до dispatch, paid calls пока 0.
Существующие Octo/Admin credentials/state и runtime не переключаются.
Остальные обязательные provider accounts пока не разрешены/не квалифицированы.

Official [Support](https://openrouter.ai/support) описывает credits как USD.
Это reference для отдельного currency reader contract, не live account witness:
authenticated statement и generation-bound proof ещё нужны. Endpoint
[GET credits](https://openrouter.ai/docs/api/api-reference/credits/get-credits)
требует management key; inference key не расширяется до management ради проверки.
Текущий metadata reader использует только GET key/models и сохраняет currency unknown.
