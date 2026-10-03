# CLI 首次使用引导（`apeireth onboard`）

> 适用版本：v2.0.0-rc.3 起。相关旋钮全集见
> [`user-manual.md`](./user-manual.md)；桌面端首启向导是另一条独立链路
> （`frontend/companion-desktop/src/lib/components/FirstRunWizard.svelte`），
> 两者互不覆盖（见文末「与桌面向导的关系」）。

## 这是什么

`apeireth onboard` 是命令行侧的首次使用引导（OOBE），一次引导完成四件事，
并且**先分流、再配置**：

1. **简单介绍**——Apeireth 是什么、有哪些命令、什么默认关闭；
2. **用户分流**——普通用户 / 专业用户两条轨道：
   - **普通用户**：写入预设档案，使用中**自动调整参数**；
   - **专业用户**：**只做介绍**，不写任何预设 / 档案 / 环境变量；
3. **API 设置引导**——服务商预设（DeepSeek / MiniMax / 其他 OpenAI 兼容 /
   Anthropic）→ 端点 → 模型列表 → 默认模型 → API Key；
4. **自我描述词**——一两句话描述你自己与相处方式，作为身份段带进每次
   CLI 对话。

## 怎么用

```powershell
apeireth onboard                 # 交互式引导（推荐）
apeireth onboard --tier casual   # 预选「普通用户」轨道
apeireth onboard --tier pro      # 预选「专业用户」轨道（只看介绍）
apeireth onboard --show          # 查看当前引导档案与调参状态
apeireth onboard --reset         # 清除引导档案（重走引导）
```

## 普通用户：预设值 + 自动调参

普通用户完成引导后写入引导档案（`<数据目录>/onboarding.json`），带两组预设：

### 推荐配置（六旋钮一键开）

与桌面端「推荐配置」（frontend `RECOMMENDED_CAPABILITY_PRESET`）同名镜像：

| 旋钮 | 预设值 |
| --- | --- |
| `APEIRETH_ENABLE_PROACTIVE_RECALL` | `1` |
| `APEIRETH_ENABLE_PREFERENCE_LEARNING` | `1` |
| `APEIRETH_ENABLE_MEMORY_INJECTION` | `1` |
| `APEIRETH_ENABLE_CONSOLIDATION` | `1` |
| `APEIRETH_ENABLE_REFLEXION` | `1` |
| `APEIRETH_ENABLE_ORGANS` | `1` |

**危险能力绝不进预设**：shell / fetch / 文件写保持默认关闭（fail-closed），
要用请自己显式开（见 user-manual）。

### 预算族 + 自动调参（使用中自动调整参数）

预设基准：上下文 24000 字符 / 回合轮数 6 / 单轮工具调用 12。每次
`apeireth chat` 后回写用量（`<数据目录>/onboarding-tuning.json`），下次
对话前按上一回合用量**确定性自动伸缩**：

| 触发条件（上一回合） | 调整 |
| --- | --- |
| 输入 ≥ 6000 字符 或 输出 ≥ 3000 token | 上下文预算 ×1.5、轮数 ≥ 8 |
| 轮数 ≥ 5（接近基准上限） | 轮数 8、工具调用 ≥ 16 |
| 输入 ≤ 200 字符、≤ 2 轮、输出 ≤ 800 token | 轮数 ≤ 4、工具调用 ≤ 8（快档） |

一切取值钳制在显式区间内：上下文 8000..96000 字符 / 轮数 2..16 /
工具调用 4..32。引导第三步可选「应用预设但关闭自动调参」（预设照写，
预算族不再随用量伸缩）。

## 专业用户：只做介绍

专业用户轨道只输出完整配置参考（Provider 三通道 / 身份 / 预算 / 能力 /
记忆 / 逃生门的全部环境变量），**不写任何文件、不碰任何环境变量**。
之后随时可以用 `apeireth onboard --tier casual` 换轨。

## 存储与隐私

| 内容 | 位置 | 是否机密 |
| --- | --- | --- |
| 引导档案（分流 / 端点 / 模型 / 自我描述词 / 预设） | `<数据目录>/onboarding.json` | 否（**零机密字段**） |
| 自动调参用量快照 | `<数据目录>/onboarding-tuning.json` | 否 |
| API Key | **系统钥匙串**（平台钥匙串优先，自动降级加密文件后端） | 是（永不落盘明文、永不回显） |

数据目录 = `APEIRETH_DATA_DIR`（缺省 `~/.apeireth`）。档案里只记
`key_stored` 布尔值，不记任何密钥形态内容；`apeireth onboard --show`
也永不显示密钥。极端环境下降级到内存钥匙串时，引导会**如实告知**改用
环境变量，不假装已持久。

密钥环境变量兜底（自行配置时）：

```powershell
$env:APEIRETH_API_KEY = "sk-…"        # 通用通道（DeepSeek / MiniMax / 自定义 OpenAI 兼容）
$env:APEIRETH_ANTHROPIC_KEY = "sk-…"  # Anthropic 通道
```

## 优先级与作用范围（0 行为变化承诺）

- **优先级**：显式环境变量（哪怕非法）> 引导档案预设 > 运行时默认。
  档案只补**未设置**的变量，用户 / 桌面端的显式配置永远最高优先。
- **作用范围**：只在 CLI 命令入口激活（`chat` / `dream` / `council` /
  `subagent` / `nightwatch`）。`apeireth gateway serve` 与桌面侧车路径
  **不**激活，与桌面端现有设置链路零冲突。
- **没有档案 = 零行为变化**：所有消费点在无档案时均为 no-op。
- **逃生门**：`$env:APEIRETH_DISABLE_ONBOARDING = "1"` 整体关闭档案消费
  （预设补位 / 自动调参 / 身份注入 / 密钥补位全关）。

## 自我描述词如何生效

引导写入的自我描述词会作为身份系统段注入每次 CLI 对话（`apeireth chat` /
`subagent` 等走 `execute_canonical_cli_turn` 的回合），例如：

```text
【用户自我描述】我是学生，常用中文，回答请简洁可靠。
（主体 local-user / 人设 apeireth —— 来自 apeireth onboard 引导的自我描述词设置。）
```

想换说法：重跑 `apeireth onboard`（或直接编辑档案里的 `self_description`）。

## 与桌面向导的关系

两者是**同一产品面的两条独立入口**，按运行形态自然分工：

| | CLI `apeireth onboard` | 桌面首启向导 |
| --- | --- | --- |
| 载体 | 终端交互 | Tauri 窗口（`FirstRunWizard.svelte`） |
| 用户分流（普通/专业） | ✅ | —（后续可消费同一档案） |
| 自我描述词 | ✅ | — |
| 普通用户预设 + 自动调参 | ✅ | — |
| API Key 落点 | 系统钥匙串 | 系统钥匙串（IPC 注入侧车） |
| 配置生效面 | CLI 命令（gateway 不激活） | 桌面侧车环境注入 |

两者的密钥都只进系统钥匙串；CLI 引导档案（`onboarding.json`）目前只被
CLI 消费，桌面端后续可以按需读取同一档案实现「一次引导、两端生效」。
