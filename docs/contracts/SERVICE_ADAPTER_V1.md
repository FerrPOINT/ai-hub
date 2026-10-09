# Подписанный service-adapter AI Hub v1

Статус: Target approved для нового Hub transport. Текущие Fleet/Forge/Admin
runtime и strict SDLC v1 не изменяются. Это полный target wire, не live receipt.
Источник Base ExecutionContextV2: `81decf7d9edd2c4218d8625a96e2e25c0617e9f1`,
`crates/sdlc-shared/src/resource_context.rs`. Формат legacy Ed25519 envelope:
Admin `7c4240a2154478b4866661cbc45dbd06d3e855e2`, `backend/ai-runtime/src/execution_grant.rs`.
Legacy audience `sdlc2-ai-runtime`, ordinal profile_revision и workspace не
переносятся автоматически в Hub. Потребуются новый grant и UUID revision mapping.

## Вызов

`POST /internal/v1/service-inference`, Content-Type application/json:

- Authorization: Bearer scoped Hub client key.
- Idempotency-Key: UUID, равный signed request_id.
- X-AIHub-Delegation: base64url UTF-8 ServiceGrantEnvelope.
- Body: ServiceInferenceInput — protocol chat/responses, неизменённый Base
  ExecutionContextV2 и обычный ChatInput/ResponsesInput.

Service identity устанавливает проверенная подпись разрешённого issuer; Hub key
устанавливает клиент, Namespace, profile allowlist и финансовые ограничения.
Обе границы обязательны. Human PAT не является ни provider key, ни service grant.
Обычный SDK продолжает использовать /v1 без Task/run и без этого envelope.

## Подпись и доверие

Envelope имеет schema_version=1, key_id, исходный JSON payload string и
signature_hex (128 lowercase hex). Подписываются байты:
`AIHUB-SERVICE-INFERENCE-V1` + NUL + исходные UTF-8 bytes payload.
Verifier не пересериализует payload до проверки подписи. Payload <=32 KiB,
header <=64 KiB; unknown/duplicate fields и неподдерживаемые версии отвергаются.
Ключи Ed25519, issuer instance, machine subject и audience installation входят в
deployment trust allowlist. Caller не передаёт public key или новое доверие.

ServiceInferenceClaims связывает grant_id, client_id, request_id,
context_operation_id, execution_id, profile_revision_id UUID, NamespaceRef,
fencing_token decimal u64 string, issued/expires/lease_expires, currency и
max_provider_cost. `request_body_sha256` — SHA256 точных HTTP body bytes.
Signer сериализует тело один раз и сохраняет те же bytes для retry. Прокси не
переформатирует JSON. Подпись поэтому защищает и invocation, и ExecutionContextV2.
context_operation_id — исходная операция привязки Base context, не новый request ID.
execution_id — Fleet execution; Hub не создаёт его и не ведёт Task lifecycle.

## Admission и fencing

До записи dispatch intent проверяются:

1. Signature/trusted key/issuer/subject/audience и raw body digest.
2. Client key active, exact signed client_id; nonnull client Namespace равен
   claim Namespace и body.execution_context.namespace по двум UUID.
3. Idempotency-Key=request_id; body context operation совпадает с claim.
   Task/Repository owner references проверены через bounded fixed-owner readers.
4. Signed profile_revision_id принадлежит invocation.model и allowlist клиента;
   текущая квалификация account/model/generation остаётся обязательной.
5. Grant/lease expiry, revoke tombstone и монотонный fence issuer/execution.
   Expired/revoked/lower fence не получают новый dispatch. Clock skew <=30 s
   разрешён только issued_at, не продлевает expiry.
6. Signed cost cap сужает trusted client/budget policy. Grant не повышает
   monetary authority; null cap разрешён только explicit cost_unknown_allowed.

Verified context фиксируется один раз. Перед следующим attempt повторяются
expiry/revocation/fence/qualification checks; Task role и tool grants остаются
у Fleet. Исторический запрос сохраняет исходные IDs, price/tariff и profile snapshot.

## Отзыв и recovery

`POST /internal/v1/delegations/revoke` имеет Idempotency-Key и подписанный
X-AIHub-Delegation с ServiceRevocationClaims (action=revoke, issuer/audience/subject,
issued_at, raw body hash). Body — grant_id. Только его trusted issuer может
отозвать grant. Для ещё неизвестного grant сохраняется tombstone; поздний invoke
не обходит отзыв. Отзыв запрещает новые attempts и вызывает best-effort cancel
уже dispatched requests; uncertain reserve не освобождается без receipt.

Missing response → повтор точных body/key или GET original request по Hub key.
Original request_id/profile revision возвращаются до SSE content. Duplicate
unknown/in-progress не создаёт новый stream. Expired raw reply не повторяет provider.
Issuer cancellation использует own outbox/retry для revoke; отключённый Hub не
разрешает прямой обход к legacy runtime. Legacy rollback — отдельное reconciliation.

## Приёмка потребителей

Fleet adapter маппит свою service identity/execution и сохранённый V2 context;
Forge либо использует accepted Fleet execution delegation, либо именованный
trusted Forge issuer с теми же gates. RepositoryRef не даёт inference право.
Новый transport не включает существующие foundation-only starts: runtime_ready
и dispatch_allowed остаются false до actual owner acceptance.
S7 проверяет packed SDK, signatures/body mutation, UUID/ordinal mismatch,
Namespace/issuer/client mismatch, expired lease/revocation/fence и restart/readback.
Примеры и offline cryptographic vectors не заменяют live consumer qualification.
