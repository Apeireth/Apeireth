# orphan-scan.ps1 — workspace 孤儿 crate + dev-dep 回环体检 (台账 N18/TP9 规范配套工具)
#
# 职责 (maintenance-guide §三 消费方登记规范的定期检查):
#   1. 零内部消费者扫描: 有 lib 但无任何 normal 内部依赖者的 workspace 成员
#      (dev/test 专用件与 bin 终点件单独标注, 不算孤儿)
#   2. dev-dep 回环检测: dev-dependencies 内部边的自引用/双向环/DFS 长环/dev↔normal 互指环
#   3. 与台账 #33 的 12 孤儿清单自动对账
#
# 数据源: cargo metadata --no-deps (依赖 kind=normal/dev/build 权威区分, 不靠文本 grep)
# 0 装 PASS: 本脚本只读分析, 不修改任何 crate; 孤儿处置决策归 Leader。
#
# 用法:
#   powershell -NoProfile -ExecutionPolicy Bypass -File _scripts\orphan-scan.ps1
#   powershell ... -File _scripts\orphan-scan.ps1 -OutFile reports\orphan-scan.md
# 建议: 每次新增 crate / release 前跑一次; 输出并入 docs/backlog.md 台账。

param([string]$OutFile = '')

# cargo metadata 输出为 UTF-8 (含中文描述), PS 5.1 默认按 ANSI 码页解码会破坏 JSON → 强制 UTF-8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Push-Location $repo
try {
    $meta = & cargo metadata --no-deps --format-version 1 2>$null | ConvertFrom-Json
    if (-not $meta -or -not $meta.packages) { throw "cargo metadata 失败 (需在 workspace 根运行且工具链可用)" }
} finally { Pop-Location }

$memberNames = @($meta.packages.name)

# 依赖方向: dep.name = 被依赖者(消费目标), pkg.name = 消费者
$normalConsumers = @{}; $devConsumers = @{}; $devEdges = @(); $selfDevRefs = @()
foreach ($n in $memberNames) { $normalConsumers[$n] = @(); $devConsumers[$n] = @() }
foreach ($pkg in $meta.packages) {
    foreach ($dep in $pkg.dependencies) {
        if ($memberNames -notcontains $dep.name) { continue }   # 只看内部依赖
        $kind = if ($dep.kind) { $dep.kind } else { 'normal' }
        switch ($kind) {
            'normal'  { $normalConsumers[$dep.name] += $pkg.name }
            'build'   { $normalConsumers[$dep.name] += $pkg.name }   # build 依赖也是真实消费
            'dev'     {
                if ($dep.name -eq $pkg.name) { $selfDevRefs += $pkg.name; break }  # 自引用 (台账 #33②)
                $devConsumers[$dep.name] += $pkg.name
                $devEdges += ,@($pkg.name, $dep.name)
            }
        }
    }
}

$libTargets  = @(); $orphans = @(); $devOnly = @(); $binTerminals = @()
foreach ($pkg in $meta.packages) {
    $hasLib = @($pkg.targets | Where-Object { $_.kind -contains 'lib' }).Count -gt 0
    $hasBin = @($pkg.targets | Where-Object { $_.kind -contains 'bin' }).Count -gt 0
    if (-not $hasLib) { continue }
    $libTargets += $pkg.name
    $nc = $normalConsumers[$pkg.name]
    if ($nc.Count -eq 0) {
        if ($pkg.name -match '-test$|-e2e$|-bench$|-eval$') { $devOnly += $pkg.name }
        elseif ($hasBin) { $binTerminals += $pkg.name }   # 有 bin = 进程/CLI 终点, 天生 0 lib 消费者, 不算孤儿
        else { $orphans += $pkg.name }
    }
}

# --- dev-dep 环检测 (DFS, 深度上限 8 防组合爆炸) ---
$adj = @{}
foreach ($e in $devEdges) { if (-not $adj.ContainsKey($e[0])) { $adj[$e[0]] = @() }; $adj[$e[0]] += $e[1] }
$pairs  = @()
$seenP = @{}
foreach ($e in $devEdges) {
    $back = $devEdges | Where-Object { $_[0] -eq $e[1] -and $_[1] -eq $e[0] }
    if ($back) { $key = @($e[0], $e[1]) | Sort-Object; $k = "$($key[0]) <-> $($key[1])"; if (-not $seenP[$k]) { $seenP[$k] = $true; $pairs += $k } }
}
# dev↔normal 互指: A --dev--> B 且 B --normal--> A (语义环, 台账 #33③ 模式)
$devNormalPairs = @(); $seenDN = @{}
foreach ($e in $devEdges) {
    $a = $e[0]; $b = $e[1]
    if ($normalConsumers[$a] -contains $b) {
        $key = @($a, $b) | Sort-Object; $k = "$($key[0]) <-> $($key[1])"
        if (-not $seenDN[$k]) { $seenDN[$k] = $true; $devNormalPairs += "$a --dev--> $b --normal--> $a" }
    }
}
$longCycles = @(); $seenC = @{}
function Find-Cycles($node, $start, $path, $depth) {
    if ($depth -gt 8) { return }
    foreach ($nxt in $adj[$node]) {
        if ($nxt -eq $start -and $path.Count -ge 2) {
            $cyc = @($path | ForEach-Object { $_ }) + $start
            $canon = ($cyc | Sort-Object) -join '|'
            if (-not $seenC.ContainsKey($canon)) { $seenC[$canon] = $true; $script:longCycles += ,($cyc -join ' -> ') }
        } elseif ($path -notcontains $nxt) {
            Find-Cycles $nxt $start ($path + $nxt) ($depth + 1)
        }
    }
}
foreach ($s in ($adj.Keys | Sort-Object)) { Find-Cycles $s $s @($s) 1 }

# --- 台账 #33 对账 (12 孤儿候选, C3 盘点) ---
$ledger33 = @('apeireth-provider','apeireth-cron','apeireth-experience','apeireth-environment','apeireth-config','apeireth-state','apeireth-naming-v05','apeireth-livekit','apeireth-blueprint-impl','apeireth-library-governance','apeireth-voice','apeireth-context-fold')

$lines = @()
$lines += "# orphan-scan 报告 ($(Get-Date -Format 'yyyy-MM-dd HH:mm'))"
$lines += ""
$lines += "workspace 成员: $($memberNames.Count) | 含 lib: $($libTargets.Count) | 零 normal 消费者: $($orphans.Count + $devOnly.Count + $binTerminals.Count) (孤儿 $($orphans.Count) + dev-only $($devOnly.Count) + bin 终点 $($binTerminals.Count))"
$lines += ""
$lines += "## 一、孤儿 lib crate (纯 lib 且零内部 normal 消费者, 待 Leader 处置决策)"
if ($orphans.Count -eq 0) { $lines += '(无)' } else { $lines += $orphans -join ', ' }
$lines += ""
$lines += "## 二、dev/test 专用件 (零 normal 消费者, 但属 dev-only, 不算孤儿)"
if ($devOnly.Count -eq 0) { $lines += '(无)' } else { $lines += $devOnly -join ', ' }
$lines += ""
$lines += "## 二b、bin 终点件 (含 bin target, 天生 0 lib 消费者, 不算孤儿)"
if ($binTerminals.Count -eq 0) { $lines += '(无)' } else { $lines += $binTerminals -join ', ' }
$lines += ""
$lines += "## 三、dev-dep 自引用 (台账 #33② 模式)"
if ($selfDevRefs.Count -eq 0) { $lines += '(无)' } else { $lines += $selfDevRefs -join ', ' }
$lines += ""
$lines += "## 四、dev-dep 双向回环"
if ($pairs.Count -eq 0) { $lines += '(无)' } else { $lines += $pairs -join ', ' }
$lines += ""
$lines += "## 四b、dev↔normal 互指环 (台账 #33③ 模式)"
if ($devNormalPairs.Count -eq 0) { $lines += '(无)' } else { $lines += $devNormalPairs -join '; ' }
$lines += ""
$lines += "## 五、dev-dep 长环 (DFS, 深度上限 8)"
if ($longCycles.Count -eq 0) { $lines += '(无)' } else { $lines += $longCycles -join '; ' }
$lines += ""
$lines += "## 六、台账 #33 清单对账"
foreach ($c in $ledger33) {
    $status = if ($memberNames -notcontains $c) { '不在 workspace (已归档/删除?)' }
              elseif ($orphans -contains $c) { '仍为孤儿 (零 normal 消费者)' }
              elseif ($devOnly -contains $c) { 'dev-only 专用件' }
              elseif ($binTerminals -contains $c) { 'bin 终点件' }
              else { "已有 normal 消费者: $(($normalConsumers[$c] | Sort-Object) -join ', ')" }
    $lines += "- $c : $status"
}
$lines += ""
$lines += "## 七、全部成员消费计数 (normal)"
$lines += ""
$lines += "| crate | normal 消费者数 | 消费者 |"
$lines += "|---|---|---|"
foreach ($n in ($memberNames | Sort-Object)) {
    $cs = $normalConsumers[$n] | Sort-Object
    $lines += "| $n | $($cs.Count) | $(if ($cs) { $cs -join ', ' } else { '-' }) |"
}

$report = $lines -join "`n"
Write-Output $report
if ($OutFile) { Set-Content -Path $OutFile -Value $report -Encoding utf8; Write-Output "`n[已保存] $OutFile" }
