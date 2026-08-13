# R188 GitHub 优秀项目调研 — core / central / bus (核心子系统)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R188
> **日期**: 2026-08-13
> **范围**: apeireth-core (7 文件 125KB) + apeireth-central (11 文件 177KB) + apeireth-bus (7 文件 87KB)
> **状态**: 调研为升级预备.

---

## 0. 现状

### apeireth-core (7 文件 125KB)
- lib.rs (88KB) — 2975 行单文件, 整个 crate 中心
- eight_anchors.rs (23KB) — 8 哲学锚 (S-1..O-5)
- philosophy.rs (7KB) — 哲学评分
- onion.rs (4KB) — 双洋葱结构
- gate.rs (6KB) — gate 守门
- lifecycle.rs (4KB) — 生命周期
- memory.rs (2KB) — core 内部 memory

### apeireth-central (11 文件 177KB)
- lib.rs (46KB) — 中央调度
- skill_*.rs (8 文件 130KB) — skill 系统 (companion/execution/frontmatter/outcome/prompt/recommender/registry/runner/trait/validation)
- 中央调度 + skill 注册 + skill 执行 + skill 推荐 + skill 验证

### apeireth-bus (7 文件 87KB)
- channel.rs (13KB) — 消息通道
- l0-l4.rs (5 文件 60KB) — 5 层消息总线
- lib.rs (13KB)

**已实现**:
- core 8 哲学锚 + 13 键 verdict + 30 维 V0.5 + 双洋葱 + 生命周期
- central skill 体系 (注册/执行/推荐/验证/companion)
- bus 5 层 L0-L4 消息总线

**已经领先**:
- core 哲学 + ASI 评分业界独一档
- bus 5 层结构比大多数 LLM agent 的单 channel 细

---

## 1. 核心库 (Rust 生态)

### 1.1 tokio (tokio-rs/tokio) — **基础设施**

- 30K+ stars, MIT
- 异步运行时
- 我们已经用

### 1.2 serde (serde-rs/serde) — **基础设施**

- 我们已经用
- 序列化基础

### 1.3 async-trait / async-channel / dashmap — 基础设施

- 我们已经用

### 1.4 anyhow / thiserror — 错误处理

- 我们已经用

---

## 2. 消息总线 SOTA

### 2.1 NATS (nats-io/nats-server) — **RECOMMENDED 学习**

- **Stars**: 16K+
- **License**: Apache 2.0
- **定位**: 轻量高性能消息系统
- **核心能力**:
  - Pub/Sub + Request/Reply + Queue Group
  - JetStream 持久化
  - Subject-based routing
  - 跨语言 (Rust client: async-nats)
- **学习点**: 我们 bus L0-L4 借鉴 subject routing

### 2.2 Redis Pub/Sub (再) — 学习

- 简单 pub/sub
- 我们 bus 5 层结构比它细

### 2.3 Apache Kafka (再) — 不集成

### 2.4 ZeroMQ (再) — 学习

- 低层 socket
- 我们的 channel.rs 类似抽象

### 2.5 MPSC / tokio::sync — 基础设施

- 我们已经用

### 2.6 Crossbeam — 学习

- 无锁并发
- **学习点**: bus 内部 channel 优化

### 2.7 Bevy ECS (bevyengine/bevy) — **学习 (ECS 模式)**

- **Stars**: 38K+
- **License**: MIT/Apache-2.0
- **核心**: Entity-Component-System
- **学习点**: 我们 bus L0-L4 可以借鉴 ECS 解耦
- **价值**: bus 5 层 → 单一 event bus + tag system

### 2.8 actix / actix-rt — 备选

- actor model
- 我们不用

---

## 3. 中央调度 / Skill 体系 SOTA

### 3.1 LangChain Tools / Anthropic Skills — **学习**

- LangChain Tool abstraction
- Anthropic Skills (新) — Agent Skills 模式
- **学习点**: 我们 central skill_trait.rs 类似

### 3.2 OpenAI Functions / Tools (Function Calling) — **学习**

- 我们 tool 体系已经对齐
- **学习点**: schema 标准化

### 3.3 Claude Skills (Anthropic, 2025+) — **RECOMMENDED 学习**

- Skills = 可加载的 prompt + tool + knowledge 集合
- 渐进式披露 (progressive disclosure)
- **学习点**: 我们 central skill_frontmatter + skill_registry 借鉴

### 3.4 OpenAI GPTs (gpts-store) — 学习

- 商业 skill 平台
- **学习点**: 评分 + 反馈

### 3.5 Hugging Face Agents / Spaces — 学习

- 多 skill 组合

---

## 4. 类型系统 / 哲学工程化 SOTA

### 4.1 我们 8 哲学锚 + 13 键 + 30 维 — **业界独一档**

- 没看到其他 LLM 项目有类似设计
- 学术参考: NIST AI RMF (R178 提过)
- 工业参考: ISO/IEC 42001 (AI 管理体系)

### 4.2 typed-builder / derive-builder — 学习

- Rust 类型安全 builder
- 我们 onion.rs / gate.rs 可用

### 4.3 sealed / enum_dispatch / downcast — 学习

- 静态分发 + 类型擦除
- 我们 gate.rs 13 键 verdict 借鉴

### 4.4 bevy_ecs / hecs — 学习

- ECS 数据流
- **学习点**: 我们 central 状态管理

### 4.5 Rust traits + PhantomData — **学习**

- Zero-sized types (ZST) 模式
- **学习点**: 我们哲学 anchor 类型化

---

## 5. 编译期检查 SOTA

### 5.1 const-eval / const generics (Rust 1.70+) — 基础设施

- 我们哲学 30 维 sum=1.00 编译期守门已经用

### 5.2 compile-time checks via type system

- typenum / generic-array
- **学习点**: 我们 13 键 verdict cache key 类型化

### 5.3 Kani (再, R178 提过) — 形式化

- 我们已经在用

### 5.4 Prusti / Creusot (再) — 备选

---

## 6. 升级方案 (R188+ 实施)

### 6.1 短期 (1-2 days)

1. **bus subject routing**: 借鉴 NATS, 我们 L0-L4 加 subject-based 路由
2. **bevy_ecs 风格 event bus**: 单一 bus + tag 替代 5 层结构 (评估)
3. **typed-builder 应用**: onion.rs / gate.rs builder 模式

### 6.2 中期 (3-5 days)

4. **Anthropic Skills 渐进式披露**: central skill_frontmatter 升级
5. **Crossbeam 优化**: bus 内部 channel 性能
6. **typed enum dispatch**: 13 键 verdict key 静态分发

### 6.3 长期 (持续)

7. **NATS JetStream 借鉴**: bus 持久化 (如果需要)
8. **Bevy ECS 完全重构**: central 状态管理 (如果需要)
9. **形式化证明 13 键**: Kani

---

## 7. 依赖增量

| crate | 体积 | License | 必需 |
|---|---|---|---|
| async-nats (评估) | ~500KB | Apache 2.0 | 长期 |
| crossbeam (评估) | ~200KB | MIT/Apache-2.0 | 中期 |
| typed-builder | ~30KB | MIT/Apache-2.0 | 短期 |

**总增加**: < 250KB (短期), ~700KB (中长期)

---

## 8. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| core | 所有 crate 依赖 |
| central | 中央调度 + skill |
| bus | 消息总线 (cross-crate) |
| consciousness (R187) | core 6 状态 + bus L1 |
| council (R180) | bus L2-L3 协调 |
| pipeline (R184) | bus L4 完成通知 |

---

## 9. 0 触碰声明

- 3 不可变脊柱: 0 触碰 (注意: core 包含 13 键 verdict 语义, 这是 3 脊柱之一 — 调研不动它)
- workspace.version 1.2.0: 0 改
- core 公开 API: 0 改 (新能力在子模块内)

---

## 10. 参考链接

- tokio: https://github.com/tokio-rs/tokio
- serde: https://github.com/serde-rs/serde
- NATS: https://github.com/nats-io/nats-server
- async-nats: https://github.com/nats-io/nats.rs
- Bevy ECS: https://github.com/bevyengine/bevy
- actix: https://github.com/actix/actix
- Crossbeam: https://github.com/crossbeam-rs/crossbeam
- Anthropic Skills: https://www.anthropic.com/news/skills
- LangChain Tools: https://python.langchain.com/docs/concepts/tools
- typed-builder: https://github.com/idanarye/rust-typed-builder
- typenum: https://github.com/paholg/typenum
- Kani: https://github.com/model-checking-ai/kani