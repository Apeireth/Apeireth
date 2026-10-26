#requires -Version 7
<#
.SYNOPSIS
    Windows 真机自检：一条命令跑完「只有 Windows 能验 / 值得在 Windows 验」的项，并出一份报告。

.DESCRIPTION
    **为什么有这个脚本**：本仓的真机验证长期挂在「需要 Windows 机器」上，而清单式清单
    每次都要人重新想一遍该跑什么 ⇒ 变成「清单即搁置」。本脚本把该跑的项**固化成代码**，
    跑完自动写一份带判据的报告。

    **覆盖的项（每条都对应一个具体承诺，不是"跑跑看"）**：
      ① D32 自述面读数注记 —— 注记在 Windows 上同样在场（自述面是产品对外唯一承诺面）
      ② 缺陷 ⑳ / ㉑ 的回归单测 —— 逻辑与平台无关，但在 Windows 上再跑一次是廉价的交叉确认
      ③ **shell 可达 / 沙箱墙探针**（`shell_reach_probe --ignored`）
         ⇒ **这是唯一"必须 Windows"的一条**：Linux 上没有等价隔离后端，
         沙箱墙探针在 Linux 必然失败（`IsolationRequirementUnsatisfied`），
         **在 Windows 上它才真正测得到东西**
      ④ **D19 试验田跨平台字节可复现** —— 同一份代码在 Windows 上出的 PNG，
         SHA-256 是否仍等于 Linux 侧的 golden
      ⑤ fuzz 冒烟（自述面 / git 写 / fetch+图像三个 target 各跑一小段）
      ⑥ **gateway HTTP 探针**（台账行 3/4 的**协议侧**）：events 订阅 + approvals 参数校验

    **诚实边界（写在这里，不藏）**：
    - 本脚本是在 **Linux 上写的**，**从未在 Windows 上执行过**（我没有那台机器的访问权）
      ⇒ **首次运行可能需要小修**（路径、端口占用、工具链组件名）；
    - 为此**每一步都是独立的**（单步失败不阻断其余步），**最后一定出报告**；
    - 报告里**如实记**每步的状态与关键输出，**不粉饰**。

.PARAMETER OutDir
    报告输出目录（默认 `reports`）。

.PARAMETER FuzzRuns
    每个 fuzz target 的执行次数（默认 3000，与 CI 的 PR 档一致）。

.PARAMETER SkipFuzz
    跳过 fuzz 冒烟（只跑其余项）。

.EXAMPLE
    pwsh scripts/windows-live-check.ps1
#>
[CmdletBinding()]
param(
    [string]$OutDir = "reports",
    [int]$FuzzRuns = 3000,
    [switch]$SkipFuzz
)

$ErrorActionPreference = "Continue"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $repoRoot

# ⚠️ **自测抓到的坑 ①：sccache 坏了会让**每一步**都红**（2026-10-26 Linux 自测实测）。
# 本机 `RUSTC_WRAPPER=/usr/local/bin/sccache-bai` 指向一个**连不上的 sccache 服务**
# ⇒ 连 `rustc -vV` 都失败 ⇒ 8 个步骤全红 ⇒ **报告变成噪声，看不出哪步真有问题**。
# ⇒ 脚本自己把 wrapper 关掉：**要可复现，不要快**。
$env:RUSTC_WRAPPER = ""
$env:CARGO_BUILD_RUSTC_WRAPPER = ""

# ⚠️ **自测抓到的坑 ②：`$env:TEMP` 在非 Windows 上可能为 null** ⇒ `Join-Path $env:TEMP …` 直接抛错
# ⇒ 步骤被记成 FAIL，但其实是**脚本自己的错**。⇒ 给一个回退。
$script:TempDir = if ($env:TEMP) { $env:TEMP } elseif ($env:TMP) { $env:TMP } else { "/tmp" }

$script:results = New-Object System.Collections.Generic.List[object]
$script:startedAt = Get-Date

function Add-Result {
    param(
        [string]$Name,
        [string]$Status,          # PASS / FAIL / SKIP
        [string]$Detail = "",
        [string]$Command = ""
    )
    $script:results.Add([pscustomobject]@{
        Name    = $Name
        Status  = $Status
        Detail  = $Detail
        Command = $Command
    })
    $mark = switch ($Status) { "PASS" { "OK  " } "FAIL" { "FAIL" } default { "SKIP" } }
    Write-Host ("[{0}] {1}  {2}" -f $mark, $Name, $Detail)
}

function Invoke-Step {
    param(
        [string]$Name,
        [string]$Command,
        [scriptblock]$Action,
        [int]$TimeoutSec = 1800
    )
    Write-Host ""
    Write-Host ("=== {0} ===" -f $Name)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $out = & $Action 2>&1 | Out-String
        $sw.Stop()
        $text = $out.Trim()
        if ($LASTEXITCODE -eq 0 -or $null -eq $LASTEXITCODE) {
            Add-Result -Name $Name -Status "PASS" -Detail ("{0}s｜{1}" -f [int]$sw.Elapsed.TotalSeconds, ($text -replace "\s+", " ").Substring(0, [Math]::Min(220, ($text -replace "\s+", " ").Length))) -Command $Command
        }
        else {
            Add-Result -Name $Name -Status "FAIL" -Detail ("exit={0}｜{1}s｜{2}" -f $LASTEXITCODE, [int]$sw.Elapsed.TotalSeconds, ($text -replace "\s+", " ")) -Command $Command
        }
        return $text
    }
    catch {
        $sw.Stop()
        Add-Result -Name $Name -Status "FAIL" -Detail ("异常：{0}" -f $_.Exception.Message) -Command $Command
        return ""
    }
}

# ── 0. 环境口径（报告要能复现 ⇒ 环境必须入报告）
Write-Host "=== 环境 ==="
$envLines = New-Object System.Collections.Generic.List[string]
$envLines.Add("- 时间：$((Get-Date).ToString('yyyy-MM-dd HH:mm:ss zzz'))")
$envLines.Add("- 仓库：$repoRoot")
$envLines.Add("- OS：$([System.Environment]::OSVersion.VersionString)")
try { $envLines.Add("- rustc：$(rustc --version)") } catch { $envLines.Add("- rustc：**缺**") }
try { $envLines.Add("- cargo：$(cargo --version)") } catch { $envLines.Add("- cargo：**缺**") }
try { $envLines.Add("- nightly：$(rustup toolchain list | Select-String nightly | Select-Object -First 1)") } catch { $envLines.Add("- nightly：**缺**") }
try {
    $gpu = Get-CimInstance Win32_VideoController | Select-Object -First 1
    $envLines.Add("- GPU：$($gpu.Name)")
} catch { $envLines.Add("- GPU：**未知**") }
$envText = $envLines -join "`n"
Write-Host $envText

# ── ① D32 自述面注记 + ② 缺陷 ⑳/㉑ 回归单测
Invoke-Step -Name "① D32 自述面读数注记（单测）" -Command "cargo test -p apeireth-tools-canonical --lib -- self_status" -Action {
    cargo test -p apeireth-tools-canonical --lib -- self_status 2>&1
}
Invoke-Step -Name "② 缺陷 ⑳（畸形 HTML panic）回归" -Command "cargo test -p apeireth-tools-canonical --lib -- html_text" -Action {
    cargo test -p apeireth-tools-canonical --lib -- html_text 2>&1
}
Invoke-Step -Name "② 缺陷 ㉑（git commit 长选项别名绕过）回归" -Command "cargo test -p apeireth-tools-canonical --test git_write_grading" -Action {
    cargo test -p apeireth-tools-canonical --test git_write_grading 2>&1
}

# ── ③ shell 可达 / 沙箱墙探针（**必须 Windows 才测得到东西**）
Invoke-Step -Name "③ shell 可达 + 沙箱墙探针（Windows 语义）" -Command "cargo test -p apeireth-tools-canonical --test shell_reach_probe -- --ignored" -Action {
    cargo test -p apeireth-tools-canonical --test shell_reach_probe -- --ignored --nocapture 2>&1
}

# ── ④ D19 试验田：跨平台字节可复现
$golden = "f8b5b6d8de31d5b3cc0c3b2a56ad61a9b5be6ab6487b9eabf78e013131584d93"
$trialDir = Join-Path $repoRoot "docs/design/m-axis-trial-2026-10-26"
$trialOut = Join-Path $script:TempDir "m-axis-trial-win.png"
$trialText = Invoke-Step -Name "④ D19 试验田 headless 出帧（跨平台 SHA-256）" -Command "cargo run --release -- <tmp>/m-axis-trial-win.png" -Action {
    Push-Location $trialDir
    try { cargo run --release -- $trialOut 2>&1 }
    finally { Pop-Location }
}
$hash = ""
if (Test-Path $trialOut) {
    $hash = (Get-FileHash -Path $trialOut -Algorithm SHA256).Hash.ToLower()
    if ($hash -eq $golden) {
        Add-Result -Name "④ D19 试验田 SHA-256 与 Linux 侧 golden 一致" -Status "PASS" -Detail $hash
    }
    else {
        Add-Result -Name "④ D19 试验田 SHA-256 与 Linux 侧 golden **不一致**" -Status "FAIL" -Detail ("got={0} want={1}" -f $hash, $golden)
    }
}
else {
    Add-Result -Name "④ D19 试验田 SHA-256 比对" -Status "SKIP" -Detail "没产出 PNG（上一步失败）"
}

# ── ⑤ fuzz 冒烟
if ($SkipFuzz) {
    Add-Result -Name "⑤ fuzz 冒烟（三个 target）" -Status "SKIP" -Detail "-SkipFuzz"
}
else {
    foreach ($target in @("self_status", "git_write", "fetch_image")) {
        Invoke-Step -Name ("⑤ fuzz 冒烟：{0}（{1} runs）" -f $target, $FuzzRuns) -Command ("cargo +nightly fuzz run {0} -- -runs={1}" -f $target, $FuzzRuns) -Action {
            Push-Location (Join-Path $repoRoot "fuzz")
            try { cargo +nightly fuzz run $target -- "-runs=$FuzzRuns" 2>&1 }
            finally { Pop-Location }
        }
    }
}

# ── ⑥ gateway HTTP 探针（台账行 3/4 的协议侧）
$port = 18099
$gw = $null
try {
    # ⚠️ **自测抓到的坑 ③：`cargo run --` 在本仓**有 4 个二进制**（apeireth / apeireth-bench-harness /
    # apeireth-tui / sandbox-test-child）⇒ 不给 `--bin` 会报「could not determine which binary to run」
    # ⇒ 步骤被记成 FAIL，其实是**脚本自己的错**。
    # ⇒ 优先用**已编译的二进制**（省编译时间），没有再退回 `cargo run --bin apeireth`。
    $binCandidates = @(
        (Join-Path $repoRoot "target/debug/apeireth.exe"),
        (Join-Path $repoRoot "target/debug/apeireth")
    )
    $gwExe = $binCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if ($gwExe) {
        $gwArgs = @("gateway", "serve", "--port", "$port")
        $gwFile = $gwExe
    }
    else {
        $gwFile = "cargo"
        $gwArgs = @("run", "--bin", "apeireth", "--", "gateway", "serve", "--port", "$port")
    }
    $gw = Start-Process -FilePath $gwFile -ArgumentList $gwArgs -PassThru -NoNewWindow -RedirectStandardOutput (Join-Path $script:TempDir "gw-live-check.out") -RedirectStandardError (Join-Path $script:TempDir "gw-live-check.err")
    # ⚠️ **自测抓到的坑 ④：探针不能用「无异常」当就绪判据**。
    # gateway 的 `/v1/models` **要鉴权** ⇒ 未配 token 时返回 401/403 ⇒
    # `Invoke-WebRequest` 会**抛异常** ⇒ 「有异常 = 没起来」⇒ **明明起来了却判失败**（自测实测）。
    # ⇒ 改用 `-SkipHttpErrorCheck`：**拿到任何 HTTP 响应**就算起来了（401 也算）。
    $ready = $false
    for ($i = 0; $i -lt 30; $i++) {
        Start-Sleep -Seconds 2
        try {
            $probe = Invoke-WebRequest -Uri "http://127.0.0.1:$port/v1/models" -TimeoutSec 3 -UseBasicParsing -SkipHttpErrorCheck
            $ready = $true
            break
        } catch { }
    }
    if ($ready) {
        # 行 3：events 订阅必须是活流（text/event-stream）
        $eventsOk = $false
        $eventsDetail = ""
        try {
            $resp = Invoke-WebRequest -Uri "http://127.0.0.1:$port/v1/apeireth/events" -TimeoutSec 8 -UseBasicParsing -SkipHttpErrorCheck
            $eventsOk = ($resp.Headers["Content-Type"] -like "text/event-stream*")
            $eventsDetail = ("content-type={0}" -f $resp.Headers["Content-Type"])
        } catch { $eventsDetail = ("异常：{0}" -f $_.Exception.Message) }
        Add-Result -Name "⑥ 行3 events 订阅是活流（text/event-stream）" -Status $(if ($eventsOk) { "PASS" } else { "FAIL" }) -Detail $eventsDetail

        # 行 4：approvals 参数校验层（缺 session ⇒ 400/422 且给精确原因）
        $code = 0
        $body = ""
        try {
            $resp = Invoke-WebRequest -Uri "http://127.0.0.1:$port/v1/approvals" -TimeoutSec 8 -UseBasicParsing -SkipHttpErrorCheck
            $code = [int]$resp.StatusCode
        } catch {
            if ($_.Exception.Response) { $code = [int]$_.Exception.Response.StatusCode }
            $body = $_.Exception.Message
        }
        Add-Result -Name "⑥ 行4 approvals 缺参（期望 400/422）" -Status $(if ($code -in 400, 422) { "PASS" } else { "FAIL" }) -Detail ("http={0} {1}" -f $code, ($body -replace "\s+", " "))
    }
    else {
        Add-Result -Name "⑥ gateway 起动" -Status "FAIL" -Detail ("60s 内没起来（看 {0}/gw-live-check.err）" -f $script:TempDir)
    }
}
finally {
    if ($gw -and -not $gw.HasExited) { Stop-Process -Id $gw.Id -Force -ErrorAction SilentlyContinue }
}

# ── 出报告
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir | Out-Null }
$stamp = (Get-Date).ToString("yyyyMMdd-HHmmss")
$reportPath = Join-Path $OutDir "windows-live-check-$stamp.md"

$pass = ($script:results | Where-Object { $_.Status -eq "PASS" }).Count
$fail = ($script:results | Where-Object { $_.Status -eq "FAIL" }).Count
$skip = ($script:results | Where-Object { $_.Status -eq "SKIP" }).Count

$md = New-Object System.Collections.Generic.List[string]
$md.Add("# Windows 真机自检报告 · $stamp")
$md.Add("")
$md.Add('> 由 `scripts/windows-live-check.ps1` 自动生成；**PASS/FAIL 均为实测**，不粉饰。')
$md.Add('> 首次运行可能需要小修（脚本在 Linux 上写就，**从未在 Windows 上执行过**）。')
$md.Add("")
$md.Add("## 环境")
$fence = ([string][char]96) * 3      # markdown 代码栅栏（**单引号/拼接**，避免反引号被当转义符）
$md.Add($fence)
$md.Add($envText)
$md.Add($fence)
$md.Add("")
$md.Add("## 汇总")
$md.Add("")
$md.Add("- PASS **$pass** ｜ FAIL **$fail** ｜ SKIP **$skip**")
$md.Add("")
$md.Add('| 项 | 状态 | 详情 | 命令 |')
$md.Add('|---|---|---|---|')
foreach ($r in $script:results) {
    $detail = ($r.Detail -replace "\|", "\|") -replace "\r?\n", " "
    $md.Add(("| {0} | {1} | {2} | `{3}` |" -f $r.Name, $r.Status, $detail, $r.Command))
}
$md.Add("")
$md.Add("## 怎么读这份报告")
$md.Add("")
$md.Add('- **③ 沙箱墙探针**在 Windows 上应当**全绿**；若红，看是不是 `IsolationRequirementUnsatisfied`')
$md.Add('  （那说明**隔离后端没装**，不是产品缺陷）。')
$md.Add('- **④ SHA-256 不一致**不必然是缺陷：可能是**驱动差异**导致的光栅化差别')
$md.Add('  ⇒ 请把两份 PNG 一起带回来对比（判据：PSNR ≥ 50 dB）。')
$md.Add('- **⑥ 只验协议侧**；**UI 消费那一厘米仍需人工看**（台账行 3/4 的最后一厘米）。')
$md.Add("")
$md.Add("---")
$md.Add("总耗时：$([int]((Get-Date) - $script:startedAt).TotalSeconds)s")

Set-Content -Path $reportPath -Value ($md -join "`n") -Encoding UTF8
Write-Host ""
Write-Host "报告已写：$reportPath"
Write-Host ("汇总：PASS {0} ｜ FAIL {1} ｜ SKIP {2}" -f $pass, $fail, $skip)
if ($fail -gt 0) { exit 1 }
