# Recon Suite（2026-10）

> 状态：能力层完成，生产装配（`ProductionModules`）**未接线**，默认零行为变化。

## 1. 组成

### 原生工具（9，`crates/capabilities/tools/src/recon/`）

| 工具 | 能力 | 主要边界 |
|---|---|---|
| `re.pack` | 字符串、Shannon 熵、魔数、嵌入雕刻、壳线索 | 静态，不执行目标 |
| `re.binfmt` | ELF/PE/Mach-O/fat；节、段、导入、导出、notes、PE 版本资源 | 不反汇编 |
| `re.dotnet` | CLR 元数据：TypeDef/MethodDef/TypeRef/AssemblyRef | 不解析 IL/签名 |
| `re.js` | 词法级 JS/Electron：字符串、imports、sourcemap、eval 迹象 | 非 AST |
| `re.yara` | YARA 子集：修饰符、条件、hex 跳转/分支、正则、pe/elf/hash 模块 | 非完整 YARA |
| `net.recon` | TCP connect + 被动 banner | 无 raw socket |
| `web.probe` | HTTP 指纹 + 安全头 | 不跟随重定向、不读 body |
| `net.proto` | pcap/pcapng → 流 + DNS | 不重组 TCP、不解密 |
| `re.tls` | 主动 TLS 握手（版本/密码套件/ALPN/SNI） | 不读证书链 |

共享：`Evidence` 信封、fail-closed `Scope`、`ToolPlatform`。

### 外部适配器（10，`crates/capabilities/tools/src/external.rs`）

`ext.nmap`（含 `-sS` SYN 选项）、`ext.exiftool`、`ext.tshark`、`ext.yara`、`ext.binwalk`、
`ext.radare2`、`ext.jadx`、`ext.apktool`、`ext.ghidra`、`ext.openssl`（证书链）

- 统一走 `ProcessExecutor`：argv 数组、无 shell 插值、超时/输出上限
- 网络目标过 `Scope`，文件目标限域 workspace
- 只读档：nmap 用 `-sT`，radare2 固定 `aaa; aflj`，不接受任意命令
- 外部二进制缺失时返回永久错误

## 2. 启用方式（2026-10-22 接线批：**生产装配已接线 · PRODUCTION WIRED**）

三个环境变量，默认全关（fail-closed）：

| 开关 | 作用 | 缺省 |
|---|---|---|
| `APEIRETH_ENABLE_RECON=1` | 注册 9 个 `tool.recon.*` 原生侦察工具 | 关 |
| `APEIRETH_ENABLE_EXTERNAL=1` | 注册 10 个外部适配器（能力 id `tool.external.*`，模型侧名 `ext.*`） | 关 |
| `APEIRETH_TOOL_SCOPE=/path/scope.json` | 操作员授权范围（路径 / 域名 / CIDR） | 空范围（网络全拒） |

scope 文件格式：

```json
{ "name": "my-lab", "authorized": true,
  "paths": ["/home/bai/Projects/targets"],
  "domains": ["example.com"], "cidrs": ["10.0.0.0/8", "127.0.0.1/32"] }
```

- 缺省不配 = 空范围：网络类工具（`net.recon` / `web.probe` / `re.tls` /
  `ext.nmap` / `ext.openssl`）在每个目标上被 `Scope::validate` 拒绝；文件类工具
  仍受 workspace 根限域。
- 文件缺失 / JSON 坏 / 未显式 `authorized: true` / CIDR 畸形 = **启动失败并给可读
  错误**，绝不静默降级为空范围或全放行。
- 开关语义真源：`crates/capabilities/tools/src/recon/wiring.rs`（装配侧注册与
  治理侧 grant 读同一函数，不各解一份 env）。

库层仍可用：`BuiltinToolsOptions { recon: Some(ReconConfig { scope }) }` /
`BuiltinToolsOptions { external: Some(ExternalToolsConfig { ... }) }`。

装配点：`ProductionModulesConfig.recon` / `.external`
（`crates/engine/runtime-assembly/src/canonical/production.rs`），
CLI 组装修：`crates/adapters/cli/src/env_switches.rs`。

## 3. 治理

- Scope：路径 canonicalize、域名/子域、IPv4/IPv6 CIDR；空 scope 拒绝一切网络目标。
- 授权档位（**按爆炸半径分档，不按便利**）：
  - 只读本地分析 6 件（pack/binfmt/yara/dotnet/js/pcap）= **grant 即用零审批**
    （与 `tool.repo` / `tool.education` 同档：零出网、零执行、workspace 限域）；
  - 出网 3 件（net/web/tls）+ 外部适配器 10 件 = **grant + require_approval**，
    每次调用停人工审批（前者对外发报文，后者执行第三方二进制）。
- 风险声明：文件类 `low`；`net.recon`/`web.probe`/`re.tls`/`nmap`/`ghidra` `medium`。
- 外部工具不链接 GPL/AGPL，只经进程边界；High 风险默认不提供（如 sqlmap/hydra）。

## 4. 验证

- `cargo test -p apeireth-tools-canonical --lib recon` → 102 passed
- `cargo test ... --lib external` → 16 passed
- `--test recon_differential` → 与 readelf 差分 + 真实系统二进制（3）
- `--test recon_fuzz` → 3200 组随机输入 panic-free（2，含溢出回归）
- clippy `-D warnings`、fmt 通过

## 5. 已知差距

| 差距 | 处理 |
|---|---|
| YARA 完整兼容（全部模块/语义） | 不自研；`ext.yara` 走系统 YARA |
| SYN 扫描 | 不自研；`ext.nmap` 提供（`-sT` 当前，SYN 需 root 可另行开启） |
| 反编译器（IDA/JEB/Ghidra 级） | 不自研；`ext.ghidra` headless |
| TLS 证书链 | `ext.openssl`（链不校验证书信任） |
| JS 完整 AST | 长尾待办（需要时用 swc parser） |
| YARA 正则/模块 | 子集；lookaround、反向引用、Unicode 属性不支持 |
| .NET IL/签名 | 未解析 |
