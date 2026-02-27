param(
    [int]$Runs = 5,
    [string]$GjfJar = "",
    [Parameter(Mandatory = $true)]
    [string[]]$Inputs
)

$ErrorActionPreference = "Stop"
if ($Runs -lt 1) {
    throw "-Runs must be >= 1"
}

New-Item -ItemType Directory -Path "target" -Force | Out-Null
$report = "target/gjf-bench.json"

if ($GjfJar) {
    cargo run -p gjf-reference -- --gjf-jar $GjfJar --runs $Runs --report $report $Inputs
} else {
    cargo run -p gjf-reference -- --runs $Runs --report $report $Inputs
}

$json = Get-Content $report -Raw | ConvertFrom-Json
Write-Host ("runs={0} files={1} comparisons={2} mismatches={3}" -f $json.runs, $json.files, $json.comparisons, $json.mismatches)
Write-Host ("javafmt_us={0} gjf_us={1} ratio={2}" -f $json.javafmt_elapsed_us, $json.gjf_elapsed_us, $json.gjf_over_javafmt_ratio)
