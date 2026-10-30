$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
# Set OPENROUTER_API_KEY / OPENROUTER_MODEL in this terminal for live mode.
$env:BORDER_SESSION_TOKEN = [guid]::NewGuid().ToString('N')
$port = if ($env:BORDER_PORT) { $env:BORDER_PORT } else { '3847' }
$env:BORDER_SERVER_ADDR = "127.0.0.1:$port"
$env:BORDER_SERVER_URL = "http://127.0.0.1:$port"
cargo build --locked -p border-game -p border-caller-server
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
New-Item -ItemType Directory -Force output | Out-Null
$server = Start-Process -FilePath "$PSScriptRoot/target/debug/border-caller-server.exe" -PassThru -RedirectStandardOutput 'output/server-out.log' -RedirectStandardError 'output/caller-server.log'
$key = $env:OPENROUTER_API_KEY
$model = $env:OPENROUTER_MODEL
try {
    $ready = $false
    for ($i = 0; $i -lt 100; $i++) {
        if ($server.HasExited) { throw 'Interview server exited. See output/caller-server.log.' }
        try { $health = Invoke-RestMethod "$env:BORDER_SERVER_URL/health"; if ($health.ok) { $ready = $true; break } } catch { Start-Sleep -Milliseconds 100 }
    }
    if (!$ready) { throw 'Interview service failed to start.' }
    Remove-Item Env:OPENROUTER_API_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:OPENROUTER_MODEL -ErrorAction SilentlyContinue
    & "$PSScriptRoot/target/debug/border-game.exe" @args
} finally {
    if (!$server.HasExited) { Stop-Process -Id $server.Id }
    if ($key) { $env:OPENROUTER_API_KEY = $key }
    if ($model) { $env:OPENROUTER_MODEL = $model }
}
