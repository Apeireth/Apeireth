# 权限面边界说明（三层权限面区分）

> `Status: 🟢 活跃（2026-10-19 立，安全加固批 B4）`
> 命题：**「Shell 命令工具已关闭」关的只是智能体工具面**——桌面手动终端（舵室）是**用户直操面**，
> 实际权限高于「shell 已关闭」给人的印象。本文是三层权限面的单一口径说明；
> 终端面板的权限边界提示文案与本文同口径（落点见 §三）。
> 执行语义本文一字不改（那是 `terminal_exec.rs` 契约的事）——这里只把**边界说清楚**。

## 〇、一句话

治理开关与审批链只管**智能体工具面**；桌面手动终端以**当前 Windows 用户权限**执行、不走审批链；
「工作目录钉死」只是进程当前目录，**不是操作系统沙箱**。三件事是三个面，不要互相代入。

## 一、三层权限面（不是一个面）

| 层 | 面 | 权限来源 | 受治理开关 / 审批链？ | 受沙箱保护？ |
|---|---|---|---|---|
| 1 | 智能体工具面（模型可调用的工具） | 工具注册 + 授权（能力开关） | **受**：开关注册/授权 + 人工审批链（审批族归一口径） | shell 另有沙箱姿态开关分层（见[开关权力审计表](capability-switch-power-audit.md)） |
| 2 | 桌面用户直操面（舵室手动终端） | 当前 Windows 用户账户既有权限 | **不受**：用户自己动手，本面不代批、不代办 | **无** |
| 3 | 工作目录（`workspace_dir`） | ——（只设子进程当前目录） | —— | **非沙箱** |

### 1. 智能体工具面（受治理）

模型可调用的工具（shell / fetch / 文件写入等）由能力开关管辖：默认关（需 env 显式开启）、
开启即注册 + 授权 + 每次调用走人工审批链；shell 工具的权力还由沙箱姿态开关与文件写入总闸
分层管辖——「开关名 = 开关权」的逐行口径见
[capability-switch-power-audit.md](capability-switch-power-audit.md)，
默认开关现值见 [capabilities-matrix.md](../03-reference/capabilities-matrix.md) §0.2。
**这一层就是 `APEIRETH_ENABLE_SHELL`（Shell 命令工具开关）所管辖的全部范围。**

### 2. 桌面用户直操面（舵室手动终端，以当前用户权限执行）

用户在桌面终端面板亲自键入命令执行，执行语义是平台壳常规语义（Windows `cmd /C`，
其余平台 `/bin/sh -c`，见 `frontend/companion-desktop/src-tauri/src/terminal_exec.rs` 的
`shell_invocation`）。命令以**当前 Windows 用户权限**执行——就是该用户账户本身的既有权限，
无提权、无降权。

**该面不走智能体审批链、不受治理开关管辖**：`enable_shell=false`（`APEIRETH_ENABLE_SHELL=0`）
关闭的是**智能体工具面**，不影响此处。产品语义定位 = **用户直接操作面**（用户自己动手的常规
终端语义）；智能体驱动的命令执行必须走审批族（审批桥→审批管理→权限包链路，见
[E-approval-family-contract.md](../04-internal/backend-enhancement/E-approval-family-contract.md)），
**接线缝：未接线**（届时点亮，本面绝不代批、绝不代办）。

### 3. 工作目录不是沙箱

`workspace_dir` 的校验只保证「绝对路径 + 真实存在为目录」（相对 / 越界 / 不存在一律拒绝），
作用仅是把该目录设为子进程的当前工作目录（`current_dir`）。**它不是操作系统沙箱**：
命令可以访问工作目录之外一切用户可访问的资源（文件 / 网络 / 进程），不提供任何隔离。
「工作目录钉死」≠「被关在工作区里」——把它读成沙箱是误读。

## 二、超时杀进程的边界（如实，含树杀缺口）

- **30 秒超时强杀只杀直接子进程**（显式 kill + `kill_on_drop` 兜底）；进程树杀是后续批的
  接线缝（**树杀待接**）——超时后**孙进程可能遗留存活**，并继续占用输出管道。
- 孙进程占管道时输出排水有 **2 秒宽限**，超限即弃取并把 `truncated` 如实标注为真；
  输出合计超 **256KB** 截断并如实标注——不装全量。
- 未取得退出码时（超时强杀 / 信号终了）退出码如实外显为空（`code = null`），
  界面显示「非正常终了」——不假成功。

（数字出处：`frontend/companion-desktop/src-tauri/src/terminal_exec.rs` 常量
`COMMAND_TIMEOUT` / `DRAIN_GRACE` / `OUTPUT_BUDGET_BYTES` 与契约注释 §4-§5，2026-10-19 读取。）

## 三、文案与实现落点（同口径三处）

1. **面板提示文案**：`frontend/companion-desktop/src/lib/starport/apps/terminal-app.ts`
   （面板头部权限边界行），与本文同口径；文案由
   `frontend/companion-desktop/tests/starport-face-apps.mjs` 源镜像钉住；
2. **执行实现与契约**：`frontend/companion-desktop/src-tauri/src/terminal_exec.rs`
   （`run_workspace_command`，契约注释 §1 用户直操面 / §4 超时强杀）；
3. **治理开关口径**：[capability-switch-power-audit.md](capability-switch-power-audit.md)
   （开关名 = 开关权）+ [capabilities-matrix.md](../03-reference/capabilities-matrix.md) §0.2。
