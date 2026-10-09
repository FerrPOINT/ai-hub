# Источники baseline

Проверка: 2026-10-08. Current checkout и source hashes — authoritative для design,
не историческая память. [source-audit.json](source-audit.json) фиксирует exact
source revisions и hashes выбранных файлов; license/notice copied без изменения.

## Репозитории

- [Base standards](https://github.com/FerrPOINT/services-base/blob/3a48de8c5696dd20b78c94205d9feb7dbb69c8e0/docs/REPO_STANDARDS.md)
  и README/documentation/CI/UI/auth/compatibility contracts — root matrix и owners.
- [Fleet docs map](https://github.com/FerrPOINT/fleet-control/blob/c8093aace07e54436893c5f7e35df1f968690266/docs/README.md)
  — функциональные категории, traceability и проверки.
- [Admin AI contract](https://github.com/FerrPOINT/admin-panel/blob/237a697f4e3eef9ff95e8cf6082932e5a182e6ae/docs/contracts/AI_PROFILE_V1.md)
  и ADR-0018/ai-runtime README — существующий source, partial inference/readiness.
  backend/domain/src/lib.rs подтверждает fixed capability paths /health,
  /integration/status и /branding/contract; Hub draft содержит эти endpoints.
- WARTZ koara1/litellm/README.md и gateway decision — stable aliases/ordered fallback
  Octo reference. Не проводился live SSH readback; stored provider names/quotas
  не объявлены текущими и не используются как Hub defaults.

## Официальные API

- [OpenAI Responses SSE](https://developers.openai.com/api/reference/resources/responses/streaming-events):
  response stream содержит typed events и completion; Hub проектирует bounded subset.
- [Ollama compatibility](https://docs.ollama.com/api/openai-compatibility):
  существуют Chat/Responses совместимые пути, но stateless restrictions;
  это основание explicit reject previous_response_id/conversation v1.
- [Z.AI completion](https://docs.z.ai/api-reference/llm/chat-completion):
  usage различает input/output и cached input; adapter обязан нормализовать без
  двойного счёта. Physical models/access и billing endpoint проверяются отдельно.
- [LiteLLM routing](https://docs.litellm.ai/docs/routing) /
  [spend tracking](https://docs.litellm.ai/docs/proxy/cost_tracking):
  reference routing/cost patterns; calculated cost требует сверки с provider bill.
  Политика unknown/atomic budgets AI Hub определена своим ADR, не скопирована.

## Limits доказательств

Официальные страницы подтверждают протоколы, не availability конкретного аккаунта,
prices/quotas/subscription rights или real installed provider behavior.
Исходная документация не copied API implementation. Current fleet toolchains имеют
различия; выбран own baseline без автоматического upgrade.
Нет реальных credentials, provider calls, runtime screenshots и application release
в documentation preparation.

## Актуализация 2026-10-09

Base reference `81decf7d9edd2c4218d8625a96e2e25c0617e9f1`, Admin reference `7c4240a2154478b4866661cbc45dbd06d3e855e2`. Source hashes обновлены по committed blobs; исходный SDK pin независим и сохранён. Это provenance документов, не live acceptance.

## Актуализация 2026-10-09

Текущая актуализация перепроверяет локальные committed sources. Внешние provider references сохраняют прежнюю дату snapshot; их live requalification и изменившиеся условия проверяются на S4, не объявлены свежими source-only audit.
