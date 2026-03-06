param(
    [string]$Version = "",
    [string]$OutputDir = "fixtures/upstream-gjf"
)

$ErrorActionPreference = "Stop"

if (-not $Version) {
    $versionFile = "tools/gjf/version.txt"
    if (-not (Test-Path $versionFile)) {
        throw "Missing $versionFile."
    }
    $Version = (Get-Content $versionFile).Trim()
}

if (-not $Version -or $Version -eq "latest") {
    throw "Resolved GJF version must be pinned before syncing upstream suite."
}

$tag = "v$Version"
$root = Join-Path $OutputDir $Version
New-Item -ItemType Directory -Path $root -Force | Out-Null

function Get-ContentEntries([string]$RepoPath) {
    $encodedPath = $RepoPath.Replace("\", "/")
    $response = gh api "repos/google/google-java-format/contents/${encodedPath}?ref=$tag"
    return $response | ConvertFrom-Json
}

function Download-File([string]$Url, [string]$Destination) {
    $parent = Split-Path -Parent $Destination
    if ($parent) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    Invoke-WebRequest -Uri $Url -OutFile $Destination
}

$licenseUrl = gh api "repos/google/google-java-format/contents/LICENSE?ref=$tag" --jq '.download_url'
Download-File $licenseUrl (Join-Path $root "LICENSE")

$suiteDefs = @(
    @{
        Id = "testdata"
        Kind = "full_format"
        Note = "FormatterIntegrationTest full-file formatting corpus from pinned upstream tag."
        RepoPath = "core/src/test/resources/com/google/googlejavaformat/java/testdata"
    },
    @{
        Id = "testjavadoc"
        Kind = "full_format"
        Note = "JavadocFormattingTest corpus from pinned upstream tag."
        RepoPath = "core/src/test/resources/com/google/googlejavaformat/java/testjavadoc"
    },
    @{
        Id = "testimports"
        Kind = "asset_only"
        Note = "Upstream import-order/import-mode assets are vendored for future dedicated runners."
        RepoPath = "core/src/test/resources/com/google/googlejavaformat/java/testimports"
    }
)

$manifestSuites = @()
foreach ($suiteDef in $suiteDefs) {
    $entries = Get-ContentEntries $suiteDef.RepoPath
    $localDir = Join-Path $root $suiteDef.Id
    New-Item -ItemType Directory -Path $localDir -Force | Out-Null

    foreach ($entry in $entries) {
        $destination = Join-Path $localDir $entry.name
        Download-File $entry.download_url $destination
    }

    if ($suiteDef.Kind -eq "full_format") {
        $names = $entries | ForEach-Object { $_.name }
        $cases = @()
        foreach ($inputName in ($names | Where-Object { $_ -like "*.input" } | Sort-Object)) {
            $baseName = [System.IO.Path]::GetFileNameWithoutExtension($inputName)
            $outputName = "$baseName.output"
            if ($names -notcontains $outputName) {
                throw "Missing output pair for $($suiteDef.Id)/$inputName"
            }
            $cases += @{
                name = $baseName
                input = "$($suiteDef.Id)/$inputName"
                expected_output = "$($suiteDef.Id)/$outputName"
            }
        }

        $manifestSuites += @{
            id = $suiteDef.Id
            kind = $suiteDef.Kind
            note = $suiteDef.Note
            cases = $cases
        }
    } else {
        $manifestSuites += @{
            id = $suiteDef.Id
            kind = $suiteDef.Kind
            note = $suiteDef.Note
            assets = @($entries | ForEach-Object { "$($suiteDef.Id)/$($_.name)" } | Sort-Object)
        }
    }
}

$manifest = @{
    version = $Version
    repository = "google/google-java-format"
    tag = $tag
    suites = $manifestSuites
}

$manifestPath = Join-Path $root "manifest.json"
$manifest | ConvertTo-Json -Depth 8 | Set-Content -Path $manifestPath

Write-Host "Synced GJF upstream suite:"
Write-Host "  version: $Version"
Write-Host "  root: $root"
Write-Host "  manifest: $manifestPath"
