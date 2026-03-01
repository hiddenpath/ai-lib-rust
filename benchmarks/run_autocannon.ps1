# Run autocannon and append JSON output locally.
param(
  [string]$ConfigPath = "benchmark_config.json"
)
$OutputPath = Join-Path (Split-Path $PSScriptRoot -Parent) "outputs/benchmark_results.json"

if (!(Test-Path $ConfigPath)) { Write-Error "Config $ConfigPath not found"; exit 1 }
$config = Get-Content $ConfigPath -Raw | ConvertFrom-Json

if (-not (Get-Command autocannon -ErrorAction SilentlyContinue)) {
  Write-Error "autocannon not installed. Run: npm install -g autocannon"; exit 1
}

# Try to inject API key from env vars
$envNames = @('AI_API_KEY','OPENAI_API_KEY','API_KEY','OPENAI_KEY')
$apiKey = $null
foreach ($n in $envNames) { 
  $val = [Environment]::GetEnvironmentVariable($n)
  if ($val) { $apiKey = $val; break } 
}
if ($apiKey) { $config.headers.Authorization = "Bearer $apiKey" }

$target = [string]$config.targetUrl
$duration = [int]$config.durationSeconds
$connections = [int]$config.connections
$pipelining = [int]$config.pipelining
$body = $config.payload | ConvertTo-Json -Compress
$headers = @()
if ($config.headers) {
  foreach ($prop in $config.headers.PSObject.Properties) {
    $headers += @("-H", "$($prop.Name): $($prop.Value)")
  }
}

Write-Output "Running: $target ($($duration)s, $($connections) conns)"

$args = @("-d", "$duration", "-c", "$connections", "-p", "$pipelining", "--method", "POST", "--body", $body, "--json")
$args += $headers
$args += $target
$raw = & autocannon @args 2>$null | Out-String

try { $obj = $raw | ConvertFrom-Json } catch { Write-Error "Failed to parse autocannon JSON output"; exit 1 }

# Build run record
$repo = (Split-Path (Get-Location) -Parent | Split-Path -Leaf)
$runRecord = [PSCustomObject]@{
  repo = $repo
  timestamp = (Get-Date).ToString("o")
  target = $target
  duration = $duration
  connections = $connections
  result = $obj
}

# Append to local outputs
if (!(Test-Path (Split-Path $OutputPath))) {
  New-Item -ItemType Directory -Path (Split-Path $OutputPath) | Out-Null
}
if (Test-Path $OutputPath) {
  try {
    $existing = Get-Content $OutputPath -Raw | ConvertFrom-Json
    if ($existing -is [System.Object[]]) {
      $arr = @($existing)
    } else {
      $arr = @()
    }
    $arr += $runRecord
    $arr | ConvertTo-Json -Depth 15 | Set-Content $OutputPath
  } catch {
    Write-Error "Failed to append to $OutputPath : $_"; exit 1
  }
} else {
  @($runRecord) | ConvertTo-Json -Depth 15 | Set-Content $OutputPath
}

Write-Output "Saved to $OutputPath"
Write-Output "  RPS: $(if($obj.requests.average) { '{0:F2}' -f $obj.requests.average } else { 'N/A' })"
Write-Output "  Latency p99: $(if($obj.latency.p99) { '{0:F2}ms' -f $obj.latency.p99 } else { 'N/A' })"
