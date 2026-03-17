param()

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $true
}

function Get-WindowsPackagerVersion {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Version
    )

    $versionPattern = '^(?<base>\d+\.\d+\.\d+)(?:[-+].*)?$'
    if ($Version -notmatch $versionPattern) {
        throw "Unsupported Hunk version '$Version'. Expected semver 'major.minor.patch[-prerelease][+build]'."
    }

    return $Matches["base"]
}

function Get-HelixGitRevision {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CargoTomlPath
    )

    $cargoToml = Get-Content $CargoTomlPath -Raw
    $revisionMatch = [regex]::Match(
        $cargoToml,
        '(?m)^helix-core\s*=\s*\{[^}]*\brev\s*=\s*"(?<revision>[0-9a-f]{7,40})"'
    )
    if (-not $revisionMatch.Success) {
        throw "Failed to resolve Helix git revision from $CargoTomlPath"
    }

    return $revisionMatch.Groups["revision"].Value
}

function Download-HelixRuntimeSourceDir {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Revision
    )

    $downloadRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("hunk-helix-source-" + [System.Guid]::NewGuid().ToString("N"))
    $zipPath = Join-Path $downloadRoot "helix.zip"
    $extractDir = Join-Path $downloadRoot "extract"
    $archiveUrl = "https://github.com/helix-editor/helix/archive/$Revision.zip"

    New-Item -ItemType Directory -Path $downloadRoot -Force | Out-Null
    New-Item -ItemType Directory -Path $extractDir -Force | Out-Null

    Write-Host "Downloading Helix runtime source archive for revision $Revision..."
    Invoke-WebRequest -Uri $archiveUrl -OutFile $zipPath
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

    $runtimeDir = Get-ChildItem -Path $extractDir -Directory | Where-Object {
        Test-Path (Join-Path $_.FullName "runtime")
    } | Select-Object -First 1
    if (-not $runtimeDir) {
        throw "Downloaded Helix archive for revision $Revision did not contain a runtime/ directory"
    }

    return [pscustomobject]@{
        Path        = (Resolve-Path (Join-Path $runtimeDir.FullName "runtime")).Path
        CleanupRoot = $downloadRoot
        Source      = "download"
    }
}

function Resolve-HelixRuntimeSourceDir {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CargoTomlPath
    )

    $cargoHome = if ($env:CARGO_HOME) {
        $env:CARGO_HOME
    } else {
        Join-Path $HOME ".cargo"
    }

    $preferredRevision = Get-HelixGitRevision -CargoTomlPath $CargoTomlPath
    $preferredRevisionPrefix = $preferredRevision.Substring(0, [Math]::Min(7, $preferredRevision.Length))
    $checkoutsDir = Join-Path $cargoHome "git/checkouts"
    if (Test-Path $checkoutsDir) {
        $helixRepos = Get-ChildItem -Path $checkoutsDir -Directory -Filter "helix-*"
        foreach ($repo in $helixRepos) {
            $preferredRuntime = Join-Path $repo.FullName "$preferredRevisionPrefix/runtime"
            if (Test-Path $preferredRuntime) {
                return [pscustomobject]@{
                    Path        = (Resolve-Path $preferredRuntime).Path
                    CleanupRoot = $null
                    Source      = "cargo-git-checkout"
                }
            }
        }

        foreach ($repo in $helixRepos) {
            $runtimeDir = Get-ChildItem -Path $repo.FullName -Directory | Where-Object {
                Test-Path (Join-Path $_.FullName "runtime")
            } | Select-Object -First 1
            if ($runtimeDir) {
                return [pscustomobject]@{
                    Path        = (Resolve-Path (Join-Path $runtimeDir.FullName "runtime")).Path
                    CleanupRoot = $null
                    Source      = "cargo-git-checkout"
                }
            }
        }
    }

    Write-Host "Helix runtime was not found under $checkoutsDir; falling back to the Helix source archive for revision $preferredRevision"
    return Download-HelixRuntimeSourceDir -Revision $preferredRevision
}

function Escape-TomlString {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Value
    )

    return ($Value -replace '"', '\"')
}

function New-StagedHelixRuntimeDir {
    param(
        [Parameter(Mandatory = $true)]
        [string]$HelixRuntimeSourceDir
    )

    $stagingRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("hunk-helix-runtime-" + [System.Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $stagingRoot -Force | Out-Null
    Copy-Item -Path $HelixRuntimeSourceDir -Destination $stagingRoot -Recurse

    $stagedRuntimeDir = Join-Path $stagingRoot (Split-Path $HelixRuntimeSourceDir -Leaf)
    $grammarSourcesDir = Join-Path $stagedRuntimeDir "grammars/sources"
    if (Test-Path $grammarSourcesDir) {
        Remove-Item -Path $grammarSourcesDir -Recurse -Force
    }

    if (-not (Test-Path (Join-Path $stagedRuntimeDir "queries")) -or -not (Test-Path (Join-Path $stagedRuntimeDir "grammars"))) {
        throw "Staged Helix runtime is missing queries/ or grammars/: $stagedRuntimeDir"
    }

    return $stagedRuntimeDir
}

function Test-WindowsCodexRuntimeBundle {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RootDir
    )

    $runtimeDir = Join-Path $RootDir "assets/codex-runtime/windows"
    if (-not (Test-Path $runtimeDir -PathType Container)) {
        throw "Missing Windows Codex runtime directory: $runtimeDir"
    }

    foreach ($fileName in @("codex.cmd", "codex.exe")) {
        $filePath = Join-Path $runtimeDir $fileName
        if (-not (Test-Path $filePath -PathType Leaf)) {
            throw "Missing Windows Codex runtime file: $filePath"
        }
    }
}

function Invoke-CargoPackagerWithManifestOverride {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CargoTomlPath,
        [Parameter(Mandatory = $true)]
        [string]$CargoLockPath,
        [Parameter(Mandatory = $true)]
        [string]$OriginalVersion,
        [Parameter(Mandatory = $true)]
        [string]$WindowsPackagerVersion,
        [Parameter(Mandatory = $true)]
        [string]$TargetTriple,
        [Parameter(Mandatory = $true)]
        [string]$PackagerOutDir,
        [Parameter(Mandatory = $true)]
        [string]$HelixRuntimeSourceDir
    )

    $originalCargoToml = Get-Content $CargoTomlPath -Raw
    $updatedCargoToml = $originalCargoToml
    if ($WindowsPackagerVersion -ne $OriginalVersion) {
        $updatedCargoToml = [regex]::Replace(
            $updatedCargoToml,
            '(?ms)^(\[package\]\s.*?^version = ")([^"]+)(")',
            ('${1}' + $WindowsPackagerVersion + '${3}'),
            1
        )

        if ($updatedCargoToml -eq $originalCargoToml) {
            throw "Failed to rewrite [package] version in $CargoTomlPath"
        }
    }

    $escapedHelixRuntimeSourceDir = Escape-TomlString (($HelixRuntimeSourceDir -replace '\\', '/'))
    $resourceBlock = @"
resources = [
  "../../assets/codex-runtime",
  { src = "$escapedHelixRuntimeSourceDir", target = "runtime" },
]
"@
    $tomlBeforeResourceRewrite = $updatedCargoToml
    $updatedCargoToml = [regex]::Replace(
        $updatedCargoToml,
        '(?m)^\s*resources\s*=\s*\[\s*"\.\./\.\./assets/codex-runtime"\s*\]\s*\r?$',
        $resourceBlock,
        1
    )
    if ($updatedCargoToml -eq $tomlBeforeResourceRewrite) {
        throw "Failed to rewrite [package.metadata.packager] resources in $CargoTomlPath"
    }

    $originalCargoLockBytes = $null
    $cargoLockExisted = Test-Path $CargoLockPath
    if ($cargoLockExisted) {
        $originalCargoLockBytes = [System.IO.File]::ReadAllBytes($CargoLockPath)
    }

    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    try {
        [System.IO.File]::WriteAllText($CargoTomlPath, $updatedCargoToml, $utf8NoBom)
        if ($WindowsPackagerVersion -ne $OriginalVersion) {
            Write-Host "Using Windows packager version $WindowsPackagerVersion for Cargo version $OriginalVersion"
        }
        cargo packager -p hunk-desktop --release -f wix --target $TargetTriple --out-dir $PackagerOutDir
    } finally {
        [System.IO.File]::WriteAllText($CargoTomlPath, $originalCargoToml, $utf8NoBom)
        if ($cargoLockExisted) {
            [System.IO.File]::WriteAllBytes($CargoLockPath, $originalCargoLockBytes)
        } elseif (Test-Path $CargoLockPath) {
            Remove-Item -Path $CargoLockPath -Force -ErrorAction SilentlyContinue
        }
    }
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$resolveTargetDirScript = Join-Path $PSScriptRoot "resolve_cargo_target_dir.ps1"
$cargoTomlPath = Join-Path $rootDir "crates/hunk-desktop/Cargo.toml"
$cargoLockPath = Join-Path $rootDir "Cargo.lock"
$targetTriple = "x86_64-pc-windows-msvc"
$versionLabel = if ($env:HUNK_RELEASE_VERSION) {
    $env:HUNK_RELEASE_VERSION
} else {
    $versionLine = Get-Content $cargoTomlPath | Select-String '^version = "' | Select-Object -First 1
    if (-not $versionLine) {
        throw "Failed to resolve Hunk version from $cargoTomlPath"
    }
    [regex]::Match($versionLine.Line, '^version = "(.*)"$').Groups[1].Value
}
$windowsPackagerVersion = Get-WindowsPackagerVersion -Version $versionLabel
$helixRuntimeSource = Resolve-HelixRuntimeSourceDir -CargoTomlPath $cargoTomlPath
$stagedHelixRuntimeDir = $null
$downloadedHelixRuntimeRoot = $helixRuntimeSource.CleanupRoot

Push-Location $rootDir
$originalCargoTargetDir = $env:CARGO_TARGET_DIR
try {
    $targetDir = (& $resolveTargetDirScript -RootDir $rootDir).Trim()
    $packagerOutDir = Join-Path $targetDir "packager"
    Write-Host "Using Helix runtime source from $($helixRuntimeSource.Source): $($helixRuntimeSource.Path)"
    $stagedHelixRuntimeDir = New-StagedHelixRuntimeDir -HelixRuntimeSourceDir $helixRuntimeSource.Path
    $env:CARGO_TARGET_DIR = $targetDir
    Write-Host "Downloading bundled Codex runtime for Windows..."
    & ./scripts/download_codex_runtime_windows.ps1 | Out-Null
    Write-Host "Validating bundled Codex runtime for Windows..."
    Test-WindowsCodexRuntimeBundle -RootDir $rootDir
    Write-Host "Building Windows release binary..."
    cargo build -p hunk-desktop --release --target $targetTriple --locked
    Write-Host "Building Windows MSI package..."
    Invoke-CargoPackagerWithManifestOverride `
        -CargoTomlPath $cargoTomlPath `
        -CargoLockPath $cargoLockPath `
        -OriginalVersion $versionLabel `
        -WindowsPackagerVersion $windowsPackagerVersion `
        -TargetTriple $targetTriple `
        -PackagerOutDir $packagerOutDir `
        -HelixRuntimeSourceDir $stagedHelixRuntimeDir
} finally {
    if ($stagedHelixRuntimeDir) {
        Remove-Item -Path (Split-Path $stagedHelixRuntimeDir -Parent) -Recurse -Force -ErrorAction SilentlyContinue
    }
    if ($downloadedHelixRuntimeRoot) {
        Remove-Item -Path $downloadedHelixRuntimeRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
    if ($null -eq $originalCargoTargetDir) {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_TARGET_DIR = $originalCargoTargetDir
    }
    Pop-Location
}

$distDir = Join-Path $targetDir "dist"
$bundleMsi = Get-ChildItem -Path $packagerOutDir -Filter "*.msi" | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
$releaseMsiPath = Join-Path $distDir "Hunk-$versionLabel-windows-x86_64.msi"

if (-not $bundleMsi) {
    if (Test-Path $packagerOutDir) {
        Write-Host "Packager output under ${packagerOutDir}:"
        Get-ChildItem -Path $packagerOutDir -Recurse | ForEach-Object {
            Write-Host " - $($_.FullName)"
        }
    }
    throw "Expected cargo-packager to produce an MSI under $packagerOutDir"
}

New-Item -ItemType Directory -Path $distDir -Force | Out-Null
Copy-Item -Path $bundleMsi.FullName -Destination $releaseMsiPath -Force

Write-Host "Created Windows release artifact at $releaseMsiPath"

Write-Output $releaseMsiPath
