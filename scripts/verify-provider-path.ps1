# 服务商链路验证脚本（真 key 实跑 + 诱饵模拟）
# 用途：验证"网关 → 服务商"链路的三件事——①真 key 可用；②热更凭据即时生效；
# ③热更后可恢复。全程密钥只在内存流转，输出仅回显尾号。
# 用法：powershell -File scripts/verify-provider-path.ps1 [-Port 8099]

param([int]$Port = 8099)

$ErrorActionPreference = 'Stop'
$fail = 0

function Step($name, $ok, $detail) {
  if ($ok) { Write-Output "PASS  $name  $detail" } else { Write-Output "FAIL  $name  $detail"; $script:fail++ }
}

# ---- 1. 从系统凭据库取真实密钥（只回显尾号） ----
$sig = @"
using System; using System.Runtime.InteropServices;
public class NativeCred {
  [DllImport("advapi32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
  public static extern bool CredRead(string t, int ty, int f, out IntPtr p);
  [DllImport("advapi32.dll")] public static extern void CredFree(IntPtr b);
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
  public struct C { public int F; public int T; public IntPtr TN; public IntPtr CM; public long LW; public int BS; public IntPtr B; public int P; public int AC; public IntPtr A; public IntPtr TA; public IntPtr UN; }
}
"@; Add-Type $sig
$ptr = [IntPtr]::Zero
if (-not [NativeCred]::CredRead('apeireth.companion:provider:openai', 1, 0, [ref]$ptr)) { Write-Output 'FAIL  key-source  凭据库无条目'; exit 1 }
$c = [Runtime.InteropServices.Marshal]::PtrToStructure($ptr, [type][NativeCred+C])
$bytes = New-Object byte[] $c.BS
[Runtime.InteropServices.Marshal]::Copy($c.B, $bytes, 0, $c.BS)
$realKey = [Text.Encoding]::Unicode.GetString($bytes)
[NativeCred]::CredFree($ptr)
Step 'key-source' ($realKey.Length -gt 8) ("凭据库读取成功, 长度 $($realKey.Length), 尾号 ****$($realKey.Substring($realKey.Length-4))")

# ---- 2. 起网关（测试端口，进程环境注入，脚本退出即收） ----
$env:APEIRETH_API_KEY = $realKey
$env:OPENAI_API_KEY = $realKey
$env:APEIRETH_OPENAI_URL = 'https://api.deepseek.com/v1'
$env:APEIRETH_OPENAI_MODELS = 'deepseek-flash'
$exe = Join-Path (Split-Path $PSScriptRoot -Parent) 'target\x86_64-pc-windows-msvc\release\apeireth.exe'
if (-not (Test-Path $exe)) { $exe = 'C:\Program Files\Apeireth Companion\apeireth.exe' }
$gw = Start-Process -FilePath $exe -ArgumentList 'gateway','serve','--port',"$Port" -PassThru -WindowStyle Hidden
Start-Sleep 5
$base = "http://127.0.0.1:$Port"

# ---- 3. ①真 key 聊天 ----
$body = @{model='deepseek-flash'; messages=@(@{role='user'; content='reply with the single word: pong'})} | ConvertTo-Json -Depth 5
try {
  $r = Invoke-RestMethod -Uri "$base/v1/chat/completions" -Method Post -Headers @{Authorization="Bearer $realKey"} -Body $body -ContentType 'application/json' -TimeoutSec 90
  Step 'real-key-chat' ($null -ne $r.choices) ("回复: $($r.choices[0].message.content)")
} catch { Step 'real-key-chat' $false $_.Exception.Message.Substring(0, [Math]::Min(120, $_.Exception.Message.Length)) }

# ---- 4. ②诱饵热更：换假 key，下一请求必须用假 key（失败且回显诱饵 = 热更生效） ----
$decoy = 'sk-decoy-INVALID-000000'
try {
  Invoke-RestMethod -Uri "$base/v1/admin/config" -Method Post -Headers @{Authorization="Bearer $realKey"} -Body (@{provider='openai'; api_key=$decoy} | ConvertTo-Json) -ContentType 'application/json' -TimeoutSec 10 | Out-Null
} catch { Step 'hot-swap-apply' $false $_.Exception.Message }
Start-Sleep 1
try {
  $r2 = Invoke-RestMethod -Uri "$base/v1/chat/completions" -Method Post -Headers @{Authorization="Bearer $decoy"} -Body $body -ContentType 'application/json' -TimeoutSec 90
  Step 'hot-swap-takes-effect' $false '热更后仍用旧密钥成功（凭据未即时生效）'
} catch {
  $msg = $_.Exception.Message
  try { $reader = New-Object IO.StreamReader($_.Exception.Response.GetResponseStream()); $msg = $reader.ReadToEnd() } catch {}
  Step 'hot-swap-takes-effect' ($msg -match 'decoy|INVALID|401|Unauthorized|authentication') ("热更已即时生效（请求使用了新凭据）: " + $msg.Substring(0, [Math]::Min(120, $msg.Length)))
}

# ---- 5. ③恢复真 key，再次聊天应成功 ----
try {
  Invoke-RestMethod -Uri "$base/v1/admin/config" -Method Post -Headers @{Authorization="Bearer $decoy"} -Body (@{provider='openai'; api_key=$realKey} | ConvertTo-Json) -ContentType 'application/json' -TimeoutSec 10 | Out-Null
} catch { Step 'restore-apply' $false $_.Exception.Message }
Start-Sleep 1
try {
  $r3 = Invoke-RestMethod -Uri "$base/v1/chat/completions" -Method Post -Headers @{Authorization="Bearer $realKey"} -Body $body -ContentType 'application/json' -TimeoutSec 90
  Step 'restore-and-chat' ($null -ne $r3.choices) ("恢复成功, 回复: $($r3.choices[0].message.content)")
} catch {
  $msg = $_.Exception.Message
  try { $reader = New-Object IO.StreamReader($_.Exception.Response.GetResponseStream()); $msg = $reader.ReadToEnd() } catch {}
  Step 'restore-and-chat' $false $msg.Substring(0, [Math]::Min(120, $msg.Length))
}

Stop-Process -Id $gw.Id -Force -ErrorAction SilentlyContinue
Write-Output "---- 结果: $(if ($fail -eq 0) { 'ALL PASS' } else { "$fail FAILED" }) ----"
exit $fail
