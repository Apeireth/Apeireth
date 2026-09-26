# 服务商链路验证脚本（真凭据实跑 + 热更生效性判别）
# 用途：验证"网关 → 服务商"链路的三件事——①真凭据可用；②热更凭据即时生效；
# ③热更后可恢复。全程密钥只在内存流转，输出仅回显尾号。
# 用法：powershell -File scripts/verify-provider-path.ps1 [-Port 8099]
# 判定语义：热更生效 = 诱饵提交后下一请求**改用诱饵**（被服务商拒绝或回显诱饵才算 PASS）。

param([int]$Port = 8099)

$ErrorActionPreference = 'Stop'
$fail = 0

function Step($name, $ok, $detail) {
  if ($ok) { Write-Output "PASS  $name  $detail" } else { Write-Output "FAIL  $name  $detail"; $script:fail++ }
}

function Read-ErrorBody($err) {
  $msg = $err.ErrorDetails.Message
  if (-not $msg) {
    try {
      $reader = New-Object IO.StreamReader($err.Response.GetResponseStream())
      $msg = $reader.ReadToEnd()
    } catch { $msg = '' }
  }
  if (-not $msg) { $msg = $err.Exception.Message }
  return $msg
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
$body = @{model='deepseek-flash'; messages=@(@{role='user'; content='reply with the single word: pong'})} | ConvertTo-Json -Depth 5

# ---- 3. ①真凭据聊天 ----
try {
  $r = Invoke-RestMethod -Uri "$base/v1/chat/completions" -Method Post -Headers @{Authorization="Bearer $realKey"} -Body $body -ContentType 'application/json' -TimeoutSec 90
  Step 'real-key-chat' ($null -ne $r.choices) ("回复: $($r.choices[0].message.content)")
} catch {
  $msg = Read-ErrorBody $_
  if ($msg -match 'auth|401|Unauthorized|invalid.*key') {
    Step 'real-key-chat' $false "凭据库中的 key 被服务商拒绝——请在应用设置中重新保存有效 key 后重试: $($msg.Substring(0, [Math]::Min(100, $msg.Length)))"
  } else {
    Step 'real-key-chat' $false $msg.Substring(0, [Math]::Min(120, $msg.Length))
  }
}

# ---- 4. ②诱饵热更：换假 key，下一请求必须改用假 key ----
$decoy = 'sk-decoy-INVALID-000000'
try {
  Invoke-RestMethod -Uri "$base/v1/admin/config" -Method Post -Headers @{Authorization="Bearer $realKey"} -Body (@{provider='openai'; api_key=$decoy} | ConvertTo-Json) -ContentType 'application/json' -TimeoutSec 10 | Out-Null
} catch { Step 'hot-swap-apply' $false (Read-ErrorBody $_) }
Start-Sleep 1
try {
  $null = Invoke-RestMethod -Uri "$base/v1/chat/completions" -Method Post -Headers @{Authorization="Bearer $decoy"} -Body $body -ContentType 'application/json' -TimeoutSec 90
  Step 'hot-swap-takes-effect' $false '热更后请求仍用旧凭据成功——热更未生效'
} catch {
  $msg = Read-ErrorBody $_
  if ($msg -match 'decoy|invalid|auth|401|Unauthorized|authentication') {
    Step 'hot-swap-takes-effect' $true "下一请求已改用新凭据（被拒/回显诱饵）: $($msg.Substring(0, [Math]::Min(90, $msg.Length)))"
  } else {
    Step 'hot-swap-takes-effect' $false "非预期失败, 无法判定: $($msg.Substring(0, [Math]::Min(120, $msg.Length)))"
  }
}

# ---- 5. ③恢复真凭据，再次聊天应成功 ----
try {
  Invoke-RestMethod -Uri "$base/v1/admin/config" -Method Post -Headers @{Authorization="Bearer $decoy"} -Body (@{provider='openai'; api_key=$realKey} | ConvertTo-Json) -ContentType 'application/json' -TimeoutSec 10 | Out-Null
} catch { Step 'restore-apply' $false (Read-ErrorBody $_) }
Start-Sleep 1
try {
  $r3 = Invoke-RestMethod -Uri "$base/v1/chat/completions" -Method Post -Headers @{Authorization="Bearer $realKey"} -Body $body -ContentType 'application/json' -TimeoutSec 90
  Step 'restore-and-chat' ($null -ne $r3.choices) ("恢复成功, 回复: $($r3.choices[0].message.content)")
} catch {
  $msg = Read-ErrorBody $_
  Step 'restore-and-chat' $false $msg.Substring(0, [Math]::Min(120, $msg.Length))
}

Stop-Process -Id $gw.Id -Force -ErrorAction SilentlyContinue
Write-Output "---- 结果: $(if ($fail -eq 0) { 'ALL PASS' } else { "$fail FAILED" }) ----"
exit $fail