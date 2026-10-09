# Namespace и контекст потребителей v1

Статус: Target approved, проектный контракт. Нормативные технические типы принадлежат Base.
`NamespaceRef` содержит `registry_instance_id` и `namespace_id`: два nonnil UUID.
Название, slug и source workspace не являются идентичностью.

## Владение и привязка

Admin владеет registry; Tracker — project/Task; Forge — repository; Fleet — execution.
Hub сохраняет собственную проверенную projection `NamespaceBinding` и историю readback.
Подключения и виртуальные профили общие для установки; Namespace не копирует secrets/profiles.
Client и dataset получают nullable `namespace`. Null означает явно непривязанный ресурс.
Прежний project_binding сохраняется; при заданном Namespace он должен совпасть с
verified tracker_instance_id/project_id этой binding. Подстановка по имени запрещена.
Binding клиента фиксируется до выдачи ключа; смена требует отзыва старого ключа.
Каждый request фиксирует Namespace snapshot из trusted client, не из prompt/metadata.
Project budget использует ту же verified binding; installation budget остаётся общим.

## API и URL

`GET /api/v1/namespaces` возвращает bounded authorized projections; это не registry CRUD.
Query UUID pair задаётся вместе. Malformed/duplicate/foreign pair → 400/403, не all projects.
Отсутствие пары — authorized all scope; непривязанные записи доступны отдельным фильтром.
UI хранит пару в URL текущей вкладки; Back/Forward/reload восстанавливают контекст.
Selector не создаёт grants. При unavailable/archived контексте сохранённая история читается
по прежним правам, новые scoped admissions и mutations закрыты до verified recovery.
Registry reads fixed-origin, bounded 10s/64KiB, redirects off; user PAT не пересылается.
Reader outage не заменяет stale projection на active и не создаёт новую binding.

## Fleet/Forge adaptation

Service authentication и AI Hub key — разные границы. Key разрешает конкретные профили,
Namespace, expiry/concurrency/cost policy. Human PAT не становится provider credential.
Обычный совместимый SDK не требует Task/run. SDLC adapter использует существующий Base
`ExecutionContextV2`, сверяет его Namespace с client binding и сохраняет provenance.
TaskRef/RepositoryRef не становятся grants; source owner/instance и подпись/lease проверяются
до dispatch. Strict SDLC v1 не расширяется. Target transport задан в [SERVICE_ADAPTER_V1](SERVICE_ADAPTER_V1.md): отдельный signed service endpoint, raw-body digest и expiry/revocation/fence. Public OpenAI-compatible metadata не заменяет этот envelope.
Original request ID/idempotency/revision сохраняются; uncertain response → readback,
не новый provider вызов. Legacy Fleet grant не преобразуется автоматически в Hub key.

## Проверки

TC-035: одинаковые имена в разных registry/Namespace, неверная пара, foreign grant,
null binding, archived/unavailable, независимые вкладки, replay с прежней identity.
Live SDK/reader/consumer acceptance — not_run. Старый .base-revision сохранён;
новый Namespace cohort фиксируется отдельным подтверждённым SHA при реализации.
