# Уточнение исходов и расходов

## Trusted ingestion

Provider adapter/approved receipt importer produces bounded witness: installation,
connection/generation, external provider request/event ID, currency/category amounts,
occurred_at, trust source и content digest. Body metadata/price settings не proof.
trusted_receipt_ref — opaque own store reference, не arbitrary URL для server fetch.
Imported receipt validated against exact invocation/account, never other tenant.

## Алгоритм

1. Read canonical request/attempt and scope, not projection.
2. Validate witness ownership/generation/source ID and payload consistency.
3. Lock affected ledger/budget rows in stable order.
4. Dedupe external source/event; same witness readback has no extra charge.
5. Append usage/charge/correction + receipt links; replace effective estimate in view,
   retain original record. Release only proven excess reservation.
6. Commit audit/state/settlement atomically. After commit refresh projections by
   idempotent canonical watermark, not second provider call.

## Ambiguity

No provider acceptance/status evidence: unknown remains and reserve held.
Native transport or gateway hiding attempts/usage cannot qualify budget guarantee.
Known terminal cancelled can still have billing unknown.
Conflicting receipt, impossible category counts, mismatched currency or out-of-range
numeric amount: quarantine safe witness/reason, block affected paid admissions,
do not silently clamp/delete valid records. Operator investigates source, not fake zero.

## Subscription и period

One payment source ID once; cash occurred_at != service period.
Statement settlement can confirm estimate and late correction; allocation is
separate policy/projection, never a second cash debit. Monthly installation budget
includes all applicable month fees, while a7-day view may legitimately exclude them.
Scope attribution from canonical trusted client/project/connection policy, not input label.
Tests include duplicate receipt, refund, late fee, unknown cancel, two replicas,
currency mismatch and provider unable to read status.
