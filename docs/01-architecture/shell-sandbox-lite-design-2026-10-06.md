# Shell 沙箱轻量档设计 (Shell Sandbox Lite — Design)

> **日期**: 2026-10-06。**状态**: 设计提案, 待主人拍板后实施。
> **动机**: 真机全套边界测试 (同日志 `engineering-log-2026-10-06.md`) 证实——
> filesystem/search/fetch 各守其策, 唯 shell 以主人本机账号全权运行, 可读全盘、
> 可写任意路径、可对外联网。**"完全放行"预设下, shell 是唯一无遮的一环。**
> 本设计的目标: 给 shell 立一道与 fetch 同级、可独立开关的"圈"。

## 1. 三档总览

| 档 | 机制 | 圈内行为 | 工程量 |
|---|---|---|---|
| **轻量档 (本设计)** | Windows AppContainer + 目录授权 + 无网络能力 + 输出凭据触发器 | shell 只能读写**工作区目录**; 完全断网; 危险命令守门 + 凭据输出拦截 | 中 (1 波) |
| 中度档 | 受限用户账号 + 目录 ACL | 同轻量档, 另可配只读外延目录 | 大 (服务/ACL/打包链) |
| 重度档 | 容器 / 微 VM (libkrun) | 全隔离 | 架构级 (挂起) |

## 2. 轻量档设计

### 2.1 强制点: Windows AppContainer

`crates/capabilities/tools/src/process/windows.rs` 的进程执行层在 CREATE_SUSPENDED
之后、ResumeThread 之前, 为 shell 子进程创建 AppContainer:

- **文件授权 (grants)**: 仅工作区目录 (`%WORKSPACE%` + 子目录) 读写; 不授予任何
  其他路径。AppContainer 默认零访问, 未授权路径一律拒绝——"只读全盘"也随之消失,
  shell 只能看见工作区。
- **网络能力**: **不授予**。AppContainer 无网络能力时 socket 创建即败——
  shell 完全断网 (SSRF 无从谈起)。模型需要联网时走 fetch 工具 (受控仅公网)。
- **降级契约 (fail-closed, 0 装)**: AppContainer 创建失败 → 按配置拒绝执行
  (沙箱开时绝不裸跑), 或显式回退到"拒绝执行 + 明确报错"。
- **平台口径**: Windows 真实现; Linux/macOS 保持现状并在能力报告如实标注
  (沿用 `IsolationRequirement` 的 `Unsupported/Partial/Enforced` 分级)。

### 2.2 输出凭据触发器 (tripwire)

进程输出捕获后 (stdout/stderr), 在既有 `CredentialDisclosureHook` 语义之外加
一层本地规则: 输出含以下模式即标记 `credential_tripwire` 并附审计事件, 同时
**截断该段输出** (0 装: 截断而非放行):

- `sk-[A-Za-z0-9]{16,}` (OpenAI/DeepSeek 形态 key)
- `AKIA[0-9A-Z]{16}` (AWS access key)
- `-----BEGIN .* PRIVATE KEY-----`
- `password\s*[:=]\s*\S+` (明文口令赋值)

### 2.3 危险命令守门 (已有, 保留并扩展)

现有 `ToolGuardrail::verify_shell_command` (拒 `rm -rf /`、`dd if=/dev/zero`
等) 保留; 新增: 显式拒绝 `reg add`、`sc create`、`netsh` 写入类、`attrib +s +h`
等系统修改命令 (白名单否定式, 守门不是沙箱的替代, 是纵深)。

### 2.4 开关与 UX

- 设置页「高级能力」新增 **Shell 沙箱** 开关 (默认**开**, 因为产品定位=桌面伴侣;
  需要系统级 shell 的进阶用户在设置里显式关闭并看到风险提示)。
- 审批卡: 沙箱开启时在 `command_text` 旁显示「沙箱: 工作区限定 + 断网」徽标;
  关闭时显示「未沙箱 (本机全权)」警示徽标——让主人在批准前看见墙的存在。
- 会话级: 沙箱开关为全局旋钮 (注入侧车 env `APEIRETH_SHELL_SANDBOX=1`),
  不随会话预设走 (它是环境能力, 不是会话策略)。

### 2.5 验证计划

- 单测: AppContainer 授予/拒绝矩阵 (工作区内写成、界外写败、socket 创建败)。
- 真机 E2E: 开沙箱 → shell `type C:\Windows\win.ini` 败 + 写工作区成 +
  `curl` 断网 + `rm -rf` 守门拒; 关沙箱 → 现状行为。
- 台账更新 + 装机 e2e 增补沙箱探针。

## 3. 不做的事 (诚实边界)

- 不做"仅公网"的 shell 网络细分 (AppContainer 直接断网, 更严更简);
- 不做 per-session 沙箱档位 (第一版全局旋钮; 会话级留待验证后);
- 不做 Linux/macOS 沙箱 (平台能力如实标 `Unsupported`, 见 §2.1)。

## 4. 实施拆分 (预估)

| 阶段 | 内容 | 量 |
|---|---|---|
| P1 | 设计评审 (本文档过目) | — |
| P2 | AppContainer 基础设施 + 授予/拒绝矩阵单测 | 1-2 天 |
| P3 | ShellTool 接入 + 凭据触发器 + 守门扩展 + env 旋钮 | 1 天 |
| P4 | 前端开关/徽标 + 真机 E2E + 台账 | 1 天 |
