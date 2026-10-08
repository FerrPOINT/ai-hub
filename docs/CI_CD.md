# CI и delivery

На documentation stage hosted workflow не создан: Python stdlib gate воспроизводим
локально, fake backend/frontend jobs и badges отсутствуют.
S1 добавляет один .github/workflows/ci.yml по Base convention, если нужен hosted mirror:
docs, backend, frontend, independent minimum-rust.
Node 22.20.0/pnpm10.28.1 и Rust1.88.0 закрепляются одинаково в own Docker/CI;
Base consumer .base-revision проверяется before build.
No secrets для untrusted fork jobs; provider I/O/live login не обычный PR build.

Docs gate запускается offline и не нужен GitHub token. Backend real own PG service,
fmt/clippy/tests/OpenAPI drift; frontend frozen/type/lint/unit/package/theme/build.
Generate code не переписывает lockfile в CI. Runner artifacts содержат safe
test/evidence manifests, no credentials/transcripts/vault.
Git publication ordinary main/feature branch, no force. PR readiness требует exact
head/check/readback, user-facing gate отдельно. Merge/deploy только authorized scope.
Initial empty repository получает documentation main, без несуществующего release tag.
