# INSTALL.md — Apeireth-rust 安装步骤

> **性质**: 接手团队第一份必读——三平台安装（Windows / Linux / macOS）+ 验证步骤。
> **依据**: 我们 2026-07-31 "开干前补齐 4 件套" + rust-toolchain.toml 锁定 Rust 1.97.1 stable。
> **当前基线**: 默认分支 `main`、18-crate 工作区（v2.0.0-rc.1；测试基线 **3662 passed / 0 failed / 21 ignored**，130 套件全量实测，历史口径见 CHANGELOG）；`legacy/` 不参与构建。

---

## 📋 系统要求

| 平台 | 最低版本 | 推荐版本 |
|---|---|---|
| **Windows** | Windows 10 (1903+) | Windows 11 |
| **Linux** | Ubuntu 20.04 / Debian 11 | Ubuntu 22.04+ |
| **macOS** | macOS 11 Big Sur | macOS 13+ |

| 工具 | 最低版本 | 推荐版本 |
|---|---|---|
| **Rust** | 1.97.1 stable | 1.97.1 stable（rust-toolchain.toml 锁定）|
| **Cargo** | 1.97.1 | 1.97.1 |
| **Git** | 2.30+ | 2.40+ |
| **cmake** | 3.20+ | 3.25+（编译 sled）|
| **Python** | 3.11+ | 3.13（PyO3 桥）|
| **SQLite** | 3.35+ | 3.40+（`crates/engine/memory` 持久化）|

---

## 🪟 Windows 安装

### 步骤 1：安装 Rust

```powershell
# 下载并安装 rustup-init.exe
# https://rustup.rs/

# 安装 1.97.1 stable（与 rust-toolchain.toml 锁定一致）
rustup install 1.97.1
rustup default 1.97.1

# 安装必要组件
rustup component add rustfmt clippy rust-src
```

### 步骤 2：安装 Visual Studio Build Tools

```powershell
# 下载并安装 Visual Studio Build Tools 2022
# https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
# 必须勾选 "C++ build tools" + "Windows 11 SDK"
```

### 步骤 3：安装 Git + cmake

```powershell
# 通过 winget 或 choco 安装
winget install Git.Git
winget install Kitware.CMake
```

### 步骤 4：Clone + Build

```powershell
git clone https://github.com/Apeireth/Apeireth.git
cd Apeireth
cargo build --workspace
cargo test --workspace
```

### 步骤 5：验证

```powershell
cargo run --bin apeireth session
# 预期：打印 canonical runtime ready / providers: … 就绪信息后退出（诊断命令）；对话请用 apeireth chat <prompt>（需 APEIRETH_API_KEY）或桌面端
```

---

## 🐧 Linux 安装（Ubuntu/Debian）

### 步骤 1：安装 Rust

```bash
# 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 1.97.1 stable
rustup install 1.97.1
rustup default 1.97.1

# 安装必要组件
rustup component add rustfmt clippy rust-src
```

### 步骤 2：安装系统依赖

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential cmake git pkg-config libssl-dev

# 安装 Python + pip（如需 PyO3 桥）
sudo apt install -y python3 python3-pip python3-dev
```

### 步骤 3：Clone + Build

```bash
git clone https://github.com/Apeireth/Apeireth.git
cd Apeireth
cargo build --workspace
cargo test --workspace
```

### 步骤 4：验证

```bash
cargo run --bin apeireth session
# 预期：打印 canonical runtime ready / providers: … 就绪信息后退出（诊断命令）；对话请用 apeireth chat <prompt>（需 APEIRETH_API_KEY）或桌面端
```

---

## 🍎 macOS 安装

### 步骤 1：安装 Rust

```bash
# 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 1.97.1 stable
rustup install 1.97.1
rustup default 1.97.1

# 安装必要组件
rustup component add rustfmt clippy rust-src
```

### 步骤 2：安装 Xcode Command Line Tools

```bash
xcode-select --install
```

### 步骤 3：安装 Homebrew + cmake

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew install cmake pkg-config openssl
```

### 步骤 4：Clone + Build

```bash
git clone https://github.com/Apeireth/Apeireth.git
cd Apeireth
cargo build --workspace
cargo test --workspace
```

### 步骤 5：验证

```bash
cargo run --bin apeireth session
```

---

## ✅ 验证清单

成功安装后，运行以下命令验证：

```bash
# 1. Build（应该 0 error）
cargo build --workspace

# 2. Test（预期 3662 passed / 0 failed / 21 ignored，130 套件——工作区基线口径，以现场实测为准；历史口径见 CHANGELOG）
cargo test --workspace

# 3. Clippy（应该 0 warning）
cargo clippy --workspace -- -D warnings

# 4. Format（应该 0 diff）
cargo fmt --check

# 5. Hello World（运行时就绪自检）
cargo run --bin apeireth session
```

如果全部通过 = **安装成功**，可以开始贡献代码。

---

## 🐛 常见问题

### Q1: `error: linker 'cc' not found`（Linux/macOS）

**解决**：安装 C 编译器
```bash
# Ubuntu/Debian
sudo apt install build-essential

# macOS
xcode-select --install
```

### Q2: `error: Microsoft Visual C++ 14.0 or greater is required`（Windows）

**解决**：安装 Visual Studio Build Tools 2022（见 Windows 步骤 2）

### Q3: `error: failed to run custom build command for openssl-sys`

**解决**：
```bash
# macOS
brew install openssl
export OPENSSL_DIR=$(brew --prefix openssl)

# Linux
sudo apt install libssl-dev pkg-config
```

### Q4: `cargo build` 慢/卡

**解决**：配置 cargo 国内镜像（可选）
```bash
mkdir -p ~/.cargo
cat > ~/.cargo/config.toml <<EOF
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"

[net]
git-fetch-with-cli = true
EOF
```

### Q5: `cargo test` 失败

**解决**：查看具体错误，可能是：
- Python 版本不符（需要 3.11+）
- SQLite 版本不符（需要 3.35+）
- PyO3 链接失败（需要 Python dev headers）

---

## 📂 下一步

安装完成后：

1. **读 README.md**（顶层入口）
2. **读 CONTRIBUTING.md**（PR 流程）
3. **读 docs/01-architecture/vision.md**（5/30/60 分钟路径起点, docs/archive 体系重构后)
4. **读 ROADMAP.md**（顶层路线图, 8/19 已反映 v1.0.0 实际）
5. **读 docs/03-reference/glossary.md**（术语；v2 现状见 docs/01-architecture/architecture.md）
6. **运行 examples/**（最小 demo, `cargo run --example hello_world` 选具体 example）

---

## 🎛️ 启用高级能力（用户旋钮，2026-09-08 落地）

> 设计原则：**默认拒绝**。每个旋钮 = 主人的一次显式授权；值为 `1` 才生效。
> 全部旋钮均为 env 变量，在启动 CLI/gateway 前设置。0 装说明：逐 token 流式
> 仍为缓冲成帧（冻结 seam），无旋钮。

| 旋钮 | 作用 | 额外成本/风险 |
|---|---|---|
| `APEIRETH_ENABLE_LOCAL_READ_TOOLS=1` | 允许 AI 用 filesystem/search 读本地文件 | 低（只读，工作区为根） |
| `APEIRETH_ENABLE_SHELL=1` | **注册** shell 工具；每次调用走人工审批（`approve`/`reject`） | 高——任意代码执行；审批不可绕过，无审批者则拒绝 |
| `APEIRETH_ENABLE_FETCH=1` | **注册** fetch 工具（仅公网、GET-only、DNS 钉扎）；每次调用走审批 | 中——出站数据 |
| `APEIRETH_COGNITIVE_JUDGE=1` | 每回合 AI 自我评审（低分触发重试/停止） | 每回合 +1 次 LLM 调用 |
| `APEIRETH_COGNITIVE_COUNCIL=1` | 多视角审议（7 advisor 并行，10s/60s 有界） | 每回合最多 +7 次 LLM 调用 |
| `APEIRETH_ENABLE_ORGANS=1` | 装配 9 器官模块（W1/W2 等，LLM 重的器官会真调 LLM） | 视器官而定 |
| `APEIRETH_ENABLE_PREFERENCE_LEARNING=1` | 让 AI 把学到的偏好写回长期记忆 | 写入权交给模型；配 P1-A 准入控制使用更稳 |
| `APEIRETH_ENABLE_PROACTIVE_RECALL=1` | 记忆主动召回：已存记忆按对话线索主动浮现（每次 ≤2 条、有置信度阈值，确定性选择器） | 低——不额外调 LLM |
| `APEIRETH_DISABLE_TYPED_RECALL=1` | 关闭承诺/画像/关系三类记忆的**召回读侧**（写侧不动） | 关掉后这三类记忆不再浮现；默认**开**（写读对称，2026-10-06 修复入库不召回的断链） |
| `APEIRETH_ENABLE_MEMORY_INJECTION=1` | 记忆 overlay 换 donor 反幻觉格式（编号证据清单 + 「禁止说『我记得我们以前聊过』」） | 低——纯渲染切换；默认关（XML 封闭世界格式不变） |
| `APEIRETH_ENABLE_CONSOLIDATION=1` | 每回合后跑确定性记忆整理，提炼洞察落库（稳定 ID 幂等，只从原始证据提炼） | 低——0 模型调用；默认关 |
| `APEIRETH_ENABLE_REFLEXION=1` | 失败闭环：TurnStart 注入历史教训 + AfterTurn 把 Judge 显式否决沉淀为反思 | 低——需 Judge 开启才有信号源；默认关 |

**shell 审批流示例**（开启后）：

```powershell
$env:APEIRETH_ENABLE_SHELL='1'
apeireth chat "用 shell 执行 echo hello" --model deepseek-v4-flash
# 回合暂停 → 输出 approval id（不会直接执行）
apeireth approve --session <sid> --approval <approval-id>   # 主人批准后才执行
# 或 apeireth reject --session <sid> --approval <approval-id> --reason "不需要"
```

**2026-10-10 W2 §4.1 做梦（不是旋钮——显式命令即授权）**：`apeireth dream [--session <id>] [--limit N] [--date YYYY-MM-DD]`。取该会话近 N 条 episodes 为梦境素材（不给 `--session` = 空素材，走引擎"常规认知结构自整定"路径），跑 6 阶段做梦循环（LLM 元思考器 + 确定性降级链，LLM 未配则纯规则浅推演），苏醒把 `DreamReport` 写入 `<data>/diary` 日记（source=`dream`）。**默认不自动跑**——做梦只在这个命令下发生。

**2026-10-10 council 改造（每轮评审器 → 决策环节顾问）**：council 不再按轮常开（深度档 = 深思直答，省下原先每轮开庭的延迟），只在**决策环节**用：A 升级/部署批准、B 高危操作（L3+）、C 待裁冲突批量裁决（空闲时）三个位点随对应模块接入；**D 显式咨询已可用**：`apeireth council "<议题>"`（显式授权 = 显式命令，真顾问裁决链；未配 LLM 会 fail-loud 报错而不是给假裁决）。旋钮：`APEIRETH_COUNCIL_ADVISORS=N`（裁决顾问数 1–7，**默认 3**（7→3 拍板），规范序取前 N：Safety/Performance/Philosophy/History/Strategy/Ethics/Legal，Safety 恒首位）、`APEIRETH_COUNCIL_TIMEOUT_MS`（单顾问超时，默认 30000）。

**2026-10-10 生产 Orchestrator（不是旋钮——显式命令即授权）**：`apeireth subagent "<标题>" [--payload <JSON>]` —— 长程任务 `plan→impl→review` 三步链（IMPLEMENTED → PRODUCTION WIRED：每步**独立 LLM 实例**按角色隔离，`spec.model` 可换 model 做隔离实验）。**人工审批 fail-closed**：plan 步需主人在 CLI 交互点头（y/N），无审批门时自动 deny（绝不静默放行）；dispatch 超时有界（默认 120s）；未配 LLM 显式报错（0 装）。旋钮：`APEIRETH_ENABLE_WORKTREE_SANDBOX=1`（**默认关**）——子代理跑在独立 git worktree（物理目录级隔离，防污染主工作区；worktree 创建失败即拒、不裸跑）。

**2026-10-10 守夜人 Nightwatch（不是旋钮——显式命令即授权）**：`apeireth nightwatch [--session <id>] [--limit N]`。离线闲时审计（**report-only，不阻塞不批准**——approval_policy 留热路径）：读近 N 条 episodes 为被动快照 → 五件组合审计（risk 核词扫描 / eval 行为质量趋势 / no-degrade 复盘 / evidence 断言缺口 / rubric 立场平衡 / colang DSL 健康）→ 报告落 `<data>/nightwatch/nightwatch-<ts>.json`。**默认不自动跑**；审计链/council ballot 未持久化的分析段如实报"接线缺口"，不造假输入。**后台守护（2026-10-10 设计闭环）**：`apeireth nightwatch --watch [--idle 900] [--cooldown 3600] [--interval 60]` —— 只在**用户空闲时**复盘：活动信号 = 认知库 episode 时间戳（用户说话=新 episode，`--session` 指定观测会话），双闸（距最近活动 ≥ idle 且距上次复盘 ≥ cooldown）过后跑一次，轮询间隔 interval；Ctrl-C 退出，报告已落盘无中间态。

**与工具同批的既有旋钮**：`APEIRETH_COGNITIVE_DB`（记忆库路径）、
`APEIRETH_SESSION_DB`（会话库路径）、`APEIRETH_MODEL`（默认模型）、
`APEIRETH_OPENAI_URL`/`APEIRETH_OPENAI_MODELS`/`OPENAI_API_KEY`（provider 配置）。

**2026-10-06 W2 记忆检索旋钮**（不是 `=1` 开关，是配置项）：

- `APEIRETH_EMBEDDING_URL` + `APEIRETH_EMBEDDING_MODEL`（`APEIRETH_EMBEDDING_KEY`
  可选，本地免鉴权端点不设）——记忆检索的**语义向量阶段**（OpenAI 兼容
  `/embeddings` 端点）。**双缺 = 纯词法检索（默认，行为不变）；只设其一 = 启动报错**
  （半配是配置事故，大声失败）；DeepSeek 没有 embeddings 端点，需本地模型或
  其他兼容端点。每回合约 +N 次嵌入调用（候选逐条）。
- `APEIRETH_PERSONA_ID` / `APEIRETH_SUBJECT_ID`——typed 记忆（承诺/画像/关系）的
  主体身份。本地单用户无需设置（默认 `apeireth` / `local-user`）；多主体部署时覆写。
- `APEIRETH_REFLEXION_DIR`——reflexion 反思库目录（默认 `<data>/reflexion`）。

---

_安装指南 v1 修订版（leader 亲自产出）._
_依据我们 2026-07-31 "开干前补齐 4 件套" + rust-toolchain.toml 1.97.1 锁定._
_主哲学 8 锚穿透 (S-1 北极星 / S-2 实事求是 / S-3 质量工程化 NEW / O-1 安全优先 NEW / O-2 走在前人 / O-3 干到底 / O-4 接手 / O-5 不假装; per R125 B5 升 8 锚 + R126 P1-2 实施)._
