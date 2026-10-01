param(
    [Parameter(Mandatory = $true)]
    [string]$FfmpegArchive,

    [string]$ApplicationExe = "target/release/rhythm_app.exe",
    [string]$OutputDirectory = "dist"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$manifestPath = Join-Path $repoRoot "packaging/ffmpeg/manifest.json"
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json

function Resolve-RepoPath([string]$PathValue) {
    if ([IO.Path]::IsPathRooted($PathValue)) {
        return (Resolve-Path -LiteralPath $PathValue).Path
    }
    return (Resolve-Path -LiteralPath (Join-Path $repoRoot $PathValue)).Path
}

$appSource = Resolve-RepoPath $ApplicationExe
if (-not [IO.Path]::GetFileName($appSource).Equals("rhythm_app.exe", [StringComparison]::OrdinalIgnoreCase)) {
    throw "ApplicationExe must point to the Cargo rhythm_app.exe release binary."
}

$ffmpegArchivePath = Resolve-RepoPath $FfmpegArchive
$archiveItem = Get-Item -LiteralPath $ffmpegArchivePath
if ($archiveItem.Name -ne [string]$manifest.artifact) {
    throw "FFmpeg archive name mismatch. Expected $($manifest.artifact), got $($archiveItem.Name)."
}
if ($archiveItem.Length -ne [int64]$manifest.artifact_size_bytes) {
    throw "FFmpeg archive size mismatch. Expected $($manifest.artifact_size_bytes), got $($archiveItem.Length)."
}
$actualSha = (Get-FileHash -LiteralPath $ffmpegArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualSha -ne ([string]$manifest.sha256).ToLowerInvariant()) {
    throw "FFmpeg archive SHA-256 mismatch. Expected $($manifest.sha256), got $actualSha."
}

$outputRoot = if ([IO.Path]::IsPathRooted($OutputDirectory)) {
    [IO.Path]::GetFullPath($OutputDirectory)
} else {
    [IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDirectory))
}
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

$version = (Get-Content -LiteralPath (Join-Path $repoRoot "Cargo.toml") -Raw |
    Select-String -Pattern '(?m)^version\s*=\s*"([^"]+)"' -AllMatches).Matches |
    Select-Object -First 1
if ($null -eq $version) {
    throw "Could not determine workspace package version from Cargo.toml."
}
$packageVersion = $version.Groups[1].Value
$packageName = "RhythmEffects-$packageVersion-windows-x86_64"
$stageRoot = Join-Path $outputRoot $packageName
$zipPath = Join-Path $outputRoot "$packageName.zip"

if (Test-Path -LiteralPath $stageRoot) {
    Remove-Item -LiteralPath $stageRoot -Recurse -Force
}
if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}

$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("rhythm-effects-package-" + [Guid]::NewGuid().ToString("N"))
try {
    New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
    Expand-Archive -LiteralPath $ffmpegArchivePath -DestinationPath $tempRoot

    $ffmpegSource = Join-Path $tempRoot ([string]$manifest.source_executable_relative_path)
    if (-not (Test-Path -LiteralPath $ffmpegSource -PathType Leaf)) {
        throw "Pinned FFmpeg executable is missing from archive at $($manifest.source_executable_relative_path)."
    }

    $ffmpegArchiveRoot = Join-Path $tempRoot ([string]$manifest.archive_root)
    $licenseCandidates = @(
        Get-ChildItem -LiteralPath $ffmpegArchiveRoot -File -Recurse |
            Where-Object { $_.Name -match '^(LICENSE|COPYING)(\..*)?$' }
    )
    if ($licenseCandidates.Count -eq 0) {
        throw "Pinned FFmpeg archive contains no LICENSE/COPYING file; refusing to create a release package."
    }

    $licenses = Join-Path $stageRoot "licenses"
    $ffmpegDirectory = Join-Path $stageRoot "ffmpeg"
    New-Item -ItemType Directory -Force -Path $licenses, $ffmpegDirectory | Out-Null

    Copy-Item -LiteralPath $appSource -Destination (Join-Path $stageRoot "RhythmEffects.exe")
    Copy-Item -LiteralPath $ffmpegSource -Destination (Join-Path $ffmpegDirectory "ffmpeg.exe")
    Copy-Item -LiteralPath (Join-Path $repoRoot "packaging/PORTABLE_README.txt") -Destination (Join-Path $stageRoot "README.txt")
    Copy-Item -LiteralPath (Join-Path $repoRoot "packaging/THIRD_PARTY_PROVENANCE.md") -Destination (Join-Path $licenses "THIRD_PARTY_PROVENANCE.md")
    Copy-Item -LiteralPath (Join-Path $repoRoot "packaging/third-party-manifest.json") -Destination (Join-Path $licenses "third-party-manifest.json")
    Copy-Item -LiteralPath $manifestPath -Destination (Join-Path $licenses "ffmpeg-manifest.json")
    Copy-Item -LiteralPath (Join-Path $repoRoot "crates/rhythm_engine/assets/fonts/inter/OFL.txt") -Destination (Join-Path $licenses "Inter-OFL-1.1.txt")

    $ffmpegLicenseRoot = Join-Path $licenses "ffmpeg"
    New-Item -ItemType Directory -Force -Path $ffmpegLicenseRoot | Out-Null
    foreach ($license in $licenseCandidates) {
        $relative = [IO.Path]::GetRelativePath($ffmpegArchiveRoot, $license.FullName)
        $destination = Join-Path $ffmpegLicenseRoot $relative
        New-Item -ItemType Directory -Force -Path ([IO.Path]::GetDirectoryName($destination)) | Out-Null
        Copy-Item -LiteralPath $license.FullName -Destination $destination
    }

    $required = @(
        "RhythmEffects.exe",
        "ffmpeg/ffmpeg.exe",
        "README.txt",
        "licenses/THIRD_PARTY_PROVENANCE.md",
        "licenses/third-party-manifest.json",
        "licenses/ffmpeg-manifest.json",
        "licenses/Inter-OFL-1.1.txt"
    )
    foreach ($relative in $required) {
        $full = Join-Path $stageRoot $relative
        if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
            throw "Portable package is missing required file: $relative"
        }
    }

    Compress-Archive -LiteralPath $stageRoot -DestinationPath $zipPath -CompressionLevel Optimal
    Write-Output $zipPath
}
finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}
