$ErrorActionPreference = 'Stop'
$f = 'Apeireth-rust\crates\apeireth-api\Cargo.toml'
$t = [datetime]::ParseExact('2026-08-05 21:56:44', 'yyyy-MM-dd HH:mm:ss', $null)
$fi = Get-Item -LiteralPath $f
$fi.LastWriteTime = $t
$fi.CreationTime  = $t
Write-Host ("{0} -> {1}" -f $t.ToString('yyyy-MM-dd HH:mm:ss'), $fi.FullName)
