param([ValidateSet('foundation_pg','financial_pg','control_pg','budget_pg','pricing_pg')][string]$TestName='foundation_pg')
$ErrorActionPreference = 'Stop'
$hubRepo = Split-Path $PSScriptRoot -Parent
$hubWorkspace = Split-Path $hubRepo -Parent
$hubFixture = 'pdlc3-aihub-foundation-' + [Guid]::NewGuid().ToString('N').Substring(0,12)
$hubPrivate = Join-Path $hubWorkspace ('.local/ai-hub/fixtures/' + $hubFixture)
New-Item -ItemType Directory -Force -Path $hubPrivate | Out-Null
$hubPassword = [Convert]::ToHexString([System.Security.Cryptography.RandomNumberGenerator]::GetBytes(32)).ToLowerInvariant()
$hubPgEnv = Join-Path $hubPrivate 'postgres.env'
$hubTestEnv = Join-Path $hubPrivate 'test.env'
[IO.File]::WriteAllText($hubPgEnv, "POSTGRES_PASSWORD=$hubPassword`n")
[IO.File]::WriteAllText($hubTestEnv, "AIHUB_TEST_DATABASE_URL=postgresql://aihub_fixture:$hubPassword@127.0.0.1:5432/aihub_fixture`n")
# Use an already installed image; a temporary DB never touches pdlc-common.
$hubPgImage = & docker image inspect postgres:17.6-alpine --format '{{.Id}}'
if ($LASTEXITCODE -ne 0 -or $hubPgImage -notmatch '^sha256:[0-9a-f]{64}$') { throw 'PostgreSQL 17 fixture image unavailable' }
$hubCreated = $false
try {
    & docker run -d --pull never --name $hubFixture --label aihub.owner=goal-v1 --label aihub.kind=disposable-test --cpus 1 --memory 512m --pids-limit 128 --tmpfs /var/lib/postgresql/data:rw,size=256m --env-file $hubPgEnv $hubPgImage | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Fixture creation failed' }
    $hubCreated = $true
    $hubReady = $false
    for ($hubTry=0; $hubTry -lt 30; $hubTry++) {
        & docker exec $hubFixture pg_isready -U postgres 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) { $hubReady=$true; break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $hubReady) { throw 'Fixture did not become ready' }
    $hubSql = "CREATE ROLE aihub_fixture LOGIN PASSWORD '$hubPassword' NOSUPERUSER NOCREATEDB NOCREATEROLE;`nCREATE DATABASE aihub_fixture OWNER aihub_fixture;`nREVOKE CONNECT ON DATABASE postgres FROM PUBLIC;`nREVOKE CONNECT ON DATABASE template1 FROM PUBLIC;"
    $hubSql | & docker exec -i $hubFixture psql -v ON_ERROR_STOP=1 -U postgres | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Fixture role/database initialization failed' }
    & (Join-Path $PSScriptRoot 'rust.ps1') -CargoArgs @('test','--locked','-p','aihub-infrastructure','--test',$TestName,'--','--ignored','--nocapture') -Network "container:$hubFixture" -EnvironmentFile $hubTestEnv
} finally {
    if ($hubCreated) {
        $hubOwner = & docker inspect $hubFixture --format '{{index .Config.Labels "aihub.owner"}}'
        if ($LASTEXITCODE -eq 0 -and $hubOwner -eq 'goal-v1') {
            & docker rm -f $hubFixture | Out-Null
        }
    }
    # Exact task-owned transient files; no recursive deletion or volume cleanup.
    Remove-Item -LiteralPath $hubPgEnv,$hubTestEnv -Force
    $hubPassword=$null; $hubSql=$null
}
