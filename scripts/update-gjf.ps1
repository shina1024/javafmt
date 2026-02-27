param(
    [string]$Version = "latest",
    [string]$OutputDir = "tools/gjf"
)

$ErrorActionPreference = "Stop"

function Resolve-LatestVersion {
    $tag = gh release view --repo google/google-java-format --json tagName --jq ".tagName"
    if (-not $tag) {
        throw "Failed to resolve latest google-java-format release tag."
    }
    if ($tag.StartsWith("v")) {
        return $tag.Substring(1)
    }
    return $tag
}

if ($Version -eq "latest") {
    $resolvedVersion = Resolve-LatestVersion
} else {
    $resolvedVersion = $Version
}

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$jarName = "google-java-format-$resolvedVersion-all-deps.jar"
$jarPath = Join-Path $OutputDir $jarName
$url = "https://github.com/google/google-java-format/releases/download/v$resolvedVersion/$jarName"
$versionFile = Join-Path $OutputDir "version.txt"

Write-Host "Downloading $url"
Invoke-WebRequest -Uri $url -OutFile $jarPath
Set-Content -Path $versionFile -Value $resolvedVersion

Write-Host "Pinned GJF version: $resolvedVersion"
Write-Host "Jar: $jarPath"
