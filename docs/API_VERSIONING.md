# Эволюция API и совместимость

Текущий контракт 0.4.0-design — проектный, не опубликованная runtime-версия.
Контракт control API /api/v1 отделён от compatible /v1, profile revision и DB schema.
Generated OpenAPI заменяет draft только через parity gate и semantic fixtures.

| Изменение                                                                   | Правило                                                       |
| --------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Новое optional поле с documented default/null                               | Additive v1, проверка старого клиента без поля                |
| Required поле, удаление/переименование, изменение money/confidence или auth | Breaking: новый major transport и owner ADR                   |
| Дополнительный enum outcome                                                 | Старый клиент показывает unknown/unsupported, не success/zero |
| Удаление capability/profile parameter                                       | Новый контракт или явный отказ, не молчаливое отбрасывание    |
| SDK/adapter update                                                          | Exact SHA/version + consumer fixtures; не URL API version     |

Breaking removal получает announced deprecation date, Sunset и usage inventory.
Минимальное окно — 90 дней после доступного совместимого replacement и подтверждения
named consumers. Финальное удаление требует accepted consumer migration/rollback.
Security emergency может сократить окно только отдельным owner decision с объяснённым отказом.
Legacy Admin API сохраняет прежнюю policy до cutover; redirect только UI /ai,
не автоматический HTTP redirect credentials/inference request.
TC-033 проверяет omitted Namespace, nullable money, frozen revisions, wire SSE/tools
и совместимость fixtures. Наличие OpenAPI файла не доказывает runtime compatibility.
