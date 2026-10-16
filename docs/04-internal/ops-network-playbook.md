# 运维与网络排障手册（踩坑沉淀）

```text
[Document-Meta]
性质:    经验沉淀手册 —— 2026-10 基建事件中的坑与解法, 可复用。
范围:    代理网络排障 / 大文件下载 / WSL 修复安装 / GitHub 账号事件 / shell 陷阱。
姊妹篇:  证明面的坑见 kani-verification-playbook.md。
```

## 1. 代理网络排障（本机 7897 混合口）

| 症状 | 根因 | 药方 |
|---|---|---|
| `github.com` TLS 握手持续失败, 但 `api.github.com` 正常 | 代理**分流规则**里两者走不同节点, github.com 那条节点抽风 | 等节点恢复窗口重试（探测循环: `curl .../info/refs?service=git-upload-pack` 打 200 即推）; 或走 SSH 备胎（§4） |
| curl 报 `CRYPT_E_NO_REVOCATION_CHECK` (0x80092012) | schannel 吊销检查在代理 TLS 拦截下抽风 | curl 加 `--ssl-no-revoke`; git 加 `-c http.schannelCheckRevoke=false` |
| git 换 openssl 后端报 `unable to get local issuer certificate` | 代理 MITM 证书只在 Windows 证书库 | 保持 schannel + 上条的关吊销检查; openssl 后端别用 |
| 命令长时间无输出假死 | `git credential fill`(GCM) 被前一个被杀进程留锁 | 杀残留 `git-credential-manager` 进程; token 直接从 `~/.git-credentials` 解析, 不走 GCM |
| WSL 内访问 GitHub/crates.io 不通 | WSL2 NAT, 直连被墙 | 走宿主机代理: `export https_proxy=http://<网关IP>:7897`（网关 = `ip route` 的 default; 代理须监听 0.0.0.0） |

顺手统一: git 全局 `http.proxy`/`https.proxy` = `http://127.0.0.1:7897`。

## 2. 大文件下载（加速节点池分块并行）

适用: GitHub Releases 附件（官方直连慢/断）。源: xiake.pro 的 GitHub 加速器
节点表（`https://xiake.pro/github/script.js` 内的 URL 数组, 30+ 公共节点）。

```text
① Range 探测: curl -r 0-1 <node>/<url> → 只收 HTTP 206 的节点 (忽略 Range 的
   节点会回整文件, 分块拼接必坏);
② 分块并行: N 节点 × N 块 (curl -r start-end -o part_i), Start-Process 并发;
③ 按尺寸校验: 每块字节数 == 区间长度, 不符换节点重试 (4 轮);
④ 顺序拼接后总字节数 == Content-Length。
```

实测: 367MB WSL MSI 16 路 ~13 分钟; 140MB CBMC 包 12 路 ~16 分钟(节点波动时)。
注意: 节点速度随时间波动, 先 2MB 限速实测排座次。

## 3. WSL 修复与安装（注册表损坏/无发行版）

| 症状 | 根因 | 药方 |
|---|---|---|
| `wsl --status`/`-l`/`-e` 全报 `Wsl/CallMsi/Install/REGDB_E_CLASSNOTREG` | WSL 二进制在但 MSI 注册烂, 连 `wsl --install` 自修都 exit -1 | 直接装 **WSL MSI 包**（GitHub Releases, msix/msi 自注册）: `msiexec /i wsl.<ver>.x64.msi /qn` |
| `wsl --install -d <distro>` 报「系统找不到指定的文件」 | Store 发行版下载通道不通 | **rootfs 直下 + `wsl --import`**: `https://cloud-images.ubuntu.com/wsl/releases/24.04/current/ubuntu-noble-wsl-amd64-24.04lts.rootfs.tar.gz`（Ubuntu CDN 国内可达, 实测 17MB/s）|
| `wsl --import` 报 `HCS/HCS_E_SERVICE_NOT_AVAILABLE` | 虚拟机平台组件未启用 | `dism /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart` + `Microsoft-Windows-Subsystem-Linux`, **重启**后重试 |
| `cargo kani setup` 拉不动 CBMC 包 | GitHub 直连抖 | `cargo kani setup --local-bundle <本地tar.gz>`（读 kani-verifier setup.rs 可知这是正门, 它不会检查"下载目标路径已存在的包"） |

权限: 本机 UAC 静默提权, `Start-Process -Verb RunAs -Wait` 可无人值守。

## 4. GitHub 账号事件（封禁 → 申诉 → 恢复）

- 症状: API 403 `Sorry. Your account was suspended`; git 读可能仍活, 写会死;
  该账号的 PR 在匿名视角"消失"。
- 处置次序: ① 注册邮箱查 GitHub 通知找原因类别; ②
  [恢复表单](https://support.github.com/contact/reinstatement)（Reinstatement/
  Appeal 二选一, 登不进去先走 `cannot_sign_in`）; ③ 邮件工单跟进只发一封、
  引用工单号; ④ **不认没干过的事**（公开 API 可自证: `search/issues?q=author:X`
  查 PR 足迹、`/users/X/starred` 查 star 量）; ⑤ 不开小号（二次违规 = 永久）。
- SSH over 443 备胎（HTTPS 线路瘫痪时推送）: 密钥挂账号（`POST /user/keys`,
  api.github.com 线路独立常活）; ssh 配置 `HostName ssh.github.com Port 443`
  + `ProxyCommand` 用 Python 做 CONNECT 中继; 中继脚本两个坑: ① CONNECT
  响应与 SSH banner 可能同段到达（逐字节读到 `\r\n\r\n`, 余下字节转给 ssh）;
  ② 收尾用 `os.read/os.write` + `os._exit(0)`（BufferedReader + daemon 线程
  在解释器退出时竞态崩栈）。

## 5. shell 陷阱（PowerShell/bach 通用）

- **管道截断杀进程**: `... | head -N` / `Select-Object -First N` 提前关管道,
  SIGPIPE 杀掉上游, `$LASTEXITCODE`/`exit 0` 是截断者的——**验证/构建输出
  一律落文件再 grep**（三次假绿事故的根源, 详见 kani-playbook K9）。
- `pkill -f <pattern>` 会匹配到当前 bash -c 自身命令行 → 自杀; pattern 里
  避开自身串或用 pid 精确杀。
- **bash 嵌套引号/here-doc 是重灾区**（一日三踩: `wsl bash -c @'...'@` 里的
  here-doc、awk 单引号、sed 多行全炸过）: 优先 **UNC 路径直改文件**
  （`\\wsl.localhost\<发行版>\root\...` 用 PowerShell/编辑器改）+ bash 只跑
  **单条纯命令**; 必须多行时用 `-e bash -c` 配最简引号, 避免 here-doc 嵌套。
- wsl.exe 输出是 UTF-16, PowerShell 直接 `Out-String` 是乱码;
  `[Console]::OutputEncoding = [System.Text.Encoding]::Unicode` 或让 WSL 内
  bash 输出（UTF-8）。
- awk/复杂引号经 `wsl bash -c "..."` 双层转义易碎; 用 `bash -c @'...'@`
  here-string 喂整段脚本。
- Python 在 Windows 默认 GBK 读 JSON 会炸中文; `open(..., encoding='utf-8')`。

## 6. 跨平台编译面陷阱（Windows 本地绿 ≠ Linux CI 绿）

- 症状: 本地 Windows `cargo check`/`clippy` 全绿, CI (ubuntu) 直接编译错误
  E0599——例: `file_encrypted.rs` 的 `opts.mode(0o600)` 找不到方法。
- 根因: **`#[cfg(unix)]` 屏蔽的是可见性, 不是"trait 要在作用域内"这条编译规则**
  —— 块内用了 Unix 专有扩展方法（`OpenOptionsExt::mode`）但没 `use` 该 trait,
  Linux 编译必炸; Windows 因 cfg 整块不编译, 本地零报错。与 kani 手册 K 系列
  同款病根: **本地验证面 ≠ CI 验证面**。
- 药方:
  - cfg(unix)/cfg(windows) 块内**第一行带全平台专属导入**;
  - 动过跨平台代码后, **WSL/Linux 侧过一遍 `cargo check`** 再收工
    （干净副本: `tar --exclude=./target -cf - . | tar -C /root/debt -xf -`）;
  - CI clippy 是 **`-D warnings` + 长白名单**口径 —— 本地复现必须带同款
    flag 组（见 .github/workflows/rust-lint.yml）, 否则"本地全绿"是假象。

### §6.2 「本地绿 ≠ CI 绿」三变种 + CI 解剖工具箱（2026-10-15 欠账战役实证）

三变种（同一个病根的三种形态）:
1. **编译面**: cfg(unix) 块本地不编译（§6 主条, OpenOptionsExt 案）;
2. **平台语义面**: 测试含 Windows 路径语义（`C:\data` 反斜杠/盘符）, unix 上
   `\` 非分隔符 —— windows_service_script_dry_run / tuning_log_lands_beside_
   session_db 两案。**测试名或断言含平台语义的, 一律 cfg 守卫**;
3. **键规范化面**: `fs::canonicalize` 返回**真实大小写 + symlink 解析后**的
   拼写（macOS `/var`→`/private/var`、Windows 大小写/`\\?\` verbatim）, 归一化
   键不展开 symlink 时同一文件落两个键 —— 写后回填查无此人（apply_patch
   版本 CAS 三连败案, 示踪招供: `target_key=/var/...` vs
   `gate_keys=[/private/var/...]`）。**路径键必须按目标平台语义折叠 + 查询方
   用 canonical 拼写命中**; 本地大小写一致/symlink 不经斜路时永不复现,
   最隐蔽的一变种。

破案方法论（理论三连破后唯一正道）: **示踪三选一判据** —— 在失败点埋
诊断打印（双方键/分支标记）, 让 CI 现场自己招供: ①键对比差异 = 失配实锤;
②特定标记出现 = 对应理论复活; ③空 = 上游根本没发生。一轮 CI 破三轮悬案。

CI 解剖工具箱（按效率排序）:
- **报错归属会误导**: "could not document `X`" 的 X 不一定是病灶 crate
  （真凶在 sdk 的文档链接）—— 以 `--> file:line` 的地址为准;
- **退出码读心术**: bash 门挂 101 = 内有 cargo test/clippy（读判定脚本比
  猜日志快）;
- check-run annotations API 只给退出码, 真相在日志中段;
- **miri 线程名/栈帧定位**: `this is on thread 'X::'` + 栈直指
  `函数:行`（sync_parent_dir:203 案）;
- 日志过滤陷阱: `Get-Content -Raw` 与 `-TotalCount` 不可同用; GitHub 日志
  行带时间戳前缀, `^error` 锚点会全部落空。

### §6.3 「CI 假死」真相: PR 冲突 = checks 静默不建（2026-10-15 一小时悬案）

- 症状: push 落库成功、PR 开着、Actions 状态全绿, 但 **check-runs /
  check-suites 全为 0**; 空提交踢不动, 关开 PR 无效, 新开 PR 也 0 runs;
  `workflow_dispatch` 却能正常建 run —— 极像"事件系统坏了"。
- **真相: `pulls.mergeable=false`（分支与 base 冲突）→ GitHub 构造不出 merge
  ref → 全部 pull_request 类 checks 静默不创建**（不报错、不通知、无注解）。
  main 被别人推进后最容易踩。
- 排查序列（按性价比）: ① `pulls/N` 的 `mergeable`/`mergeable_state` 字段
  （dirty=冲突实锤）; ② `actions/runs?head_sha=` 按 sha 查 runs（分支列表
  视图会误导）; ③ `commits/<sha>/check-suites` 底层探针; ④ `workflow_dispatch`
  验证创建链（通 → 事件链问题; 不通 → 更深）。
- 药方: **合并 base 进分支解冲突 → push → checks 自动复活**; 顺手收益 =
  分支不再落后主干。教训: "CI 全体静默" 先查 mergeable, 再怀疑事件系统。

### §6.4 分支锁与保护 API 姿势（2026-10-15 合并收官实录）

- **`lock_branch=true` = "Cannot change this locked branch"** —— 字面意思:
  main 只读, 一切合并/推送被卡（哪怕 checks 全绿、admin 也一样）。上锁
  入口在 Settings→Branches→Lock branch, 易被误开; 无意之锁直接解除。
- **required check 旧名对账**: 分支保护要求名 `"kani"` 而现行检查叫聚合器
  名 → 合并被卡 "Required status check expected"。临时对账 = **Statuses API**
  发一枚同名 legacy status（`POST /statuses/<完整 sha>`, 40 字符 sha,
  短 sha 会 422）; 治本 = 改保护配置或让 job 同名。
- **保护 API 的子端点会 404 而全量 PUT 能通**: `PUT .../protection/
  required_status_checks` 404、`DELETE .../branches/<b>/lock` 404 —— 但
  `PUT .../branches/<b>/protection`（全量回写原值+目标改动）成功。管理端
  操作优先走全量保护 PUT, 读什么写回什么, 只改目标字段。
- 判定链速查: `pulls.mergeable_state` (blocked/dirty) → GraphQL
  `mergeStateStatus + statusCheckRollup`（区分"检查没过"vs"其他封锁"）→
  `branches/<b>/protection` 全字段逐项（lock_branch 藏在字段清单尾部）。
