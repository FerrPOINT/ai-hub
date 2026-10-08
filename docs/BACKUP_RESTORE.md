# Backup и restore

Защищены own DB (revisions/requests/ledger/audit/dedupe), encrypted credential state,
private deployment manifest и отдельный encryption key escrow.
Backup manifest: installation/schema/source IDs, component hashes, created_at UTC,
key ID без значения, consistent snapshot/watermark и restore instructions.
Encryption key не входит в тот же backup archive. Snapshot финансовых данных и
vault generation согласован; aggregate можно rebuild из canonical facts.

## Rehearsal

1. Зафиксировать synthetic nonzero balance, unknown reserve, connection generations,
   receipt dedupe IDs и expired/revoked authorization.
2. Получить own consistent DB/state backup и separate key receipt.
3. Restore только в inventory-proven новый/пустой изолированный own target,
   external_calls=false и без production credentials/network route.
4. Проверить ledger totals per currency, active revision references, dedupe,
   audit chain, unknown states, disabled admissions и no model I/O.
5. Проверить replay/cancel/reconcile synthetic server без повторного dispatch.
6. Собрать exact-source restore receipt, затем удалить только owned rehearsal resources
   по отдельному scoped cleanup; реальные данные/соседи не затрагиваются.

Missing/wrong key, corrupted hash, wrong installation, incomplete snapshot или
nonempty unapproved destination — fail, не новая initialization.
Live restore принят только после rehearsal с real schema и nonempty own state.
RPO/RTO рабочие targets: RPO<=24h scheduled backup, RTO<=1h на qualified local host;
измерить на S7 и не выдавать эти цели за уже достигнутые показатели.
