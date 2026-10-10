param(
    [Parameter(Mandatory)][string[]]$CargoArgs,
    [string]$Network = '',
    [string]$EnvironmentFile = ''
)
$ErrorActionPreference = 'Stop'
$hubRepo = Split-Path $PSScriptRoot -Parent
$hubWorkspace = Split-Path $hubRepo -Parent
. (Join-Path $hubWorkspace 'enter-dev.ps1')
& python (Join-Path $PSScriptRoot 'verify_dependencies.py') --workspace $hubWorkspace
if ($LASTEXITCODE -ne 0) { throw 'SDK verification failed' }
$hubInventory = Get-Content -LiteralPath (Join-Path $hubWorkspace 'source-workspace.json') -Raw | ConvertFrom-Json
$hubImage = $hubInventory.toolchain.rustDevImageId
if ($hubImage -notmatch '^sha256:[0-9a-f]{64}$') { throw 'Exact workspace Rust image missing' }
& docker image inspect $hubImage --format '{{.Id}}' | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Workspace Rust image unavailable; no implicit pull' }
$hubBuildRoot = Join-Path $hubWorkspace '.local/ai-hub'
foreach ($hubChild in @('cargo', 'target', 'reports')) { New-Item -ItemType Directory -Force -Path (Join-Path $hubBuildRoot $hubChild) | Out-Null }
$hubArgs = @('run','--rm','--label','aihub.owner=goal-v1','--cpus','2','--memory','3g','--pids-limit','512',
    '--mount',"type=bind,source=$hubRepo,target=/workspace/ai-hub",
    '--mount',"type=bind,source=$hubWorkspace/.local/sdk/services-base-875cac2,target=/workspace/.local/sdk/services-base-875cac2,readonly",
    '--mount',"type=bind,source=$hubWorkspace/.local/sdk/services-base-81decf7,target=/workspace/.local/sdk/services-base-81decf7,readonly",
    '--mount',"type=bind,source=$hubBuildRoot/cargo,target=/usr/local/cargo/registry",
    '--mount',"type=bind,source=$hubBuildRoot/target,target=/target",
    '-e','CARGO_TARGET_DIR=/target','-w','/workspace/ai-hub')
if ($Network) { $hubArgs += @('--network',$Network) }
if ($EnvironmentFile) {
    $hubEnvPath = (Resolve-Path -LiteralPath $EnvironmentFile).Path
    $hubArgs += @('--env-file',$hubEnvPath)
}
& docker @hubArgs $hubImage cargo @CargoArgs
if ($LASTEXITCODE -ne 0) { throw "Scoped Cargo command failed: $LASTEXITCODE" }
