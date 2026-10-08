# Виртуальные модели и маршрутизация

## Draft и revision

VirtualModel хранит mutable draft и active revision pointer. Публикация создаёт
immutable ProfileRevision с ordered deployment IDs, exact connection generation,
adapter revision, input/output/context bounds, параметрами, required capabilities,
timeout/max-attempts, разрешёнными caller overrides и mode.

Mode `development` разрешает только explicit ordered fallback; `pinned_test`
имеет ровно один deployment, max_attempts=1 и без hidden retries/fallback.
Default max_attempts=3, cap=5; provider I/O timeout=120s, cap=600s; один общий
request deadline, fallback не начинает новую полную deadline.
Параметры не поддерживаемые всеми candidate deployments блокируют publication.
Общее context/output bound не превышает минимальное подтверждённое bound каждого.

Verification TTL publication — 15 минут; сохранённая active revision не истекает
сама, но admission проверяет текущие account generation/capability validity/access.
Publication expires_at и runtime qualification — разные свойства. Qualification
active до invalidation по generation/adapter/endpoint/подтверждённым capability
changes; expiry publication proof сам не выключает опубликованный профиль.
Каждый admission связывается с current qualified receipt и его bounds.
Provider contradiction инвалидирует qualification; новая проверка — explicit
budgeted operation, не скрытый платный background вызов каждые 15 минут.
Изменение draft/generation инвалидирует proof. Operator rollback меняет pointer
на прежнюю revision через новую audited operation; ongoing request остаётся frozen.
Raw draft не доступен inference API. Archive запрещён для active pointer без disable.
Создание draft не требует proof. Connection-model qualification независима от
профиля; profile verification берёт frozen draft/version/config hash и проверяет
весь chain. Публикация принимает один exact profile proof и его child receipts.
Allowed caller overrides по умолчанию пусты и сохраняются в revision отдельно.

## Eligibility и fallback

Перед каждым новым attempt проверить: разрешён ли профиль client/project, не revoked
ли connection, capability qualified, quota/cooldown и budget bound.
Названия моделей не подбираются substring и не заменяются «похожей» моделью.

| Ситуация                                                                 | Действие                                                  |
| ------------------------------------------------------------------------ | --------------------------------------------------------- |
| 429 с adapter-qualified доказательством rejection/no acceptance          | Следующий разрешённый deployment, отдельная attempt       |
| Доказанный connect failure до отправки request body                      | Возможен explicit fallback                                |
| Auth failure / invalid input / unsupported capability / context overflow | Typed failure; никакой обход account/input policy         |
| Timeout/5xx после отправки либо unknown provider acceptance              | Unknown, резерв сохраняется, автоматического resend нет   |
| Уже отправлен любой output/tool fragment клиенту                         | Завершить stream ошибкой; переключение upstream запрещено |
| Все eligible исчерпаны                                                   | provider_unavailable с trace; без прямого обхода Hub      |

Provider adapter документирует certainty; generic статус 5xx сам по себе не
доказательство безопасного retry. Повтор к одному upstream в v1 выключен.
Cooldown только на exact deployment: 30s default, не дольше min(Retry-After,300s),
одна recovery probe; profile revision не переписывается.
Provider-owned native automatic retries должны быть выключены либо учтены как
qualified attempts; иначе adapter не принимается для cost/budget guarantees.

## Ответ и trace

Public response model — вызванный virtual slug; metadata/readback раскрывает exact
revision, actual provider/upstream model, attempts и fallback reason без secrets.
Если provider сообщил alias/неподтверждённое actual name — actual_model_verified=false.
Реальный лимит аккаунта и модель нельзя вывести только из каталога.
