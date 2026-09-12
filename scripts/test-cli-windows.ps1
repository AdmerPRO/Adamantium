$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Invoke-Adamantium {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)

    & adamantium @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "adamantium $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
    }
}

$projectRoot = Join-Path $env:RUNNER_TEMP 'AdamantiumWindowsCliProject'
if (Test-Path -LiteralPath $projectRoot) {
    $resolvedTemp = [System.IO.Path]::GetFullPath($env:RUNNER_TEMP)
    $resolvedProject = [System.IO.Path]::GetFullPath($projectRoot)
    if (-not $resolvedProject.StartsWith($resolvedTemp, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to remove a test project outside RUNNER_TEMP.'
    }
    Remove-Item -LiteralPath $resolvedProject -Recurse -Force
}

Invoke-Adamantium --version
$helpOutput = (& adamantium --help | Out-String)
if ($LASTEXITCODE -ne 0) { throw 'adamantium --help failed.' }
foreach ($command in @(
    'adamantium build [PROJECT_DIRECTORY]',
    'adamantium run [PROJECT_DIRECTORY]',
    'adamantium test run'
)) {
    if (-not $helpOutput.Contains($command)) {
        throw "CLI help does not contain '$command'."
    }
}

Invoke-Adamantium new $projectRoot
foreach ($relativePath in @('project.toml', 'requirement.toml', 'code/main.ad')) {
    if (-not (Test-Path -LiteralPath (Join-Path $projectRoot $relativePath))) {
        throw "adamantium new did not create $relativePath."
    }
}

$mainSource = @'
fun main() {
    print.newline("Windows CLI works");
}
'@
[System.IO.File]::WriteAllText(
    (Join-Path $projectRoot 'code/main.ad'),
    $mainSource,
    [System.Text.UTF8Encoding]::new($false)
)

$testSource = @'
#[test]
fun windows_cli_test() {
    print.newline("Windows test works");
}
'@
[System.IO.File]::WriteAllText(
    (Join-Path $projectRoot 'code/tests.ad'),
    $testSource,
    [System.Text.UTF8Encoding]::new($false)
)

Invoke-Adamantium check $projectRoot
Invoke-Adamantium build $projectRoot

$executable = Join-Path $projectRoot 'target/AdamantiumWindowsCliProject.exe'
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw 'adamantium build did not create the Windows executable.'
}
$nativeOutput = (& $executable | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $nativeOutput -ne 'Windows CLI works') {
    throw "Generated executable failed or returned unexpected output: '$nativeOutput'."
}

$runOutput = (& adamantium run $projectRoot | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $runOutput -ne 'Windows CLI works') {
    throw "adamantium run failed or returned unexpected output: '$runOutput'."
}

$testList = (& adamantium test list $projectRoot | Out-String)
if ($LASTEXITCODE -ne 0 -or $testList -notmatch '(?m)^windows_cli_test\r?$') {
    throw 'adamantium test list did not discover windows_cli_test.'
}
Invoke-Adamantium test run $projectRoot windows_cli_test
Invoke-Adamantium test run $projectRoot

Invoke-Adamantium clean $projectRoot
if (Test-Path -LiteralPath (Join-Path $projectRoot 'target')) {
    throw 'adamantium clean did not remove target.'
}
Invoke-Adamantium build $projectRoot
Invoke-Adamantium clear $projectRoot
if (Test-Path -LiteralPath (Join-Path $projectRoot 'target')) {
    throw 'adamantium clear did not remove target.'
}
