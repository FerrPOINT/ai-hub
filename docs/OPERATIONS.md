# Операционные сценарии

## Diagnosis

Read-only: served identity, readiness/schema, principal/grant denied reason,
connection generation/proof TTL, request/attempt IDs, ledger unknown/reserved,
quota freshness и projection lag. No env dump/raw transcript/key.
Пустой provider catalog после ошибки не successful catalog; health не inference proof.

## Operator actions

- Disable connection/model/client: audited policy, blocks new attempts, existing
  dispatched calls отменяются best-effort по adapter receipt без fake cost release.
- Change profile: draft/CAS/proof/publication, not active run rewrite.
- Unknown attempt: получить provider-owned status/receipt; если нет evidence,
  сохранять unknown/reserve и controlled no-cost retry запрещён.
- Reprice/reconcile: append-only operation с receipt/source ID; не SQL UPDATE суммы.
- Rebuild statistics: scoped canonical watermark; не вызов моделей.
- Rotate own key/client key: own operation, backup compatible old decryption
  window и revoke readback; plaintext не логи.
- Restore: [BACKUP_RESTORE](BACKUP_RESTORE.md); own target и external_calls=false.

## Notifications

Budget warnings 80/95%, hard deny, persistent unknown reserve, quota stale/provider
availability и projection lag — own in-app notifications в v1.
Delivery outbox unique scope/period/threshold/event key; alert ACK не меняет finances.
External email/Telegram/webhooks — последующий scope; никаких сообщений без owner
настройки/авторизации. Poll recovery bounded, no perpetual whole-provider retest.

## Retention и incident

Deletion/archival сверяется с independent replay/financial TTL и restore policy.
При подозрении на key leak revoke affected connection/client и сохранить safe
evidence, не переписывать history. При cost overrun block new paid admission,
показать real fact/held reserve и разблокировать только audited policy/reconciliation.
