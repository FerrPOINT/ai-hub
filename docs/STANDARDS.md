# Стандарты реализации

## Backend

Rust edition 2024, api/application/domain/infrastructure ports; async external I/O,
lifecycle pooled clients, typed error taxonomy. No runtime DDL, handler SQL,
copied routing/accounting policy или global mutable config. Cancellation/deadlines
через accepted framework primitives; no retry by default.
Config loader принадлежит Hub; shared Base transport/error helpers не владеют
product status codes, env defaults, grants или decimals.
New persistent field → canonical migration + compatibility test.

## API

OpenAPI 3.1 из Rust handlers после draft transition. UUID IDs, RFC3339 timestamps,
decimal strings, consistent null/absence/default. Immutable mutation receipts,
If-Match для config, Idempotency-Key для external side effects.
Parameterized SQL, allowlisted filters/sorts/columns, opaque cursor bound to
subject/query/snapshot; default limit50 max100. Errors без internal private payload.

## Frontend

Base shell/primitives/themes, typed thin clients/React Query/query keys,
forms preserve draft and match server validation. Product owns navigation/filters.
No duplicate UI kit, business sums/auth rules в browser или credentials storage.
Code comments explain WHY; docs Russian, code comments English.
Formatting/lint/test commands создаются вместе с actual manifests, не как заглушки.

## Review

Assess product/owner/state lifetime, database schema, contracts, price/usage knownness,
idempotency/unknown/routing, context propagation, UI state/geometry, diff hygiene,
regression semantics и exact evidence. Green static checks не доказывают live outcome.
