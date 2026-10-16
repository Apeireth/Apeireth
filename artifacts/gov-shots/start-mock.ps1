$root = 'C:\Users\31683\Apeireth-rust'
$m = Start-Process node -ArgumentList "`"$root\artifacts\gov-shots\mock-gateway.mjs`"" -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 1
try {
  $r = Invoke-WebRequest -UseBasicParsing -Uri http://127.0.0.1:8080/health -TimeoutSec 3
  Write-Output "mock=$($m.Id) health=$($r.StatusCode)"
} catch {
  Write-Output "mock=$($m.Id) health=FAIL"
}
