param(
  [Parameter(Mandatory = $true, Position = 0)]
  [ValidateSet("running", "completed", "failed")]
  [string]$Phase,

  [string]$StatusPath = "",

  [string]$VerificationPath = ""
)

$ErrorActionPreference = "Stop"

function Get-DefaultStatusPath {
  if ($env:CODEX_BEACON_STATUS_PATH) {
    return $env:CODEX_BEACON_STATUS_PATH
  }

  if ($env:APPDATA) {
    return Join-Path $env:APPDATA "app.codexbeacon.desktop\codex-status.json"
  }

  return Join-Path $env:LOCALAPPDATA "app.codexbeacon.desktop\codex-status.json"
}

function Get-PayloadValue {
  param(
    [object]$Payload,
    [string]$Name
  )

  if ($null -ne $Payload -and $Payload.PSObject.Properties.Name -contains $Name) {
    return [string]$Payload.$Name
  }

  return ""
}

function Convert-ToSessionTable {
  param([object]$Source)

  $session = [ordered]@{
    sessionId = Get-PayloadValue -Payload $Source -Name "sessionId"
    turnId = Get-PayloadValue -Payload $Source -Name "turnId"
    phase = Get-PayloadValue -Payload $Source -Name "phase"
    prompt = Get-PayloadValue -Payload $Source -Name "prompt"
    cwd = Get-PayloadValue -Payload $Source -Name "cwd"
    model = Get-PayloadValue -Payload $Source -Name "model"
    lastAssistantMessage = Get-PayloadValue -Payload $Source -Name "lastAssistantMessage"
    startedAt = 0
    updatedAt = 0
  }

  if ($null -ne $Source -and $Source.PSObject.Properties.Name -contains "startedAt") {
    $session.startedAt = [long]$Source.startedAt
  }
  if ($null -ne $Source -and $Source.PSObject.Properties.Name -contains "updatedAt") {
    $session.updatedAt = [long]$Source.updatedAt
  }

  return $session
}

if (-not $StatusPath) {
  $StatusPath = Get-DefaultStatusPath
}
if (-not $VerificationPath) {
  $VerificationPath = Join-Path (
    Split-Path -Parent $StatusPath
  ) "codex-hook-verification.json"
}

$payload = $null
$standardInput = [Console]::OpenStandardInput()
$inputReader = New-Object System.IO.StreamReader(
  $standardInput,
  [System.Text.UTF8Encoding]::new($false),
  $true
)
$rawInput = $inputReader.ReadToEnd()
$inputReader.Dispose()
if ($rawInput.Trim()) {
  try {
    $payload = $rawInput | ConvertFrom-Json
  } catch {
    $payload = $null
  }
}

$now = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$sessionId = Get-PayloadValue -Payload $payload -Name "session_id"
$turnId = Get-PayloadValue -Payload $payload -Name "turn_id"
if (-not $sessionId) {
  $sessionId = "codex-local"
}
if (-not $turnId) {
  $turnId = "turn-$now"
}

$mutex = New-Object System.Threading.Mutex($false, "Local\CodexBeacon.CodexStatus")
$hasLock = $false

try {
  $hasLock = $mutex.WaitOne([TimeSpan]::FromSeconds(5))
  if (-not $hasLock) {
    throw "Timed out waiting for the Codex Beacon status lock."
  }

  $sessions = @()
  if (Test-Path -LiteralPath $StatusPath) {
    try {
      $existing = Get-Content -LiteralPath $StatusPath -Raw -Encoding UTF8 | ConvertFrom-Json
      if ($null -ne $existing.sessions) {
        $sessions = @($existing.sessions | ForEach-Object { Convert-ToSessionTable -Source $_ })
      }
    } catch {
      $sessions = @()
    }
  }

  $matchIndex = -1
  for ($index = 0; $index -lt $sessions.Count; $index++) {
    if (
      [string]$sessions[$index].sessionId -eq $sessionId -and
      [string]$sessions[$index].turnId -eq $turnId
    ) {
      $matchIndex = $index
      break
    }
  }

  $current = if ($matchIndex -ge 0) {
    $sessions[$matchIndex]
  } else {
    Convert-ToSessionTable -Source $null
  }

  if ($Phase -eq "running") {
    $current = [ordered]@{
      sessionId = $sessionId
      turnId = $turnId
      phase = "running"
      prompt = Get-PayloadValue -Payload $payload -Name "prompt"
      cwd = Get-PayloadValue -Payload $payload -Name "cwd"
      model = Get-PayloadValue -Payload $payload -Name "model"
      lastAssistantMessage = ""
      startedAt = $now
      updatedAt = $now
    }
  } else {
    $current.sessionId = $sessionId
    $current.turnId = $turnId
    $current.phase = $Phase
    $current.updatedAt = $now
    if ([long]$current.startedAt -le 0) {
      $current.startedAt = $now
    }

    $cwd = Get-PayloadValue -Payload $payload -Name "cwd"
    $model = Get-PayloadValue -Payload $payload -Name "model"
    $assistantMessage = Get-PayloadValue -Payload $payload -Name "last_assistant_message"
    if ($cwd) {
      $current.cwd = $cwd
    }
    if ($model) {
      $current.model = $model
    }
    if ($assistantMessage) {
      $current.lastAssistantMessage = $assistantMessage
    }
  }

  if ($matchIndex -ge 0) {
    $sessions[$matchIndex] = $current
  } else {
    $sessions += $current
  }

  $sessions = @(
    $sessions |
      Sort-Object -Property @{ Expression = { [long]$_.updatedAt }; Descending = $true } |
      Select-Object -First 6
  )

  $state = [ordered]@{
    sessions = $sessions
    updatedAt = $now
  }

  $statusDirectory = Split-Path -Parent $StatusPath
  New-Item -ItemType Directory -Force -Path $statusDirectory | Out-Null
  $temporaryPath = "$StatusPath.$PID.tmp"
  $json = $state | ConvertTo-Json -Depth 6
  [System.IO.File]::WriteAllText(
    $temporaryPath,
    $json,
    [System.Text.UTF8Encoding]::new($false)
  )
  Move-Item -LiteralPath $temporaryPath -Destination $StatusPath -Force

  $verification = [ordered]@{
    verifiedAt = $now
    lastPhase = $Phase
  }
  $verificationTemporaryPath = "$VerificationPath.$PID.tmp"
  [System.IO.File]::WriteAllText(
    $verificationTemporaryPath,
    ($verification | ConvertTo-Json),
    [System.Text.UTF8Encoding]::new($false)
  )
  Move-Item `
    -LiteralPath $verificationTemporaryPath `
    -Destination $VerificationPath `
    -Force
} finally {
  if ($hasLock) {
    $mutex.ReleaseMutex() | Out-Null
  }
  $mutex.Dispose()
}
