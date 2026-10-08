# Поставка и переключение

Deployment contract запланирован; новых images/runtime resources ещё нет.

## Candidate

Exact Hub source/Base SHAs, frozen lockfiles/toolchain, dependency notices,
schema revision, non-root image digest/ID и config hash в внешнем delivery manifest.
Не использовать latest как release identity. Source workspace inventory не installer.
Build export clean git sources; target runtime explicit installation/URLs/ports.

## Target до Apply

Owner инвентаризирует защищённые соседние installations, own DB/vault/key/bundle,
capacity/ports, backup receipt и rollback image/config. Prepare только проверяет,
не создаёт пустую replacement DB. Старт, infra/migrate и Apply отдельные операции.
Secrets private, filesystem read-only где возможно, no Docker socket/host checkout
или global Codex HOME. Provider egress fail-closed по deployment allowlist.
Common infrastructure не перезапускается при app update.

## Приёмка

Target served version/build info совпадает с candidate SHA/Base/image/config.
Health отдельно от DB/schema, central auth, provider capability, data-plane,
financial ledger и UI acceptance. Пробный synthetic consumer сначала; платный
external smoke только explicit budget/grant для выбранного account.
Current URLs/actual accounts фиксируются в private rollout receipt, не угадываются.
Documentation delivery не разрешает Apply, production writes или перенос Octo.

## Rollback

Переключить own application на сохранённый immutable image/config при schema
compatibility; disable new admissions до recovery. Retained ledger/vault не
удаляются, dispatched unknown attempts не повторяются. Accounting/projection
readback и consumer compatibility обязательны после rollback.
Admin/Octo consumer transition — отдельный [handoff](contracts/ADMIN_HANDOFF_V1.md).
