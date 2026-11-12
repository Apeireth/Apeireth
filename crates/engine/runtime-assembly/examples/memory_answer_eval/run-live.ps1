param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$Endpoint,
    [Parameter(Mandatory = $true)][string]$Model,
    [Parameter(Mandatory = $true)][double]$Temperature,
    [Parameter(Mandatory = $true)][int]$MaxOutputTokens,
    [Parameter(Mandatory = $true)][int]$MaxCalls,
    [Parameter(Mandatory = $true)][int]$Repetitions,
    [Parameter(Mandatory = $true)][string]$Output,
    [Parameter(Mandatory = $true)][string]$Cases,
    [Parameter(Mandatory = $true)][ValidateSet('omit','enabled','disabled')][string]$Thinking,
    [Parameter(Mandatory = $true)][ValidateSet('omit','none','low','high','max')][string]$ReasoningEffort,
    [int]$MaxRequestBytes = 24576,
    [int]$TimeoutMs = 60000
)

# Invoke this locally in a terminal. The secret is neither an argument nor a file.
# No existing credential or parent/global environment value is read or changed.
$ErrorActionPreference = 'Stop'
$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
if (-not (Test-Path -LiteralPath $resolvedExecutable -PathType Leaf)) {
    throw 'Executable must name the compiled evaluation example.'
}
$temporarySecret = Read-Host 'Temporary evaluation API key (hidden)' -AsSecureString
$secretPointer = [IntPtr]::Zero
$startInfo = $null
$child = $null
try {
    $secretPointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($temporarySecret)
    $startInfo = [Diagnostics.ProcessStartInfo]::new($resolvedExecutable)
    $startInfo.UseShellExecute = $false
    $startInfo.Environment['APEIRETH_EVAL_TEMP_API_KEY'] = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($secretPointer)
    $arguments = @('run-live', '--provider', 'compatible', '--endpoint', $Endpoint,
        '--model', $Model, '--key-env', 'APEIRETH_EVAL_TEMP_API_KEY',
        '--temperature', $Temperature.ToString([Globalization.CultureInfo]::InvariantCulture),
        '--max-output-tokens', $MaxOutputTokens.ToString(), '--max-calls', $MaxCalls.ToString(),
        '--repetitions', $Repetitions.ToString(), '--output', $Output, '--cases', $Cases,
        '--thinking', $Thinking, '--reasoning-effort', $ReasoningEffort,
        '--max-request-bytes', $MaxRequestBytes.ToString(), '--timeout-ms', $TimeoutMs.ToString())
    foreach ($argument in $arguments) { $startInfo.ArgumentList.Add($argument) }
    $child = [Diagnostics.Process]::Start($startInfo)
    if ($null -eq $child) { throw 'Evaluation child process did not start.' }
    $startInfo.Environment.Remove('APEIRETH_EVAL_TEMP_API_KEY') | Out-Null
    $child.WaitForExit()
    $resultCode = $child.ExitCode
} finally {
    if ($null -ne $startInfo) { $startInfo.Environment.Remove('APEIRETH_EVAL_TEMP_API_KEY') | Out-Null }
    if ($secretPointer -ne [IntPtr]::Zero) { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($secretPointer) }
    $temporarySecret.Dispose()
    if ($null -ne $child) { $child.Dispose() }
}
exit $resultCode
