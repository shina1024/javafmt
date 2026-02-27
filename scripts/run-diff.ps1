param(
    [string]$GjfJar = "",
    [Parameter(Mandatory = $true)]
    [string[]]$Inputs
)

$resolvedJar = $GjfJar
if (-not $resolvedJar) {
    $versionFile = "tools/gjf/version.txt"
    if (-not (Test-Path $versionFile)) {
        throw "Missing $versionFile. Run scripts/update-gjf.ps1 first or pass -GjfJar."
    }
    $version = (Get-Content $versionFile -Raw).Trim()
    if (-not $version -or $version -eq "latest") {
        throw "tools/gjf/version.txt must contain a resolved version. Run scripts/update-gjf.ps1."
    }
    $resolvedJar = "tools/gjf/google-java-format-$version-all-deps.jar"
}

if (-not (Test-Path $resolvedJar)) {
    throw "GJF jar not found: $resolvedJar"
}

cargo run -p gjf-reference -- --gjf-jar $resolvedJar $Inputs
