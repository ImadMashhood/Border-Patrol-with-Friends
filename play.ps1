$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$gameArgs = @()
$joinMode = $false
foreach ($item in $args) {
    if ($item -eq '--join') { $joinMode = $true }
    elseif ($item.StartsWith('--join=')) { $joinMode = $true; $env:BORDER_HOST_URL = $item.Substring(7) }
    elseif ($item.StartsWith('--seed=')) { $env:BORDER_HOST_SEED = $item.Substring(7); $gameArgs += $item }
    else { $gameArgs += $item }
}
cargo build --locked -p border-game -p border-caller-server
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
New-Item -ItemType Directory -Force output | Out-Null
$children = @()
$key = $env:OPENROUTER_API_KEY
$model = $env:OPENROUTER_MODEL
try {
    if ($joinMode) {
        if (!$env:BORDER_HOST_URL) {
            $session = Get-Content output/local-session.json | ConvertFrom-Json
            $env:BORDER_HOST_URL = $session.url
            $env:BORDER_HOST_JOIN_CODE = $session.join_code
        }
        if (!$env:BORDER_HOST_JOIN_CODE) { throw 'Set BORDER_HOST_JOIN_CODE to the host invite code.' }
    } else {
        $env:BORDER_HOST_MANUAL_CLOCK = '0'
        $env:BORDER_SESSION_TOKEN = [guid]::NewGuid().ToString('N')
        $env:BORDER_HOST_OPERATOR_TOKEN = [guid]::NewGuid().ToString('N')
        $env:BORDER_HOST_GATEWAY_TOKEN = [guid]::NewGuid().ToString('N')
        if (!$env:BORDER_HOST_JOIN_CODE) { $env:BORDER_HOST_JOIN_CODE = [guid]::NewGuid().ToString('N').Substring(0,12) }
        $port = if ($env:BORDER_PORT) { $env:BORDER_PORT } else { '3847' }
        $hostPort = if ($env:BORDER_HOST_PORT) { $env:BORDER_HOST_PORT } else { '8090' }
        $env:BORDER_SERVER_ADDR = "127.0.0.1:$port"
        $env:BORDER_GATEWAY_URL = "http://127.0.0.1:$port"
        $env:BORDER_HOST_URL = "http://127.0.0.1:$hostPort"
        if (!$env:BORDER_HOST_BIND) { $env:BORDER_HOST_BIND = "127.0.0.1:$hostPort" }
        $children += Start-Process -FilePath "$PSScriptRoot/target/debug/border-caller-server.exe" -PassThru -RedirectStandardOutput 'output/server-out.log' -RedirectStandardError 'output/caller-server.log'
        $children += Start-Process -FilePath "$PSScriptRoot/target/debug/border-host.exe" -PassThru -RedirectStandardOutput 'output/host-out.log' -RedirectStandardError 'output/host.log'
        $ready = $false
        for ($i=0; $i -lt 100; $i++) {
            if ($children | Where-Object HasExited) { throw 'A host service exited; see output logs.' }
            try { $null = Invoke-RestMethod "$env:BORDER_HOST_URL/health"; $null = Invoke-RestMethod "$env:BORDER_GATEWAY_URL/health"; $ready = $true; break } catch { Start-Sleep -Milliseconds 100 }
        }
        if (!$ready) { throw 'Host services failed to start.' }
        @{url=$env:BORDER_HOST_URL; join_code=$env:BORDER_HOST_JOIN_CODE} | ConvertTo-Json | Set-Content output/local-session.json
        Write-Host 'Second local player: .\play.ps1 --join --name=Officer-2'
        Write-Host "Invite code: $env:BORDER_HOST_JOIN_CODE"
    }
    Remove-Item Env:OPENROUTER_API_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:OPENROUTER_MODEL -ErrorAction SilentlyContinue
    Remove-Item Env:BORDER_SESSION_TOKEN -ErrorAction SilentlyContinue
    Remove-Item Env:BORDER_HOST_OPERATOR_TOKEN -ErrorAction SilentlyContinue
    Remove-Item Env:BORDER_HOST_GATEWAY_TOKEN -ErrorAction SilentlyContinue
    & "$PSScriptRoot/target/debug/border-game.exe" @gameArgs
    $gameExit = $LASTEXITCODE
} finally {
    foreach ($child in $children) { if (!$child.HasExited) { Stop-Process -Id $child.Id } }
    if ($key) { $env:OPENROUTER_API_KEY = $key }
    if ($model) { $env:OPENROUTER_MODEL = $model }
}
exit $gameExit
