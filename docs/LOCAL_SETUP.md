# Локальная подготовка

Cargo/pnpm manifests и locks созданы, scoped commands доступны ниже.
Постоянный установленный bundle пока не расширен новым сервисом.

```powershell
. ../enter-dev.ps1
python scripts/materialize_dependencies.py
python scripts/verify_dependencies.py
./scripts/rust.ps1 -CargoArgs @('check','--locked','-p','aihub-api')
./scripts/test_foundation_pg.ps1
./scripts/rust.ps1 -CargoArgs @('run','--locked','-p','aihub-api','--','export-openapi','openapi/openapi.json')
pnpm --dir frontend openapi:generate
pnpm --dir frontend typecheck
```

PG harness использует own временный container, tmpfs и непубличную DB/role.
Он не подключается к common PostgreSQL и не меняет его пользователей/данные.
CLI migrate/initialize применяются явно; serve никогда не выполняет bootstrap.

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
