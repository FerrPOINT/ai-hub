# Безопасность

AI Hub находится на стадии документирования. Работающий security boundary,
provider verification и production acceptance отсутствуют. Размещать реальные
ключи/данные и открывать публичный inference до соответствующих gates нельзя.

Целевые правила: [Security](docs/SECURITY.md), [Threat model](docs/THREAT_MODEL.md),
[доступ и retention](docs/adr/0005-access-and-retention.md).
Credentials вводятся write-only, шифруются own key, не возвращаются из API.
Prompt/response не входят в обычные logs/analytics; opt-in evaluation artifacts имеют
отдельный доступ и TTL. API metadata не подтверждает право внешнего аккаунта.

Не публиковать exploit, токены или реальные запросы в публичном issue.
Передать владельцу FerrPOINT приватно описание границы, минимальный synthetic
reproducer и affected revision. Если приватный GitHub Security advisory доступен,
использовать его; иначе согласовать приватный канал с владельцем.
