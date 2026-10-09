# Предметная модель

| Термин          | Значение и владелец                                                  |
| --------------- | -------------------------------------------------------------------- |
| Installation    | Own Hub installation ID и namespace; не общая БД с продуктами        |
| Provider        | Тип внешнего адаптера и policy endpoint; не сам аккаунт              |
| Connection      | Own account authorization, generation, metadata и secret reference   |
| UpstreamModel   | Provider model ID с observed metadata/capability evidence            |
| VirtualModel    | Стабильный slug, отображаемое имя и active revision pointer          |
| ProfileRevision | Immutable параметры/ordered deployments/capabilities/bounds          |
| Deployment      | Connection + upstream model + adapter revision + capability evidence |
| Client          | Приложение/service identity, доступные профили, привязка проекта     |
| Grant           | Ограничение principal/project/model/actions; не локальная identity   |
| Request         | Один логический вызов клиента с frozen profile revision              |
| Attempt         | Один внешний provider invocation в request                           |
| UsageFact       | Доверенные наблюдаемые token/cost факты для attempt, provenance      |
| PriceRevision   | Immutable rates/currency/unit/effective interval/source              |
| LedgerEntry     | Append-only debit/credit/reservation/settlement/correction           |
| Subscription    | Плата/период provider account, quota facts; отдельно от token cost   |
| EvaluationRun   | Frozen dataset/profiles/runner/scorers/config и budget               |
| Aggregate       | Перестраиваемая статистическая проекция canonical facts              |

## Инварианты

- Display name не identity; virtual slug уникален внутри installation, case-sensitive,
  regex `[a-z][a-z0-9-]{1,62}`. Rename display name не изменяет маршрутизацию.
- Один inference/evaluation request имеет одну frozen profile revision; verification
  request вместо неё имеет immutable own probe snapshot. Каждый имеет один
  client/principal/project binding.
  История не следует изменяемому active pointer.
- Request 1:N Attempt. Success одного запроса не делает предыдущую attempt бесплатной.
- Connection generation монотонна; verification относится к точным generation,
  adapter, model и capabilities, а не к слову «подключено».
- Cost unknown и token count unknown представлены null с reason/provenance.
  Ноль допустим только как доказанная величина.
- AI Hub не получает владение tool execution, agent sessions, Task/workflow lifecycle
  или source-control workspaces.
- Client metadata не доказывает user/project identity. Атрибуция к пользователю
  допускается только от доверенного authenticated delegation binding.
- Disable блокирует новые admissions; исторические facts/revisions остаются доступны
  в пределах прав. Archive сохраняет историческую ссылочную целостность.

## Проектное ценообразование

Cost-source revision выбирает auto provider или manual allocation. Project tariff revision определяет цену для Namespace/profile/currency. Project charge — внутреннее начисление; margin amount — charge минус basis, markup — процент от basis. Provider receipt и subscription cash fee не заменяются этими фактами. [Правила](PROJECT_TARIFFS.md).
