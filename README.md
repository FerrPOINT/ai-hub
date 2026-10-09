# AI Hub

**AI Hub** — проект отдельного сервиса Base: единая точка доступа к нейронкам
для приложений и агентов, собственные виртуальные модели для разработки и
тестирования, статистика и контроль расходов.

Статус: DEVELOP_READY для документов и дизайна после закрытия semantic audit. Требования и проектные контракты документированы;
backend, frontend и runtime ещё не реализованы. Наличие документов не доказывает
работу endpoint, доступ подписки, точность provider billing или live SSO.

<a name="overview"></a>

## Обзор

| Поле                     | Baseline                                                                 |
| ------------------------ | ------------------------------------------------------------------------ |
| Repository               | FerrPOINT/ai-hub, main                                                   |
| Backend — целевой        | Rust 2024, Axum 0.8, SQLx 0.8, PostgreSQL 17                             |
| Frontend — целевой       | React 19, TypeScript 5.9, Vite 6, Tailwind 4, @sdlc/ui                   |
| Build baseline — целевой | Rust 1.88.0, Node 22.20.0, pnpm 10.28.1                                  |
| Base SDK                 | Exact SHA в [.base-revision](.base-revision)                             |
| API                      | [Draft OpenAPI 3.1](docs/contracts/openapi.v1.json); runtime отсутствует |
| Порты                    | Пока не выделены; deployment owner выбирает свободные loopback ports     |
| License                  | FerrPOINT Proprietary Source-Available Evaluation License v1.0           |

<a name="features"></a>

## Возможности v1

Все строки — **целевой объём**, не работающие возможности.

| Область            | Содержание                                                             |
| ------------------ | ---------------------------------------------------------------------- |
| Провайдеры         | Own connections, secret-safe catalog, actual verification              |
| Виртуальные модели | Имена, immutable revisions, ordered fallback, capabilities и параметры |
| API доступа        | /v1/models, /v1/chat/completions, /v1/responses, stream и caller tools |
| Тестирование       | Закреплённые revisions/upstreams, datasets, сопоставимые результаты    |
| Статистика         | Запросы/attempts/tokens, latency, errors, fallback, доступность        |
| Расходы            | Confirmed/estimated/unknown, pricing snapshots, subscriptions, budgets |
| Управление         | Central Auth, own client grants, audit, ограниченный export            |

<a name="quick-start"></a>

## Быстрый старт

Сейчас можно проверить только documentation baseline:

```shell
python scripts/check_docs.py
python -m unittest discover -s scripts/tests -v
```

Проектный старт: [ТЗ](docs/TZ.md) → [требования](docs/PRODUCT_REQUIREMENTS.md) →
[этапы](docs/IMPLEMENTATION_PLAN.md) → [pre-development gate](docs/PRE_DEVELOPMENT_GATE.md).
Образов и команд запуска приложения пока нет.

<a name="architecture"></a>

## Архитектура и границы

| Поток                                      | Владелец результата                             |
| ------------------------------------------ | ----------------------------------------------- |
| UI → control API                           | AI Hub settings, revisions, grants, audit       |
| App/agent → admission → adapter → provider | AI Hub request/attempt/accounting               |
| Terminal facts → ledger → statistics       | AI Hub single canonical source                  |
| JWT/session validation                     | Central Auth; права AI Hub проверяет сам        |
| Branding/catalog                           | Admin Panel; Hub потребляет versioned contracts |

[Архитектура](docs/ARCHITECTURE.md) описывает компоненты и транзакционные границы.
Прежняя [Admin AI-основа](docs/contracts/ADMIN_HANDOFF_V1.md) — источник требований
для перехода, не действующий backend AI Hub. Текстовая таблица заменяет runtime
диаграмму в этом предварительном baseline; реальных UI screenshots пока нет.

<a name="quality"></a>

## Качество и документы

[Полный индекс](docs/README.md), [API](docs/API.md), [Data model](docs/DATA_MODEL.md),
[учёт расходов](docs/ACCOUNTING.md), [аналитика](docs/ANALYTICS.md),
[план проверок](docs/TESTING.md), [CURRENT_STATE](docs/CURRENT_STATE.md).
Локальный documentation gate — основной; hosted backend/frontend CI появится
только после реальных manifests. Release, coverage и live acceptance не заявлены.

<a name="safety"></a>

## Безопасность и лицензия

[SECURITY](SECURITY.md), [CONTRIBUTING](CONTRIBUTING.md), [CHANGELOG](CHANGELOG.md).
Credentials и provider authorization принадлежат AI Hub; passwords/sessions
принадлежат Central Auth. Hub не выполняет caller tools и не хранит обычные
transcripts в аналитике. Существующие Octo/PDLC installations этой поставкой не меняются.

[LICENSE](LICENSE) и [NOTICE](NOTICE) — единый FerrPOINT license;
[third-party notices](THIRD_PARTY_NOTICES.md) сохраняют границы чужих компонентов.

## Полный дизайн и готовность

[DEVELOP_READY](docs/DEVELOP_READY.md) — полный handoff до разработки.
[Интерактивный дизайн](docs/design/README.md) / [галерея](docs/design/gallery.html).
15 screens,3 themes, explicit states/forms/fields; IAB evidence — только prototype.
Typed DD34 tables/294 fields и execution packets позволяют начать S1.

## Дизайн и выделение из Admin

[Прототип](docs/design/prototype.html), [руководство](docs/USER_GUIDE.md), [точная карта переноса](docs/contracts/ADMIN_HANDOFF_V1.md), [Namespace](docs/contracts/NAMESPACE_V1.md). Документы описывают целевой сервис; runtime/credential/data cutover не выполнялся.

## Тарифы проектов

[PROJECT_TARIFFS](docs/PROJECT_TARIFFS.md): 1M input/output, default 20% markup, свои проектные цены, OpenRouter auto и manual Ollama Online/ChatGPT. В [прототипе](docs/design/prototype.html) добавлен /tariffs. Реальный billing/provider/runtime not_run.
