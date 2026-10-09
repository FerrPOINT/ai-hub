# Потребитель AI Hub v1

Target approved. Примеры [consumer-flow](../examples/consumer-flow.json) синтетические;
никакие команды этого документа не проверены на live endpoints.

## Настройка и вызов

Hub client получает immutable verified Namespace binding и allowlist published profiles.
Ключ выдаётся отдельно с infer/read_own_usage и явным read_result при необходимости.
Consumer выбирает stable slug либо разрешённую published revision. Provider endpoint,
credential, authorization generation и actual model не caller overrides.
Надёжный клиент задаёт свой UUID Idempotency-Key; повтор same principal/payload/revision
возвращает прежний request. Новый semantic intent использует новый key.

| Сценарий                                 | Результат и действие                                                        |
| ---------------------------------------- | --------------------------------------------------------------------------- |
| Nonstream terminal success               | Сохранить X-AIHub-Request-Id и frozen revision; читать own trace/expenses   |
| SSE                                      | Headers до content; ordered deltas и terminal; не склеивать разные attempts |
| Caller function tools                    | Consumer исполняет разрешённый tool и передаёт результат новым запросом     |
| 412 config edit                          | Сохранить draft, прочитать новую версию, согласовать явно                   |
| 429 budget denied до dispatch            | Проверить scope/reset/reserve; не bypass Hub                                |
| Timeout после dispatch / request_unknown | GET original request; не resend с новым key                                 |
| Replay expired                           | Metadata остаётся; отсутствующий reply не повторяет provider                |

Fleet/Forge adapters применяют [Namespace contract](NAMESPACE_V1.md), отдельную service
identity и owner grants. Hub не владеет Task lifecycle, agent roles или исполнением tools.
После cutover old endpoint не выбирается автоматически при ошибке Hub: сохранённый
rollback candidate включается оператором после cancel/reconcile и financial readback.

## SDLC wire

[Подписанный service-adapter](SERVICE_ADAPTER_V1.md) определяет отдельный endpoint, Header envelope, trusted issuer, Base V2 body, raw-byte digest, revocation и recovery. Public /v1 не требует Task/run. Не использовать ordinal r12 вместо UUID profile revision. Runtime consumer qualification остаётся S7.
