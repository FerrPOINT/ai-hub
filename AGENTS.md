# AI Hub — правила репозитория

## Назначение и состояние

AI Hub — отдельный сервис Base для доступа приложений к моделям, виртуальных
профилей, воспроизводимых проверок, статистики и расходов. Репозиторий находится
на стадии подготовки к разработке; API, UI, adapters и runtime ещё не реализованы.
Default branch — main. Пользователь поручил подготовить и проверить документы.

## Перед изменениями

Прочитать [индекс](docs/README.md), [требования](docs/PRODUCT_REQUIREMENTS.md),
[архитектуру](docs/ARCHITECTURE.md), [модель данных](docs/DATA_MODEL.md),
[план](docs/IMPLEMENTATION_PLAN.md) и ADR затронутой границы. Проверить git status.
Статус в [CURRENT_STATE](docs/CURRENT_STATE.md) должен соответствовать доказательствам.

## Границы

- AI Hub владеет provider connections/credentials, виртуальными моделями,
  admission/routing, attempts, usage, financial ledger и evaluations.
- Central Auth владеет identity и сессиями; Admin — branding/service catalog;
  Fleet — agent runs/tools; Tracker — задачи; Workflow — фазы; Forge — delivery.
- Никаких запросов к чужим БД, общего vault или импорта global Codex auth.
  Переход существующей Admin AI-основы только по [handoff](docs/contracts/ADMIN_HANDOFF_V1.md).
- SDK SHA читается из .base-revision. Не менять его ради зелёной проверки.

## Реализация

Целевой backend: Rust 2024, Axum 0.8, SQLx 0.8/PostgreSQL 17.
Frontend: React 19, TypeScript 5.9, Vite 6, Tailwind 4, @sdlc/ui.
Слои api → application → domain; infrastructure реализует ports.
Политика принадлежит продукту; технические механизмы берутся из Base.
Документация и пользовательский текст на русском, код и комментарии на английском.

Сначала контракт и causal tests, затем код; UI следует утверждённой
[карте](docs/UI_UX.md) и Base shell. Applied migrations не переписывать.
Credentials только write-only; денежные значения decimal, не binary float.
Каждый запрос фиксирует revision и каждую upstream attempt. Unknown не превращать
в success, нулевую стоимость или разрешение повторить потенциально платный запрос.

## Проверки и поставка

Сейчас действует только `python scripts/check_docs.py` и
`python -m unittest discover -s scripts/tests -v`.
Будущие backend/frontend/runtime gates описаны в [QUALITY_GATE](docs/QUALITY_GATE.md);
их отсутствие не означает PASS. До разработки не создавать фиктивные package,
OpenAPI-generated artifacts, migrations, screenshots или зелёные CI badges.

Сохранять чужую работу; task-owned документация коммитится и публикуется обычным git.
Identity: FerrPOINT <ferrpoint@users.noreply.github.com>. Перед push fetch и проверка
remote; never force-push. Merge/deploy и реальное provider I/O не входят в эту задачу.
Browser proof только Codex in-app browser по workspace policy.
