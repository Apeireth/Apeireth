[Document-Meta]
Document:        permission-matrix-and-packs.md
Version:         0.1-DRAFT
Layer:           产品设计 (stage1) — 权限矩阵 + 权限包 + 单人物理签
Last-Modified:   2026-08-15
Status:          🟡 DRAFT (主人已拍板核心决策, 待实施登记)
Author:          主人 + AI 协作者
Source-of-Truth: docs/stage2/stage2-decisions-permission-packs.md (权限包, 2026-07-30)
                 + docs/stage1/inspiration-stage1-2026-07-30.md (§5.3 权限包下放)
                 + 主人 2026-08-15 拍板 (见 §6)
0 主动 commit:   严守
0 装 PASS 严守:  严守 (已实现部分如实标注)

# 权限矩阵 + 权限包 + 单人物理签

> **一句话**: 授权不吝啬, 安全机制守护; 风险分级决定「付多少评审」, 权限包决定「签一次用多久」,
> 监督机制保证「责任自负但全程可查」。

---

## 1. 风险分级 → 席位矩阵 (最终版, 主人拍板)

| 级别 | 评审成本 | 日常动作 | 体感 |
|---|---|---|---|
| Info (0 席) | 0 token | 问候/闲聊/查记忆/读文件/发消息给用户 | 瞬时无感 |
| Low (1 席) | 0 token (Rust 规则) | 写笔记/存记忆/git status | 瞬时无感 |
| Medium (3 席) | 1 次 LLM 宪法评审 | 写文件/改代码/网络读 | 2-5 秒后台 |
| High (5 席) | 包覆盖则放行, 否则物理签 | shell 执行/对外发送/不可逆删除 | 签包后零打扰 |
| Critical (7 席) | 洋葱门直接拦 + 熔断 | 碰 L0/洋葱/自我复制 | 拦下 |

**关键**: 7 强制 advisor 是 **0 token 的 Rust 关键词网** (永远全开, 免费第一张网, 只审动作不审对话);
真正的评审成本只发生在 Medium+ (1 次 LLM 宪法评审), 且 Critical 由结构直接拦 (0 token)。

---

## 2. 只审动作, 不审对话 (原则, 主人拍板)

- 7 关键词网 + 宪法评审 + 审批, 全部**只吃结构化动作摘要** (`action=offer_help, tool=ShellExec, risk=High`),
  **绝不扫描**用户的输入、AI 的回复、记忆的自由文本。
- 依据: sovereignty 三域分离 — **Thought 自由 / Proposal 过哲学键 / Action 过权限层**。
- 后果: 「讨论渗透测试」「科普什么是违法」永不误伤 (它们不产生动作)。

---

## 3. 权限包 (对齐 stage2, 主人拍板更新)

### 3.1 概念

**权限包 = 用户预先签发的「意图集」** (capability): 一次强确认 → 授权一段时间。
学术对应: macaroons / Biscuits (衰减 + 过期); 工程先例: **sudo -v** (输一次密码, 缓存 timestamp_timeout)。

### 3.2 结构

- **范围**: 工具列表 + 路径前缀约束 (如 FileOperator 仅 `~/study`)。
- **有效期**: 永久 (默认日常包) / 限时 (小时) / 单次。
- **预算**: 操作次数上限 (如 50 次/天) — 防「授权失控」的软上限。
- **90 天续签提醒**: 永久包也每 90 天提醒一次 (防「忘了收权」, 主人拍板)。

### 3.3 主人拍板的更新 (vs stage2)

1. **日常包默认永久** (用户可自定义改; 自定义权限包接口是既定方向)。
2. **不可逆类 + 对外类可进包**: 授权后不再每次确认 (责任自负 = sudo 模式); 靠监督机制兜底 (§5)。
3. **High 动作不必每次确认**: 一次强确认 (Windows Hello) 签包后, 包的有效期内不限次。

---

## 4. 单人物理签方案

| 载体 | 起步 | 预留 |
|---|---|---|
| Windows Hello (指纹/人脸) | ✅ 起步 (零成本, 机器自带) | — |
| YubiKey / FIDO2 | — | ✅ 架构留口 (工业标准: 按一下 = 物理在场 + 抗钓鱼) |
| 多人 M-of-N | — | ✅ physical_multisig 已实现 (含 witness_present 亲眼在场字段) |

**单人场景的合理性** (谁守看守人): 用 **时间 + 物理 + 不可逆** 替代第二个人的监督:
- 时间: 冷静期/保质期 (所有包限时或提醒);
- 物理: 签名设备在软件不可达面 (Windows Hello/YubiKey);
- 不可逆: append-only 审计 + Self-Disable 熔断。
- 抗胁迫: sovereignty MockBiometric 已有 CoercionBehavior (胁迫模式: 表面解锁但降权/告警)。

---

## 5. 监督机制 (责任自负的兜底, 主人拍板)

| 件 | 状态 |
|---|---|
| `RecordStore` → `action_stream` (每个工具调用, append-only, 结果脱敏后存) | ✅ 已接入工具桥 |
| `apeireth-arbitration` HASH-SQL 哈希链时间线 (改一条哈希对不上) | ✅ 已有 |
| sovereignty violations 列表 (越界记录, 只增) | ✅ 已接入 |
| **每日摘要** (他今天干了什么, 一目了然) | ⏳ UI 层下一步 |

---

## 6. 拍板记录 (2026-08-15 对话)

1. ✅ 风险分级: Low/Medium 不打扰, High/Critical 才签 (Medium 付 1 次宪法评审)。
2. ✅ 只审动作不审对话 (Thought 自由 / Action 守门)。
3. ✅ 7 关键词 advisor = 0 token 免费网, 不是安全主力 (承认其可绕过性)。
4. ✅ 日常包默认永久 + 90 天强制提醒续签。
5. ✅ 不可逆/对外可进包, 责任自负 (sudo 模式)。
6. ✅ 一次强确认 → 授权一段时间 (sudo -v 先例; Windows Hello 起步, FIDO2 留口)。
7. ✅ 监督机制: append-only 记录 + HASH-SQL + violations + (待做) 每日摘要。

---

## 7. 相关文档

- 权限包原始决策: `docs/stage2/stage2-decisions-permission-packs.md`
- 机制科学依据: `docs/stage1/product-loop-rationale.md`
- 产品闭环: `docs/stage1/product-loop-design.md`
- 代码: `crates/apeireth-companion/src/packs.rs` (权限包) + `src/tool_bridge.rs` (审批+监督) + `src/security.rs` (洋葱门+主权)

---

_End of document._
