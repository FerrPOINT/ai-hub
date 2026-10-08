# Локальная подготовка

Сейчас нет Cargo/pnpm manifests, Compose или приложения. Единственная
работающая команда — documentation gate из README; его Python tooling stdlib-only.

## Рабочий baseline

- Rust 2024, build/MSRV приложения 1.88.0; Base shared/auth SDK MSRV не повышается.
- Node 22.20.0/pnpm 10.28.1, React 19/TS 5.9/Vite 6/Tailwind 4.
- PostgreSQL 17 own database/role; local data полностью отдельно от Study/PDLC соседей.
- Exact Base .base-revision materialized beside own clean source set.
- На PDLC host source/dev сессия через root enter-dev.ps1, process-local.
  Это настройка tooling, не установка Hub runtime.

## S1 делает запуск реальным

Создать actual manifests и lockfiles; documented own PostgreSQL migrations,
validated env loader, own key/state initializer для нового пустого target.
Добавить root dev Compose с namespaced resources/loopback ports, collision inventory,
health/readiness и no external_calls default. Owner Base/PDLC выбирает ports;
в документационном baseline номера не резервируются фиктивно.
No missing prior installation snapshot — no automatic empty state.

Smoke: clean source/SHA, image hashes, migrate own disposable DB, process/readiness,
invalid grant denied и external_calls=false. После этого README получает реальные
команды из подтверждённого Compose, без credentials.
Данные уже существующей установки, соседние containers/networks/secrets не меняются.
