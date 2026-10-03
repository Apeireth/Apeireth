# Apeireth 终端驾驶舱启动器：后台网关 + 全屏驾驶舱
$env:APEIRETH_KEYRING_BACKEND = 'auto'
$g = 'C:\Users\31683\Apeireth-rust\target\x86_64-pc-windows-msvc\release\apeireth.exe'
$t = 'C:\Users\31683\Apeireth-rust\target\release\apeireth-tui.exe'
Start-Process -WindowStyle Hidden $g -ArgumentList 'gateway','serve'
Start-Sleep 3
& $t
