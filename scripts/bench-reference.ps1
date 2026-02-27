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

$javafmtTotal = 0
$gjfTotal = 0
$mismatchTotal = 0

for ($i = 1; $i -le $Runs; $i++) {
    $report = "target/gjf-bench-$i.json"
    if ($GjfJar) {
        cargo run -p gjf-reference -- --gjf-jar $GjfJar --report $report $Inputs
    } else {
        cargo run -p gjf-reference -- --report $report $Inputs
    }

    $json = Get-Content $report -Raw | ConvertFrom-Json
    $javafmtTotal += [int64]$json.javafmt_elapsed_us
    $gjfTotal += [int64]$json.gjf_elapsed_us
    $mismatchTotal += [int64]$json.mismatches
}

$javafmtAvg = [double]$javafmtTotal / $Runs
$gjfAvg = [double]$gjfTotal / $Runs
$ratio = if ($javafmtAvg -gt 0) { $gjfAvg / $javafmtAvg } else { 0.0 }

Write-Host ("runs={0} mismatches_total={1}" -f $Runs, $mismatchTotal)
Write-Host ("avg_javafmt_us={0:N0} avg_gjf_us={1:N0} gjf_over_javafmt_ratio={2:N2}" -f $javafmtAvg, $gjfAvg, $ratio)
