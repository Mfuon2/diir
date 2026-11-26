param([Parameter(Mandatory=$true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$required = 'WINDOWS_CERTIFICATE_PFX_BASE64', 'WINDOWS_CERTIFICATE_PASSWORD', 'WINDOWS_SIGNER_THUMBPRINT', 'RUNNER_TEMP'
foreach ($name in $required) {
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) { throw "Missing signing configuration: $name" }
}
if (-not (Test-Path -LiteralPath $Executable -PathType Leaf)) { throw 'Missing staged executable' }
$config = python scripts/package.py config | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Cannot read packaging configuration' }
$timestamp = $config.signing.windows_timestamp_url
if ([string]::IsNullOrWhiteSpace($timestamp)) { throw 'Missing Windows timestamp service configuration' }
$tools = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" | Sort-Object FullName -Descending
if (-not $tools) { throw 'Windows SDK SignTool was not found' }
$signTool = $tools[0].FullName
$directory = Join-Path $env:RUNNER_TEMP ([Guid]::NewGuid().ToString())
$certificate = $null
$imported = @()
try {
    New-Item -ItemType Directory -Path $directory | Out-Null
    $pfx = Join-Path $directory 'developer.pfx'
    [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE_PFX_BASE64))
    $password = ConvertTo-SecureString $env:WINDOWS_CERTIFICATE_PASSWORD -AsPlainText -Force
    $imported = @(Import-PfxCertificate -FilePath $pfx -CertStoreLocation Cert:\CurrentUser\My -Password $password)
    $expected = $env:WINDOWS_SIGNER_THUMBPRINT.Replace(' ', '').ToUpperInvariant()
    $certificate = $imported | Where-Object { $_.HasPrivateKey -and $_.Thumbprint -eq $expected } | Select-Object -First 1
    if (-not $certificate) { throw 'PFX did not contain the expected signer with a private key' }
    & $signTool sign /sha1 $expected /s My /fd SHA256 /tr $timestamp /td SHA256 $Executable
    if ($LASTEXITCODE -ne 0) { throw 'Authenticode signing failed' }
    & $signTool verify /pa /all /v $Executable
    if ($LASTEXITCODE -ne 0) { throw 'Authenticode verification failed' }
    $signature = Get-AuthenticodeSignature -LiteralPath $Executable
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $expected -or -not $signature.TimeStamperCertificate) {
        throw 'Expected a valid, timestamped signature from the configured signer'
    }
} finally {
    foreach ($item in $imported) {
        Remove-Item -LiteralPath "Cert:\CurrentUser\My\$($item.Thumbprint)" -DeleteKey -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $directory) { Remove-Item -LiteralPath $directory -Recurse -Force }
}
