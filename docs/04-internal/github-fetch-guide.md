# 取材通道指南（通用网页 / crates.io / GitHub 加速节点池）

> `Status: 📋 工具指南（2026-10-18 立，2026-10-18 补通道实测；owner 裁定：池子延迟不稳，**不预筛，每次自己测**）`
> 用途：调研/搬运/下载类批需要取外部内容时，**先在 §〇 选对通道、再测活**，不要盲等官方 API 限速。

## 〇、本机通道实测（2026-10-18，**权威口径，先看这节**）

harness 已切到 DeepSeek 官方 Search/Fetch API（`web_search` / `web_fetch` 工具）。当日实测结论：

| 通道 | 实测证据 | 结论 |
|---|---|---|
| `web_fetch`（DP 官方 Fetch API）· 通用网页 | `example.com` / `crates.io/crates/tokio` / `docs.rs/tokio` 均 **HTTP 200** 返回 Markdown | **通用网页取证首选**，无需代理 |
| `web_fetch` · GitHub 域 | `github.com/ooolabdev/ooosplat`、`raw.githubusercontent.com/…/README.md` → `TypeError: fetch failed`（主控批连续 2 次） | **是「服务端抓取通道到不了 GitHub」，不是本机断网**（见下一行）——GitHub 内容别用 `web_fetch` |
| **本机直连 GitHub**（`curl.exe`） | `api.github.com/repos/tokio-rs/tokio` → **200 / 937ms**（两批均通，稳定）；`raw.githubusercontent.com/…/README.md` → 侦察批 **200**、主控批 **12s 超时**；`github.com/…` → 12s 超时 | **API 直连稳定、raw 直连时通时不通**：raw 先直连试一次（≤8s 快退）→ 不通立刻换 §二 节点池；API 直连可省节点（匿名限速 60 次/小时仍要省着用） |
| crates.io API（`pwsh` 直连） | `api/v1/crates/tokio` → 200 / **1533ms**；`api/v1/crates/itoa/1.0.15/download` → 200 / **852ms** / 11,231 B / `tar -xzf` 解出 14 文件 | **GitHub 之外最稳的源码通道**：Rust crate 直接取官方 `.crate` tarball（Apache/MIT 许可与 `Cargo.toml` 原件齐全，比搬 GitHub 更可信） |
| docs.rs 源码页（`web_fetch`） | `/crate/tokio/1.53.3/source/src/lib.rs` → 200 | 单文件源码**可读**（带行号噪声 + 会截断）；适合读、不适合批量搬 |
| GitHub 加速节点池 | 见 §二；侦察批实测 **32 节点活 20**，`git clone --depth 1` 经 `ghfast.top` **7.2s** 拿到 392 文件全仓 | 非 crate 仓库（README/许可/单文件/整个仓库）的兜底通道；浅克隆优先（一次拿全仓比逐文件省轮次） |

### crate 源码直取配方（不经 GitHub，2026-10-18 实测）

```powershell
# 1) 取 .crate tarball（crates.io 要求带 User-Agent，缺了会 403）
$ver = '1.0.15'; $name = 'itoa'
$tmp = Join-Path $env:TEMP 'crate-src'; New-Item -ItemType Directory -Force $tmp | Out-Null
Invoke-WebRequest -UseBasicParsing -TimeoutSec 30 `
  -Headers @{'User-Agent'='apeireth-build/2.0 (vendor-research)'} `
  -Uri "https://crates.io/api/v1/crates/$name/$ver/download" -OutFile "$tmp\$name.crate"
# 2) 解包（Windows 自带 tar）
tar -xzf "$tmp\$name.crate" -C $tmp
# 3) 许可/版本核对直接看解出来的 Cargo.toml + LICENSE*
```

> 边界：crates.io 只有已发布 crate；仓库里未发布的目录（workspace 子 crate、示例、CI 脚本）取不到——那部分仍要 GitHub 通道。
> 计费/QPS：Search 与 Fetch 各 5 QPS（官方文档口径，2026-10-18 查）；批量取材别打满。

## 一、取材优先序（能不走 API 就不走）

1. **Rust crate 源码** → crates.io tarball（§〇 配方，**最优：不经 GitHub、许可原件齐**）；
2. **通用网页/文档/许可页** → `web_fetch` 直接取；
3. **raw 文件** → 本机 `curl.exe` 直连 `raw.githubusercontent.com` 试一次（≤8s 快退：实测时通时不通），不行走加速前缀；
4. **README/文件内容** → raw 路径（`https://raw.githubusercontent.com/<o>/<r>/<branch>/README.md`）**优于 API**（API 匿名限速 60 次/小时，是盲等的主因）；
5. **仓库元数据/许可/清单** → crates.io API（Rust crate）/ GitHub API（非 crate 仓库，本机直连稳定；省着用：合并查询、缓存到临时文件复用）；
6. **克隆/大文件** → 加速前缀（下表）+ `git clone --depth 1`（整仓一次拿全，比逐文件省轮次；实测 `ghfast.top` 7.2s / 392 文件）；
7. **全部失败** → 本地 TUN/代理通道（`http://127.0.0.1:7900`）兜底。

## 二、候选节点池（32 个，来自公共加速池——**每次用前测活，别信历史**）

### 实测快照（2026-10-18 页面节点检测，17/17 可用——**仅作初始排序提示**）

| 排序 | 节点 | 当时延迟 | 排序 | 节点 | 当时延迟 |
|---|---|---|---|---|---|
| 1 | `gh-proxy.com` | 381ms | 10 | `g.blfrp.cn` | 839ms |
| 2 | `github.chenc.dev` | 512ms | 11 | `ghp.keleyaa.com` | 897ms |
| 3 | `jiashu.1win.eu.org` | 550ms | 12 | `gh.catmak.name` | 919ms |
| 4 | `gitproxy.mrhjx.cn` | 593ms | 13 | `gh.noki.icu` | 1006ms |
| 5 | `gh.b52m.cn` | 620ms | 14 | `gh.dpik.top` | 1053ms |
| 6 | `ghfast.top` | 653ms | 15 | `ghfile.geekertao.top` | 1054ms |
| 7 | `gh.nxnow.top` | 740ms | 16 | `tvv.tw` | 1163ms |
| 8 | `ghpxy.hwinzniej.top` | 780ms | 17 | `fastgit.cc` | 1681ms |
| 9 | `github.ednovas.xyz` | 806ms | | | |

**按此顺序先试快的**——但每次仍然测活（延迟分钟级都会漂）。

### 全量池（含上表 17 个 + 15 个备用）

```
https://git.tangbai.cc            https://gh.927223.xyz
https://gh-proxy.com              https://ghfile.geekertao.top
https://github.chenc.dev          https://gh.dpik.top
https://ghm.078465.xyz            https://ghproxy.imciel.com
https://gitproxy.mrhjx.cn         https://gh.monlor.com
https://ghp.keleyaa.com           https://gh.nxnow.top
https://gh.chjina.com             https://jiashu.1win.eu.org
https://fastgit.cc                https://gh.noki.icu
https://github.ednovas.xyz        https://tvv.tw
https://ghpxy.hwinzniej.top       https://gh.catmak.name
https://g.blfrp.cn                https://github.geekery.cn
https://gh.b52m.cn                https://ghproxy.net
https://git.yylx.win              https://ghfast.top
https://github.xxlab.tech         https://ghproxy.cxkpro.top
https://ghproxy.monkeyray.net     https://git.669966.xyz
https://github.dpik.top           https://gh.ddlc.top
```

**用法**：`<节点>/<原始URL>` 一拼即加速（如 `https://gh-proxy.com/https://raw.githubusercontent.com/o/r/main/README.md`）；各节点支持面不同（raw/releases/archive/clone），用前用后都可能变——测活只证连通，目标类型不匹配就换下一个。

## 三、进场自测活配方（**每个取材批开工先跑这段**）

```powershell
# 并行探测池子：输出当前活着的节点（15 秒内出结果，活的存变量里轮换用）
$pool = @('git.tangbai.cc','gh.927223.xyz','gh-proxy.com','ghfile.geekertao.top','github.chenc.dev','gh.dpik.top','ghm.078465.xyz','ghproxy.imciel.com','gitproxy.mrhjx.cn','gh.monlor.com','ghp.keleyaa.com','gh.nxnow.top','gh.chjina.com','jiashu.1win.eu.org','fastgit.cc','gh.noki.icu','github.ednovas.xyz','tvv.tw','ghpxy.hwinzniej.top','gh.catmak.name','g.blfrp.cn','github.geekery.cn','gh.b52m.cn','ghproxy.net','git.yylx.win','ghfast.top','github.xxlab.tech','ghproxy.cxkpro.top','ghproxy.monkeyray.net','git.669966.xyz','github.dpik.top','gh.ddlc.top')
$probe = 'https://raw.githubusercontent.com/github/gitignore/main/Rust.gitignore'
$alive = $pool | ForEach-Object -Parallel {
  $r = curl.exe -sS -o NUL -w '%{http_code}' --max-time 8 "https://$_/$using:probe" 2>$null
  if ($r -eq '200') { $_ }
} -ThrottleLimit 10
"活着 $($alive.Count) 个: $($alive -join ', ')"
```

## 四、聪明使用守则

1. **轮换不硬磕**：一个节点失败/超时 → 换下一个（同内容最多试 3 个节点，然后换通道）；
2. **限速自保**：API 查询合并批量化（一次查多个仓库）、结果缓存临时文件（同批内复用）；
3. **失败快退**：单节点超时 ≤8s，不重试超过 2 次——**盲等是最大的浪费**；
4. **可信边界**：加速节点是第三方中转——**只取公开内容**，绝不经它们传凭据/私有仓库（与 secret-scan 纪律同魂）；取回内容照常过中性表述/安全审查；
5. **随时变化**：池子会死会活会换——本表过期就再拉一次源（`https://xiake.pro/github/script.js` 的 mirrors 数组），别执念于名单。

## 五、批派发口径

派调研/搬运批时提示语带一句：**「取材按 docs/04-internal/github-fetch-guide.md：先测活、raw 优先、API 省着用、失败快退」**。
