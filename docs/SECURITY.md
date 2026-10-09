# Доступ и защита данных

## Authentication и authorization

Browser control API использует central SSO Authorization Code + PKCE и Base bridge:
verified issuer/audience/ES256/JWKS, session active state и PAT revocation/scopes.
Raw JWT signature без session-status проверки не достаточна при central policy.
Hub не создаёт passwords/users/refresh identity sessions.
JWT/session unavailable — protected admission denied; cached public key не
обходит необходимость revoke/session validation.

Все active central browser users получают platform configuration access по
текущему Base policy; локальных глобальных ролей viewer/operator/admin не вводим.
PAT read/write требует exact ai-hub:read/ai-hub:write; write-only grant не обязан
давать право просмотра transcripts. Статистика/тестовые artifacts дополнительно
ограничены installation/project grants. Самостоятельная registration Hub service
scopes в Central Auth — integration stage, до этого PAT denied.

Service inference client key: random >=256 bits, hashed at rest, shown once,
scopes infer/read_own_usage + allowlist profiles/projects, expiry/revocation.
Human SSO control token не становится provider key. Machine request не получает
admin authority. Delegated user binding требует trusted signed service identity,
а не caller поле user. UI verification/evaluation launch использует scoped short-lived own grant,
не хранит machine secrets в browser storage.

## Credentials

Own encrypted credential versions, key вне DB/backup bundle, connection generation,
write-only endpoints. AES-256-GCM, unique random nonce, AAD installation/connection/
generation, cleared buffers и private permissions. Использовать audited crypto
library; key rotation не переписывает audit/provider facts.
No key in URL, command args, response, logs/traces, browser storage, export.
Login verification URL/code — sensitive transient UI, no analytics/logging.

## Egress и payload

Endpoints allowlisted by deployment; HTTPS/TLS validated, redirects off, DNS/IP
revalidation и no arbitrary proxy env. Private Ollama разрешён только для конкретного
approved deployment destination; generic private-network access запрещён.
Body limit 2 MiB, schema depth/size bounded, response/tools output caps, timeouts.
Hub передаёт caller tools модели, но сам их не исполняет. Prompt content не даёт
прав на key/config/export/tool execution. Required capability mismatch до dispatch.

## Retention

Metadata/usage facts 90 дней online, ledger/audit 365 дней default; operator may
extend under explicit policy. Detailed plaintext prompts/responses default off.
Для output delivery/idempotent replay — own encrypted response buffer <=24h,
request replay key retention=30 дней; after payload TTL возвращается replay_expired,
не повторный inference. Это не transcript analytics.
Own result recovery endpoint требует отдельного read_result grant и exact request
ownership; доступ к статистике/финансовой trace не открывает result content.
Evaluation artifacts opt-in own dataset owner, default TTL=7 дней.
Удаление content не удаляет billing dedupe keys/settlements; after retention
ledger archival сохраняет audit digest и restore-compatible dedupe.
TTL worker не вызывает model/provider endpoints. Privacy export scope enforcement
на сервере; CSV formula injection neutralized.

## Актуализация 2026-10-09

NamespaceRef, verified project projection и machine bindings описаны в [NAMESPACE_V1](contracts/NAMESPACE_V1.md). Непривязанные записи не получают права любого Namespace. Notification ACK actor-local; credentials не переносятся обычным export.
