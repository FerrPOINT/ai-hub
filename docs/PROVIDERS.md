# Provider adapters

## Набор v1 и qualification

| Adapter           | Назначение                                                  | Обязательная граница                                                           |
| ----------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------ |
| openai_compatible | API-key accounts, generic compatible deployments/OpenRouter | Explicit endpoint allowlist, capability evidence                               |
| ollama            | Local/cloud Ollama                                          | Разные accounts/auth/capabilities; local endpoint только deployment allowlist  |
| zai               | Z.AI API/Coding plan                                        | Connection привязана к точному billing endpoint; никакого General API fallback |
| chatgpt_managed   | Собственная подписка через managed native transport         | Own authorization, native schema/version qualification                         |

Это план, не текущая поддержка. Providers получают отдельные stage receipts.
Название модели и цены не hardcoded defaults; models из account catalog не
публикуются автоматически. Возможность покупки/входа и внешние правила provider
проверяются при integration; документация не обещает конкретную подписку/модель.

## Interface

Adapter port: catalog, verify, dispatch, cancel, reconcile и normalize_usage.
Результат содержит accepted=not_accepted|accepted|unknown, terminal state,
actual model/verifiability, usage provenance, external IDs, typed safe error.
Вход фиксирует attempt ID, frozen revision, generation, native parameters,
required capability и deadline. Endpoint/headers берутся из validated connection,
caller не задаёт arbitrary base URL или proxy target.

Qualification включает nonstream/stream, exact function args, tool continuation,
JSON schema, cancel, deadline, context/output bounds, token usage и acceptance.
Capabilities имеют supported|unsupported|unverified, proof revision и TTL.
Unknown optional params отклоняются; converter не выкидывает данные молча.

## Subscription boundary

Существующая Admin managed login — source reference. Global Codex credentials,
Octo state и чужие cookies не импортируются автоматически.
Login/code/refresh принадлежат Hub connection с own generation/operation ledger.
Catalog не подтверждает доступ inference или physical context bound.
Managed transport не запускает local tools, shell, browsing либо hidden auxiliary
calls; side requests должны быть qualified, bounded и учтены.
Adapter с недоказанными native retries не допускается к budget-guaranteed requests.

## Octo compatibility

Возможный external gateway adapter явно объявляет границу наблюдения.
Если Octo скрывает provider/cost/attempts, Hub показывает actual/expense unknown;
это не qualified pinned comparison. Начальный production перевод приложений
с Octo выполняется по отдельному плану; настройки/сеть Octo здесь не меняются.
Для synthetic test provider используются тестовые модели model-a/model-b.

## Источники

[Источник и ограничения документации](SOURCE_AUDIT.md) фиксирует дату и URLs.
Ollama compatible API имеет subset/state limitations; Z.AI usage содержит cached
input; OpenAI Responses streaming — typed SSE. Mapping сверяется с exact provider
и версиями adapter, не с универсальным обещанием «полная совместимость».

## Pricing sources

OpenRouter explicit preset поддерживает qualified automatic catalog pricing и terminal usage.cost / generation readback. Ollama Online/ChatGPT по подписке получают manual per-million rates как allocation; это не API list price или confirmed cash. [PROJECT_TARIFFS](PROJECT_TARIFFS.md) хранит official source/date и unknown rules.

Billing currency квалифицируется отдельно от per-token price по exact account statement/receipt witness. Unconfigured rates не означают unknown currency разрешена; currency_unqualified отвергается до inference. Caller metadata и provider display name не источник валюты.


## Account witness до первого model probe

Первый model probe не может требовать готовую runtime qualification той же модели.
Для этого Hub вводит отдельную immutable verification account authority: exact
connection/generation/adapter/endpoint, authenticated native statement digest,
валюта, billing tier, время наблюдения/expiry и audited operation. Эта authority
разрешает только own bounded verification; inference/evaluation всё ещё требуют
model qualification. Metadata catalog и старые unqualified account observations
не преобразуются в proof.

OpenRouter collector получает свежий authenticated GET /api/v1/key по собственному
фиксированному HTTPS origin. Native usage в этом account statement имеет USD unit
по [контракту provider](https://raw.githubusercontent.com/OpenRouterTeam/terraform-provider-openrouter/main/docs/data-sources/api_key.md);
[официальный limits contract](https://openrouter.ai/docs/api_reference/limits)
также содержит spend_usd/remaining_usd. Валюта привязана к этому response digest
и unit contract, а не к display name или caller metadata. Отсутствующий/некорректный
statement usage, management/provisioning key или expired authorization не
квалифицируются. Свидетельство не создаёт cash expense и не подтверждает модели.

Перед I/O model probe использует общий financial reserve/claim/settlement: own
purpose grant, immutable probe target, exact price/source, контекст, caps и budgets.
Prepared credential разрешён только с живой account authority и own ciphertext.
Claim ещё раз проверяет generation/endpoint/credential/authority/context/source.
Unknown сохраняет reserve; повторный dispatch запрещён. Изменение или expiry
authority блокирует новые calls, но settlement уже отправленного остаётся frozen.

Эта граница находится в реализации; live qualification и actual model evidence
остаются отдельными gates IMPLEMENTATION_STATUS.
