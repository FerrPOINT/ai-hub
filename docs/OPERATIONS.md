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

## Уведомления v1

listNotifications возвращает safe authorized projection, ACK сохраняет verified actor и
original event/idempotency identity. Delivery ACK outbox не пользовательское прочтение.
Read ai-hub:read, actor ACK ai-hub:write либо accepted central access с scoped grant.
Polling 30s visible page; hidden/logout останавливает чтение. Ошибка не очищает данные.
ACK не settlement, release reserve, re-enable connection или изменение финансов.

## Runbook до появления runtime-команд

| Симптом                        | Диагностика                                           | Действие                                                      | Подтверждение                                                         |
| ------------------------------ | ----------------------------------------------------- | ------------------------------------------------------------- | --------------------------------------------------------------------- |
| Auth недоступен                | issuer/session validation health и request ID         | Закрыть новые protected admissions, сохранить UI draft        | Verified session восстановлена; denied requests не dispatch           |
| Request завис                  | original ID, attempt intent, accepted/unknown, lease  | Readback/reconcile, без нового invocation                     | Тот же ID и receipt; no duplicate attempt                             |
| Неизвестная стоимость          | trusted usage/price/receipt и reserve                 | Сохранить unknown, получить owner receipt                     | Settlement append-only, резерв освобождён только по evidence          |
| Статистика отстаёт             | watermark/lag, canonical ledger snapshot              | Scoped rebuild без model calls                                | Суммы и unknown counts совпадают с canonical                          |
| Graceful shutdown              | active attempts/streams и dispatch intents            | Запретить admission, bounded drain 120s; unresolved → unknown | Durable terminal/unknown записан, reserve сохранён; restart no resend |
| Namespace unavailable/archived | Exact registry/Namespace pair и last verified binding | Закрыть scoped writes/admission, сохранить authorised history | Owner readback подтверждает active generation                         |

Исполняемые commands добавляются после настоящего S1/S3 runtime. Не подставлять
mock responses или healthy container вместо проверки перечисленных границ.
