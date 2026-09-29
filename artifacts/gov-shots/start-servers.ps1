$root = 'C:\Users\31683\Apeireth-rust'
$m = Start-Process node -ArgumentList "`"$root\artifacts\gov-shots\mock-gateway.mjs`"" -PassThru -WindowStyle Hidden
$v = Start-Process node -ArgumentList "`"node_modules\vite\bin\vite.js`"" -WorkingDirectory "$root\frontend\companion-desktop" -PassThru -WindowStyle Hidden
"$($m.Id) $($v.Id)" | Out-File -Encoding ascii "$root\artifacts\gov-shots\pids.txt"
Write-Output "mock=$($m.Id) vite=$($v.Id)"
