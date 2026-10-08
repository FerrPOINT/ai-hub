# Интеграция с Base

SDK baseline — полный SHA в [.base-revision](../.base-revision), взят из актуальных
Rust/React продуктов. Это planned dependency; runtime сборки AI Hub пока нет.
[Base compatibility](https://github.com/FerrPOINT/services-base/blob/3a48de8c5696dd20b78c94205d9feb7dbb69c8e0/docs/BASE_COMPATIBILITY.md)
обсуждается в SOURCE_AUDIT; использовать точный подтверждённый source URL из аудита.

Shared/auth/telemetry и @sdlc/ui потребляются без второй копии primitives.
Product владеет grants, API codes/DTO/env, миграциями, routing/accounting.
SDK pin и agent-skills pin независимы; Hub не владеет role skills.

## Первый implementation stage

- Materialize exact Base sibling в own build source set; verify_base_revision
  проверяет clean HEAD и .base-revision перед cargo/pnpm.
- Зарегистрировать service_key=ai-hub, UI/API URLs и explicit capabilities
  health.read/integration.status.read/branding.runtime.read в Admin contract.
  Fixed paths из actual Admin capability catalog: GET /health, /integration/status,
  /branding/contract. Hub реализует безопасные public metadata projections;
  /health — own readiness alias, без provider calls. ui.render декларируется
  только после действующего UI по отдельному public_ui_url.
- Central Auth owner добавляет service scopes ai-hub:read/ai-hub:write по contract;
  до registration PAT не получает invented permissions.
- AppShell/PlatformHeader/PageFrame/themes/service picker через @sdlc/ui;
  branding/runtime catalog consume без blocking product fallback.
- Own PostgreSQL database/role и encrypted vault. Общий instance не shared schema.
- Source inventory не delivery manifest. Workspace runtime добавляется только после
  image/DB/SSO/capability gates; Start не мигрирует/не создаёт missing state.

## Проверки

Exact SDK clean source + feature-isolated domain dependencies, OpenAPI transport,
central session/PAT/logout, UI package consumer/effective themes и no-cross-DB tests.
Перенос нового продукта в Base docs mirror идёт owner sync tool, не ручной правкой
docs/products и не изменением соседних repo в рамках этого baseline.
