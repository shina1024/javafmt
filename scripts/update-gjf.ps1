param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$url = "https://github.com/google/google-java-format/releases/download/v$Version/google-java-format-$Version-all-deps.jar"
Write-Host "Download URL: $url"
Write-Host "Download and pin flow will be implemented in Phase 0."
