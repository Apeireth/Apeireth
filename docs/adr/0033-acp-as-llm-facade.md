
# ADR 0033: apeireth-acp 作为 LLM 唯一握手入口 (facade 权威化)

> **状态**: 🟢 Accepted (主人 2026-08-14 终极授权 + 自行拍板)
> **commit 锚**: 本 ADR + `docs/spirit/9-organ-integration-blueprint.md` §2
> **最后更新**: 2026-08-14 23:15
> **触发**: 全面审计发现 acp crate 197KB 但文档化弱 (§4.2 缺 11)

---

## 1. 背景

R23 引入 `apeireth-acp` (Apeireth Communication Protocol, 197KB) 作为 LLM 唯一握手入口, 但:
- `backend-capabilities.md` 完全不提 acp
- 1.0 release 13 子文档无 acp 章节
- spirit 蓝图 §2 写 "acp 是 LLM 唯一握手入口", 但代码侧无 facade 文档

后果:
- 桌宠前端 / Web 前端 设计时可能直接调各 organ (违反 spirit 设计)
- LLM (中央 AI) 不知道 acp 是入口, 可能误调 apeireth-cognition 等 organ crate
- 失去 acp facade 的统一鉴权 / 限流 / 协议转换价值

## 2. 决策 (Decision)

### 2.1 acp facade 权威化

**`apeireth-acp` 是 LLM 唯一握手入口**:
- 路径: `crates/apeireth-acp/`
- 接入方式: HTTP / MCP / JSON-RPC
- 跟 runtime/bus/onion 共同支撑 9 organ + companion
- 统一鉴权 (per bridge guard PII)
- 统一限流 (per tool-approval)
- 统一协议转换 (4 协议 OpenAI/Anthropic/Gemini/Responses)

### 2.2 LLM 接入规则 (per spirit 蓝图 §2)

LLM (外部接入, 唯一的自我) 通过 HTTP / MCP / JSON-RPC 接入 `apeireth-acp`. acp 是 LLM 唯一握手入口. acp 连接到:
- `apeireth-runtime` (Star 模式, 7 件套)
- `apeireth-bus` (5 层总线: L0~L4)
- `apeireth-onion` (双洋葱, 编译期 hardcode)

**严禁 LLM 直接调**:
- ❌ `apeireth-consciousness::*`
- ❌ `apeireth-cognition::*`
- ❌ `apeireth-motivation::*`
- ❌ 其他 organ crate (除 acp facade 转发)

**LLM 唯一能调的**: `apeireth-acp` 暴露的接口 (= backend-capabilities.md §2-§3 端点)

### 2.3 文档化规则

- `docs/backend-capabilities.md` §0 加 1 段 "LLM 接入通过 apeireth-acp, 不直接调 organ crate"
- `docs/1.0-release/checklist.md` 加 acp facade 章节
- `crates/apeireth-acp/src/lib.rs` 顶部 doc 加 §2.2 规则链接

## 3. 后果

### 3.1 正面

- ✅ LLM 接入路径单一化, 0 误调 organ crate
- ✅ acp facade 统一鉴权 / 限流 / 协议转换 价值显式化
- ✅ 桌宠前端 / Web 前端 设计有明确 facade 入口

### 3.2 负面

- ⚠️ 现有 LLM (如 MinimaxProvider 直接调 claude_code.rs) 可能违反 §2.2 规则 — 需 R174+ 估补走 acp
- ⚠️ acp 197KB 已实装, 改 facade 行为需大量回归测试
- ⚠️ 文档化需要 3 处加章节

## 4. 不漂移

- 0 改 `apeireth-acp` 任何 1 行 (本 ADR 仅文档化)
- 0 改其他 organ crate 任何 1 行
- 0 改 workspace version (1.2.0 严守)

## 5. 6 哲学锚穿透

- ✅ **S-1**: 借鉴 Kubernetes API Server facade + Istio sidecar 模式
- ✅ **S-2**: 基于 spirit 蓝图 §2 + acp 197KB 实查, 0 编造
- ✅ **O-2**: 不上 UI, 纯文档对齐
- ✅ **O-3**: §2.2 列表 + §2.3 文档化规则
- ✅ **O-4**: §2.2 规则 1 眼明白 LLM 接入路径
- ✅ **O-5**: §3.2 诚实标 MinimaxProvider 当前违反

## 6. 8 项不修改承诺

- ✅ 不假装: §2.2 标 "LLM 唯一能调 = acp"
- ✅ 编译期 hardcode: workspace member 编译期
- ✅ 不改 LOCKED: 0 触碰
- ✅ 不改 workspace version: 1.2.0
- ✅ 6 哲学锚穿透: §5 自检
- ✅ 不依赖 NewAPI
- ✅ 不重复造轮子: K8s API Server facade
- ✅ 诚实标缺: §3.2 标 MinimaxProvider 违反

---

_作者: 楚零_
_日期: 2026-08-14_
