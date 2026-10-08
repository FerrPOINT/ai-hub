# Участие в разработке AI Hub

Начать с [docs/README](docs/README.md) и [плана](docs/IMPLEMENTATION_PLAN.md).
Подготовка документов не запускает inference и не создаёт credentials.

## Работа с документацией

Из корня чистого checkout, Python 3.12 или совместимый 3.11+:

```shell
python scripts/check_docs.py
python scripts/check_design.py
python -m unittest discover -s scripts/tests -v
uv run --no-project --with openapi-spec-validator==0.7.2 python scripts/validate_contract.py
git diff --check
```

Проверяются ссылки/anchors, обязательный состав, JSON-контракты, матрицы FR/NFR,
TC/этапов/маршрутов и decimal-пример расходов. Это структурный gate плюс пример
арифметики; смысл и межсервисные границы проверяются по PRE_DEVELOPMENT_GATE.

## Разработка после документационного baseline

Первый этап создаёт реальные Cargo/pnpm manifests, lockfiles, миграции,
runtime config и воспроизводимые команды. До него инструкции сборки только
целевые: см. [LOCAL_SETUP](docs/LOCAL_SETUP.md), [QUALITY_GATE](docs/QUALITY_GATE.md).
SDK checkout должен точно соответствовать .base-revision; mutable main не зависимость.

Новая логика включает positive/negative/recovery tests; финансовые переходы и
streaming требуют реального boundary test. DTO → draft OpenAPI → Rust handlers →
generated spec/types; переход схемы описан в [API](docs/API.md).
Нет mock-only доказательства provider capability или реального списания.

## Git и review

Conventional Commits; одна логическая задача на commit/PR, без unrelated cleanup.
Проверить remote перед публикацией; начальный documentation baseline может
создать main пустого репозитория. Force push запрещён.
Секреты, .env, transcripts, state, локальные отчёты, caches и executable не публикуются.
После реализации обязательны checks конкретного stage, review и exact-SHA evidence.
