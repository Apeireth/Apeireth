# 自审报告 — S2 Untrusted 输入标记 + MCP 工具描述投毒防护 (任务 2071b04a)

- 任务 ID: 2071b04a-a273-43ab-9107-fb1988c6e31f
- 角色: security_reviewer2
- 对标: OWASP Agentic Top 10 **ASI-01** (Agent 输入未标记信任边界 / 工具描述投毒)
- 落点: `crates/apeireth-guard` (安全域 crate, 与 PrivacyGuard/pii/redactor 同级)

## 一、交付物

| 交付物 | 路径 | 状态 |
|---|---|---|
| Untrusted 输入标记模块 | `crates/apeireth-guard/src/untrusted_mark.rs` | ✅ 新增 |
| MCP 工具描述投毒防护模块 | `crates/apeireth-guard/src/tool_desc_audit.rs` | ✅ 新增 |
| lib.rs 挂载 + re-export | `crates/apeireth-guard/src/lib.rs` | ✅ 增量 |
| 测试 | 模块内 `#[cfg(test)]` | ✅ 82/82 全绿 |
| 台账 | `docs/backlog.md` S2 行 | ✅ 划✅ |

## 二、机制设计

### 2.1 untrusted_mark.rs — 痛点① 外部输入无标记

- **确定性包装** `wrap_untrusted(source, content)`:
  `<<<[UNTRUSTED_CONTENT source="mcp_tool_result"]>>> … <<<[/UNTRUSTED_CONTENT]>>>`
- **来源注记**: McpToolResult / WebFetch / FileRead / UserPaste / Other — 下游可按来源分级处置
- **逃逸防护** (核心不变量, 实现中修复过一处真实缺陷): 内容中任何 START 前缀
  (`<<<[UNTRUSTED_CONTENT`) 与 END 前缀 (`<<<[/UNTRUSTED_CONTENT`, 含斜杠, 是独立字符串)
  一律插入空格中和 → 内容永远无法"提前闭合边界"逃逸出 untrusted 块,
  也无法伪造带 `source="trusted"` 的假标记; 中和幂等 (二次中和不再变化)
- **trait 口** `UntrustedMarker` + `DefaultUntrustedMarker` (dyn 安全, 注入链挂接口)

### 2.2 tool_desc_audit.rs — 痛点② tool description 无审查可被投毒

- **裁决三级**: Pass / Suspect (待人工复核, 不自动阻断) / Reject (硬拒)
- **四类启发式** (确定性顺序, 同输入必同输出):
  1. 空描述 → Reject (trim 后为空即异常)
  2. 隐藏字符 → Reject: 零宽 (U+200B/C/D/U+2060/U+FEFF) + bidi 控制
     (U+202A-E/U+2066-2069/U+061C) + 其他 C0/C1 控制符 (\t \n \r 豁免);
     findings 上限 8 防刷屏, 细节带码点+字节偏移 (U+XXXX@byteN)
  3. 指令性词汇 → Suspect: 中英注入话术清单 ("ignore previous" / "忽略之前的" /
     "你必须执行" / "do not tell the user" / "秘密地" …)
  4. 越权话术 → Suspect: "绕过审批" / "无需确认" / "提权" / "bypass approval" /
     "exfiltrate" …
- **diff 告警** `description_changed(old, new)`: 归一化 (trim+空白折叠) 后比较 —
  再注册时描述实质变化即告警 (更新再投毒通道); 仅空白差异不告警
- **审查记录留痕** `ToolDescAuditLog`: ring buffer (容量默认 256, 超容挤旧),
  记录 server/tool/裁决/发现数/diff 告警标记, `count_by_verdict` 复盘统计
- **trait 口** `ToolDescriptionAuditor` + `DefaultToolDescAuditor` (注册挂接口)

## 三、验收证据

- `cargo test -p apeireth-guard`: **82 passed; 0 failed** (原有 57 + 新增 25)
- `cargo clippy -p apeireth-guard`: 无 error/warning
- 验收点覆盖对照:
  | 验收项 | 测试 |
  |---|---|
  | 标记包装 | wrap_format_well_formed / source_annotation_all_variants / empty_content_still_well_formed |
  | 逃逸防护 | breakout_with_end_marker_neutralized / breakout_with_fake_source_marker_neutralized / escape 幂等 |
  | 启发式检出 (指令性词汇) | instructional_english_detected_case_insensitive / instructional_chinese_detected |
  | 启发式检出 (越权话术) | privilege_escalation_detected |
  | 隐藏字符 | hidden_zero_width_rejected / hidden_bidi_and_bom_rejected / hidden_c0_control_rejected_but_tab_newline_ok / 上限截断 |
  | 空描述 | empty_description_rejected |
  | 确定性复测 | audit_is_deterministic / wrap_is_deterministic / escape_is_deterministic_and_idempotent_safe |
  | diff 告警 + 留痕 | diff_alert_changed_vs_whitespace_only / audit_log_ring_buffer_and_trace |
  | trait 口 | trait_default_matches_free_fn / marker_object_safe_usage / trait_default_matches_free_fn_and_dyn_safe |

## 四、边界遵守

- ✅ 新模块自包含, 落 apeireth-guard (安全域), 挂接口留 trait 口 **0 装**
- ✅ 不改 MCP 协议本体 / LLM 调用链 / prompt 组装既有代码 (lib.rs 仅 mod 声明 + re-export)
- ✅ 纯函数确定性: 0 随机 0 时间依赖 (时间戳由调用方在接线时注入, 核心结构保持可复现)
- ✅ `#![deny(unsafe_code)]` 延续 crate 纪律

## 五、实现过程如实记录 (含缺陷自曝)

1. **逃逸防护首版缺陷 (自查测试抓出)**: 首版只中和 START 前缀 `<<<[UNTRUSTED_CONTENT`,
   漏了 END 前缀 `<<<[/UNTRUSTED_CONTENT` (含斜杠, 是不同字符串) — 攻击载荷自带 END
   标记可提前闭合边界逃逸。测试 `breakout_with_end_marker_neutralized` 当场抓出
   (END 计数 2≠1), 修复为双前缀分别中和 + 回归通过。**教训登记**: 标记对的首尾变体
   必须逐一枚举中和, 不能假设共享前缀。
2. **测试断言自身缺陷 (同批修复)**: 伪造标记测试曾把合法起始标记也计入断言导致误报,
   改为"全输出中合法 START 前缀仅出现 1 次"的精确断言。
3. 词汇清单为启发式: Suspect 级宁可误报不漏报 (如 "you must execute" 可能命中良性描述),
   硬拒级 (隐藏字符/空描述) 无误报路径; 清单扩展点 = 两个 const 数组, 确定性顺序敏感。

## 六、接线指引 (部署层待办, 0 装 PASS 不含)

- prompt 组装层: MCP 返回/网页抓取/文件读入经 `UntrustedMarker::wrap` 后再拼 prompt
- MCP 注册链: 新服务器/工具注册时对 description 跑 `ToolDescriptionAuditor::audit`,
  Reject 拒注册, Suspect 入 `ToolDescAuditLog` 待人工复核; 再注册跑 `description_changed` 告警
- 两处接线均为部署层决策, 本任务不预接 (边界纪律)
