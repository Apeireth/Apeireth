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
