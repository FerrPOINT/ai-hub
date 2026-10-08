# Переход Admin AI-основы в AI Hub

Это подготовленный план интеграции, не выполненный перенос и не новый accepted
контракт соседних репозиториев. Нынешние Admin ADR-0018/AI_PROFILE_V1 продолжают
действовать для существующего runtime до owner-coordinated migration.

## Источник и отличие

В current Admin есть registry/per-model context/CAS/outbox, own ai-runtime vault,
managed authorization и scoped execution journal. README явно оставляет
verification producer/inference закрытыми. Эти ограничения нельзя объявлять
готовыми функциями AI Hub.
Новый Hub добавляет ordered profiles, compatible inference, standalone clients,
attempt/financial ledger, statistics и evaluations. Прежний scoped Fleet grant
не заменяется автоматически Hub key.

## Target ownership

| Объект                                                 | До перехода                     | После accepted перехода                   |
| ------------------------------------------------------ | ------------------------------- | ----------------------------------------- |
| Branding/platform service catalog                      | Admin                           | Admin                                     |
| AI settings/revisions/credentials/inference/accounting | Admin registry + own ai-runtime | AI Hub own DB/vault/contracts             |
| Central identity/session                               | Central Auth                    | Central Auth                              |
| Agent/task grants/roles/tool execution                 | Fleet/Tracker/Workflow          | Те же владельцы; signed adapter binding   |
| Hub navigation                                         | Не зарегистрирован              | Admin service declaration только metadata |

## Порядок

1. Owner ADR/consumer inventory: exact source/runtime/contracts/grants/users и
   existing state backups; определить объекты для миграции и reconciliation mapping.
2. Hub S1–S6 проходит own gates, native providers real qualified; consumer пока старый.
3. Добавить versioned Hub client/grant adapter и readonly compatibility metadata;
   direct SQL access и shared vault запрещены.
4. Передать только разрешённые definitions/revision mapping через audited export/
   import. Credentials authorizes owner заново либо отдельный explicitly scoped
   sealed migration с old/new key/restore verification; не copy global HOME.
5. Один consumer opt-in на exact Hub endpoint/key/revision; проверить actual model,
   usage/cost/stream/tools/recovery и grants без duplicate billing.
6. Регистрировать new service navigation, отключить прежние AI writes только после
   подтверждённой миграции/rollback. Нельзя иметь двух активных config owners.
7. Расширять consumers по immutable receipt и grace compatibility window.

## Rollback и stop

Consumer возвращается на предыдущий own endpoint/contract только при подтверждённом
cancel/reconcile dispatched Hub requests. New Hub ledger/history сохраняется.
Unmapped profile/generation/grant, no backup, uncertain billed invocation или
unknown schema compatibility блокируют cutover, но не разработку isolated Hub.
Никакого runtime переключения в documentation baseline; соседние repo неизменны.
