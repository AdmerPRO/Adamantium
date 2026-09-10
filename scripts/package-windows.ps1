param(
    [string]$NasmVersion = "3.02",
    [string]$NasmSha256 = "161D0BFAFF53C2F9E9F3E69FD0672323EBABAFD1268976A5CEC11BE92A19AEE7",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$compilerRoot = Split-Path -Parent $PSScriptRoot
$distributionRoot = Join-Path $compilerRoot "dist"
$packageRoot = Join-Path $distributionRoot "adamantium-windows-x86_64"
$archivePath = Join-Path $distributionRoot "nasm-$NasmVersion-win64.zip"
$extractedPath = Join-Path $distributionRoot "nasm-$NasmVersion"

if (-not $SkipBuild) {
    $env:RUSTFLAGS = "-C target-feature=+crt-static"
    cargo build --manifest-path (Join-Path $compilerRoot "Cargo.toml") --locked --release
    if ($LASTEXITCODE -ne 0) { throw "Cargo release build failed." }
}

New-Item -ItemType Directory -Force -Path $distributionRoot | Out-Null
Invoke-WebRequest `
    -Uri "https://www.nasm.us/pub/nasm/releasebuilds/$NasmVersion/win64/nasm-$NasmVersion-win64.zip" `
    -OutFile $archivePath
$actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash
if ($actualHash -ne $NasmSha256) {
    throw "NASM archive checksum mismatch: expected $NasmSha256, found $actualHash."
}

if (Test-Path -LiteralPath $extractedPath) {
    Remove-Item -LiteralPath $extractedPath -Recurse -Force
}
Expand-Archive -LiteralPath $archivePath -DestinationPath $extractedPath
if (Test-Path -LiteralPath $packageRoot) {
    Remove-Item -LiteralPath $packageRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path (Join-Path $packageRoot "tools") | Out-Null
Copy-Item -LiteralPath (Join-Path $compilerRoot "target/release/adamantium.exe") -Destination $packageRoot
$nasmExecutable = Get-ChildItem -LiteralPath $extractedPath -Filter "nasm.exe" -Recurse | Select-Object -First 1
if (-not $nasmExecutable) { throw "The NASM archive does not contain nasm.exe." }
Copy-Item -LiteralPath $nasmExecutable.FullName -Destination (Join-Path $packageRoot "tools/nasm.exe")
Copy-Item -LiteralPath (Join-Path $compilerRoot "THIRD_PARTY_LICENSES/NASM.txt") -Destination $packageRoot
Copy-Item -LiteralPath (Join-Path $compilerRoot "README.md") -Destination $packageRoot

$packageArchive = "$packageRoot.zip"
if (Test-Path -LiteralPath $packageArchive) {
    Remove-Item -LiteralPath $packageArchive -Force
}
Compress-Archive -LiteralPath $packageRoot -DestinationPath $packageArchive
Write-Output "Created $packageArchive"
