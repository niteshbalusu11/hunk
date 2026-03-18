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

function Write-WindowsPathSize {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if (-not (Test-Path $Path)) {
        Write-Host "missing $Path"
        return
    }

    $item = Get-Item $Path
    if ($item.PSIsContainer) {
        $bytes = (Get-ChildItem -Path $Path -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
    } else {
        $bytes = $item.Length
    }

    if ($null -eq $bytes) {
        $bytes = 0
    }

    $sizeMiB = [Math]::Round(($bytes / 1MB), 2)
    Write-Host ("{0} MiB`t{1}" -f $sizeMiB, $Path)
}

function Write-WindowsPackagerInventory {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Label,
        [Parameter(Mandatory = $true)]
        [string]$PackagerOutDir
    )

    Write-Host "Windows package inventory ($Label):"
    Write-WindowsPathSize -Path $PackagerOutDir

    if (-not (Test-Path $PackagerOutDir -PathType Container)) {
        return
    }

    Write-Host "Top-level Windows packager output:"
    $entries = Get-ChildItem -Path $PackagerOutDir -Recurse -ErrorAction SilentlyContinue |
        Sort-Object FullName |
        Select-Object -First 200
    foreach ($entry in $entries) {
        Write-Host "  $($entry.FullName)"
    }

    $exeCount = @(Get-ChildItem -Path $PackagerOutDir -Recurse -File -Filter *.exe -ErrorAction SilentlyContinue).Count
    $dllCount = @(Get-ChildItem -Path $PackagerOutDir -Recurse -File -Filter *.dll -ErrorAction SilentlyContinue).Count
    $msiCount = @(Get-ChildItem -Path $PackagerOutDir -Recurse -File -Filter *.msi -ErrorAction SilentlyContinue).Count
    Write-Host "Windows executables: $exeCount"
    Write-Host "Windows DLLs: $dllCount"
    Write-Host "Windows MSIs: $msiCount"

    $forbiddenMatches = Get-ChildItem -Path $PackagerOutDir -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '(\\|/)(helix|hx-runtime|queries|grammars)(\\|/|$)' } |
        Select-Object -First 50
    if ($forbiddenMatches.Count -gt 0) {
        Write-Host "Forbidden-looking Windows bundle content detected:"
        foreach ($match in $forbiddenMatches) {
            Write-Host "  $($match.FullName)"
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
        [string]$PackagerOutDir
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
$validateBundleScript = Join-Path $PSScriptRoot "validate_windows_release_bundle.ps1"
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

Push-Location $rootDir
$originalCargoTargetDir = $env:CARGO_TARGET_DIR
try {
    $targetDir = (& $resolveTargetDirScript -RootDir $rootDir).Trim()
    $packagerOutDir = Join-Path $targetDir "packager"
    $env:CARGO_TARGET_DIR = $targetDir
    Write-Host "Downloading bundled Codex runtime for Windows..."
    & ./scripts/download_codex_runtime_windows.ps1 | Out-Null
    Write-Host "Validating bundled Codex runtime for Windows..."
    Test-WindowsCodexRuntimeBundle -RootDir $rootDir
    & $validateBundleScript -RootDir $rootDir
    Write-Host "Building Windows release binary..."
    cargo build -p hunk-desktop --release --target $targetTriple --locked
    Write-Host "Building Windows MSI package..."
    Invoke-CargoPackagerWithManifestOverride `
        -CargoTomlPath $cargoTomlPath `
        -CargoLockPath $cargoLockPath `
        -OriginalVersion $versionLabel `
        -WindowsPackagerVersion $windowsPackagerVersion `
        -TargetTriple $targetTriple `
        -PackagerOutDir $packagerOutDir
    Write-WindowsPackagerInventory -Label "after cargo packager" -PackagerOutDir $packagerOutDir
    & $validateBundleScript -RootDir $rootDir -PackagerOutDir $packagerOutDir
} finally {
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
Write-WindowsPathSize -Path $releaseMsiPath

Write-Host "Created Windows release artifact at $releaseMsiPath"

Write-Output $releaseMsiPath
