# Конфигурация

Проектные env names; .env example/application loader появятся в S1.
Prefix AIHUB_. Secrets передаются через file/secret provider, не literal config.

| Имя                                | Назначение / безопасный default                               |
| ---------------------------------- | ------------------------------------------------------------- |
| AIHUB_INSTALLATION_ID              | Required own installation UUID; stable across restarts        |
| AIHUB_LISTEN                       | Loopback/private internal; публичный bind explicit deployment |
| AIHUB_DATABASE_URL_FILE            | Required private own DSN file; no default database            |
| AIHUB_VAULT_KEY_FILE               | Required own encryption key вне state/backup                  |
| AIHUB_AUTH_ISSUER                  | Exact trusted Central Auth issuer                             |
| AIHUB_AUTH_JWKS_URL                | Trusted endpoint; validated deployment URL                    |
| AIHUB_AUTH_BASE_URL                | Bridge session/PAT validation endpoint                        |
| AIHUB_ADMIN_BASE_URL               | Branding/catalog; отсутствие не расширяет grants              |
| AIHUB_CORS_ORIGINS                 | Exact UI origins, no wildcard with credentials                |
| AIHUB_PROVIDER_ENDPOINTS_FILE      | Deployment allowlist включая exact local Ollama               |
| AIHUB_EXTERNAL_CALLS               | false default; true только explicit qualification/rollout     |
| AIHUB_MAX_BODY_BYTES               | 2097152                                                       |
| AIHUB_REQUEST_TIMEOUT_SECONDS      | 120; validated range 1..600                                   |
| AIHUB_MAX_ATTEMPTS                 | 3; cap 5                                                      |
| AIHUB_MAX_CONCURRENCY              | 20 default bounded installation limit                         |
| AIHUB_REPLAY_PAYLOAD_TTL_SECONDS   | 86400 max                                                     |
| AIHUB_METADATA_RETENTION_DAYS      | 90                                                            |
| AIHUB_LEDGER_RETENTION_DAYS        | 365; archival сохраняет dedupe/evidence                       |
| AIHUB_EVALUATION_ARTIFACT_TTL_DAYS | 7                                                             |
| AIHUB_NATIVE_BINARY_PATH / VERSION | Own verified native adapter binary, no auto latest            |
| AIHUB_NATIVE_HOME                  | Own encrypted-state projection/tmpfs, no global auth          |
| AIHUB_METRICS_LISTEN               | Private listener, no sensitive labels                         |

Startup валидирует absolute secret paths, own marker/key, config bounds, auth
origins и encrypted state. Missing state/key — error, не fresh empty replacement.
Settings API выдаёт только allowed effective config+revision, no env dump.
Secrets rotation не требует rewriting source; advertised limits correspond to
applied config revision and are verified via runtime readback.
