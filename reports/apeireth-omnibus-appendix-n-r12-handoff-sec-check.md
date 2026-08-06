# M2.5-SEC — R12 附录 N 安全声明校验

- 任务：`6866bb3a-ab6f-42ca-8a5b-2267af7d5d97`
- 性质：read-only 文档数据校验；未修改代码、草案或工程手册
- 被核对稿：`reports/apeireth-omnibus-appendix-n-r12-handoff-draft.md` (249 行, M1 初稿)
- 依据：`APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` 6003-6241 行 (附录 M 全部) + `reports/r12-baseline-verification-2026-07-30.md` (T1 报告 §2.1 命令 2) + `reports/apeireth-omnibus-appendix-m-r11-wrapup-sec-check.md` (M2.5-SEC 附录 M 模式参照) + `reports/r11-security-review.md` (§1+§2.1-2.8)
- 判定口径：数字或归属范围错位按 P0 文档硬错；措辞可由依据直接支持则为 ✓；安全资产 (R11-SEC-001/002 + V1132 SSRF/语义门禁 + serve.py HTTP 边界) 在附录 N 缺失按 P0 范围漏盖

## 9 项核对表

| # | 草案安全声明 | 判定 | 依据与误差源 |
|---|---|---|---|
| 1 | V1121 fake-KPI detector: keys_present=9, fake_kpi_attempts=3, n_threats=2, gate_passed=False (模块自身), dashboard=yellow (V1138 综合) | ✓ | `r12-baseline-verification-2026-07-30.md` §2.1 命令 2 §3 实测: `keys_present=9, fake_kpi_attempts=3, runner_confusion_attempts=0, v03_v04_confusion=3, n_threats=2, gate_passed=False`; §5 综合 `overall_gate_passed=True, dashboard=yellow`. 字段与模块级 vs 综合级分层口径与附录 M §1.5 "R11-SEC-001 pattern drift 信息性" 完全一致. |
| 2 | V1121 ASI 9 键复用 (主 22:33 复用) — keys_present=9 但 gate_passed=False | ✓ | 附录 M §1.5 "V1121 ASI 9 键复用 keys_present=9, gate_passed=False → dashboard yellow" 与草案 §0/§1.1 一致; 未越界写成"已通过", 守住"不假装"边界. 注意: 草案 §1.1 字段中"runner_confusion_attempts=0 / v03_v04_confusion=3"两组数据未在表格中明列 (M2.5-SEC 附录 M 模式 #3 已知, 数字一致即可), 不构成错位. |
| 3 | dashboard yellow (V1138 综合) = V1121 信息性漂移, 非阻断 | ✓ | 附录 M §5.C row 4 + §1.5 "R11-SEC-001 pattern drift 信息性, 不阻断 R11"; T1 报告 §2.1 命令 2 §5 显式 `dashboard: yellow` + `overall_gate_passed: True`; 草案 §0 + §1.1 + §2.1 row 4 多处标注"信息性, 非阻断"完全支持. |
| 4 | R11-SEC-002 self-claim 补充 4/4 (runner = ASI / V1074 runner self-claim) | ✓ | `r11-philosophy-guardian.md` §3.1 `covered / total: 4 / 4` + 附录 M §1.5 "R11-SEC-002 self-claim 补充 4/4 covered" + T1 报告 §2.1 命令 2 §3.1 实测 4/4. 草案 §0 + §1.1 + §6 硬约束三处引用, 数值一致. |
| 5 | R11-SEC-001 fake-KPI regex 重写 + path traversal + secret-leak 三类修复 (附录 M §1.2 记录) | ❌ **P0 范围漏盖** | 附录 M §1.2 显式写"V1121 security guard v01 (R11-SEC-001 fake-KPI regex 重写 + path traversal + secret-leak)"; `r11-security-review.md` §1+§2.1-2.3 明示三类修复细节. 草案 §0/§1.1/§2.1 row 4/§6 全文搜索**无 R11-SEC-001 字样**, 仅在 §0 提到 R11-SEC-002, R11-SEC-001 三个具体修复维度 (regex 重写 + path traversal + secret-leak) 全部漏盖. 接手团队若要"按 R11 安全事件一致性"复用, 缺一条 R11-SEC-001 锚. |
| 6 | V1132 部署 validator 语义门禁 (canonical_bundle_valid + offline_valid/runtime_valid/passed 三分裂 + 18 跨文件语义断言) | ❌ **P0 范围漏盖** | 附录 M §1.2 显式写"V1132 部署 validator 语义门禁 canonical_bundle_valid (18 跨文件语义断言) + offline_valid/runtime_valid/passed 三分裂; daemon 不可达: runtime_valid=False, passed=False, canonical_bundle_valid=True; daemon probe 全 MISSING". 草案全文搜索 `V1132 / canonical_bundle / runtime_valid` 仅 §5.B row 2 "deploy/ 上线验证 (daemon probe 节点)" 间接提到 "daemon probe" 但**没有任何 V1132 模块级语义门禁细节**: 18 跨文件断言未列, canonical_bundle_valid / offline_valid / runtime_valid / passed 四字段未列, daemon probe "全 MISSING" 状态未列. 接手团队若要复用 V1132 部署节点, 缺一条 R11 已落语义门禁快照. |
| 7 | V1132 SSRF allowlist (_LOOPBACK_HOSTS + _LOOPBACK_PORTS 含 8765, file:// / gopher:// / 169.254.169.254 全拒) | ❌ **P0 范围漏盖** | 附录 M §1.2 "V1132 SSRF 强化 _LOOPBACK_HOSTS + _LOOPBACK_PORTS (含 8765), file:// / gopher:// / 169.254.169.254 全拒; canonical probe 可执行, 外部 host/port 仍拒绝". 草案全文搜索 `SSRF / _LOOPBACK` **零命中**; §5.B row 2 提到 "8765 /health" 端口但**未提 SSRF allowlist 完整机制** (三类 scheme + loopback host/port 唯一许可). 接手团队若要扩展或审计 V1132 入口, 缺一条 R11 已落 SSRF 强化口径. |
| 8 | R11-SEC-001/002 是 R11 安全事件全集 (R11 末 R11-SEC-001 fake-KPI regex 重写 + R11-SEC-002 self-claim 补充) | ❌ **P1 范围漏盖** | 草案 §0/§1.1/§6 提到 R11-SEC-002 但**无 R11-SEC-001**; 两事件作为"R11 安全事件全集"的关系未在 §1.1 / §3 / §6 任一处串联. `r11-security-review.md` §0 + 附录 M §1.5 把两事件并列为 R11 安全模块的两条核心事件, 草案拆开成"只有 SEC-002"印象, 失真. |
| 9 | serve.py HTTP 边界硬化 (Content-Length 1 MiB cap + 100 messages + 32 KiB 单消息 + 非 JSON→415 / 缺 Content-Length→411 / body 超限→413 + OWASP A05 DoS + multipart 旁路) | ❌ **P0 范围漏盖** | 附录 M §1.2 显式写"serve.py HTTP 边界硬化: Content-Length 1 MiB cap + 100 messages + 32 KiB 单消息 + HTTP 边界显式: 非 JSON → 415, 缺 Content-Length → 411, body 超限 → 413; OWASP A05 DoS 防护 + multipart 旁路防护" (M2.5-SEC 附录 M #6 + #7 + P0-2 重点项, 已拆清 415/411/413 各自触发条件). 草案全文搜索 `serve.py / 415 / 411 / 413 / Content-Type / Content-Length / DoS` **零命中**. 接手团队若要审计 R11 入口安全边界, 缺一条已落 HTTP 边界硬化快照. |

## P0/P1 文档硬错及最小修正

1. **(P0) 补 R11-SEC-001 三类修复维度**: 在 §1.1 命令 2 表格新增一行 "R11-SEC-001 三类修复" = `fake-KPI regex 重写 + path traversal + secret-leak` (与附录 M §1.2 字面一致); 或在 §6 硬约束加一条 "R11-SEC-001/002 是 R11 安全事件全集" 串联, 避免失真成"只有 SEC-002".
2. **(P0) 补 V1132 部署 validator 语义门禁**: 在 §1.1 或 §5.B row 2 新增子行 "V1132 部署 validator 语义门禁 (R11 已落, 可继承)" = `canonical_bundle_valid=True (18 跨文件语义断言) + offline_valid/runtime_valid/passed 三分裂; R12 接手 daemon 不可达: runtime_valid=False, passed=False, daemon probe 全 MISSING (docker_path=MISSING / kubectl_path=MISSING)`. 这是 R12 deploy/ ceiling 落点, 缺这条 §5.B row 2 只剩 "8765 /health" 一行, 缺语义门禁快照.
3. **(P0) 补 V1132 SSRF allowlist 完整机制**: 在 §5.B row 2 "deploy/ 上线验证" 行尾或新增 §1.1 子行 "_LOOPBACK_PORTS 含 8765, scheme 仅 http/https, host 仅 loopback, file:// / gopher:// / 169.254.169.254 全拒 (R11 V1132 SSRF 强化)". 不补这条, 接手团队若发现 8765 端口被拒, 缺一条 R11 已落 SSRF allowlist 解释.
4. **(P0) 补 serve.py HTTP 边界硬化**: 在 §0 表格或 §1.1 新增子行 "serve.py HTTP 边界硬化 (R11 已落, OWASP A05 DoS 防护)" = `Content-Length 1 MiB cap + 100 messages + 32 KiB 单消息; HTTP 边界显式: 非 JSON → 415, 缺 Content-Length → 411, body 超限 → 413; multipart/form-data 与 application/x-www-form-urlencoded 全拒 (防 JSON-only schema validation 旁路)`. M2.5-SEC 附录 M P0-2 重点项, 不补这条 §1.1 命令 2/3/4 缺 OWASP A05 入口.
5. **(P1) 模块级 vs 综合级分层口径明确**: §1.1 命令 2 表格 "V1121 fake-KPI detector" 行已有 `gate_passed=False (模块自身), dashboard=yellow (V1138 综合)`, 表述正确但未在 §0 表 "V3 哲学守门 (9 键)" 行同步写明 "V1138 综合 overall_gate_passed=True" 字段 (T1 报告 §2.1 命令 2 §0 显式给出). 建议 §0 加一列 "综合 gate" 标 True, 让模块级 vs 综合级一眼可分; 不改不改也不构成错位, 仅是 M2.5-SEC 推荐的清晰度改进.
6. **(P1) R11-SEC-001/002 安全事件串联**: §1.1 命令 2 标题改为 "V1138 R11 五项不假装 + V3 9 键 + V1121 复用 (R11-SEC-001 fake-KPI regex 重写) + R11-SEC-002 self-claim 补充" 或 §6 硬约束加 "R11-SEC-001/002 是 R11 安全事件全集, R12 接手时两事件都已 LOCKED", 避免拆开成"只有 SEC-002"印象.
7. **(P2) §2.1 row 4 V1121 信息性可加 R12 ceiling 锚**: 草案 §5.A #3 已把 V1121 fake-KPI 列为 🟢 低优 R12 ceiling, 但 §2.1 row 4 描述中**没有"信息性 + 不影响 R11 已落功能"与 §5.A #3 优先级说明的呼应**. 建议 §2.1 row 4 末尾加 "(R12 第 1 周可放最后或留 R13+, 见 §5.A #3)". 不加不构成错位, 仅是闭环.

## 简短结论

1. 9 项中 **4 项完全一致** (#1 V1121 字段 / #2 V1121 9 键复用 / #3 dashboard yellow 信息性 / #4 R11-SEC-002 4/4); **5 项存在 P0/P1 范围漏盖** (#5 R11-SEC-001 / #6 V1132 语义门禁 / #7 V1132 SSRF allowlist / #8 R11-SEC-001/002 串联 / #9 serve.py HTTP 边界).
2. **P0 集中在 4 类安全资产**: (a) R11-SEC-001 三类修复 + (b) V1132 部署 validator 语义门禁 (canonical_bundle_valid + 三分裂 + 18 跨文件断言 + daemon probe MISSING) + (c) V1132 SSRF allowlist + (d) serve.py HTTP 边界硬化 (415/411/413 + A05 DoS + multipart 旁路). 草案是 R12 接手第一步文档化收尾, R12 团队若按 §5.B 6 命令 + §5.A 4 项遗留 + §5.B 4 项 ceiling 推进, 这 4 类资产是"复用 R11 已落安全边界"的硬性锚, 缺任何一条都意味着 R12 接手时要重新读附录 M §1.2 推一遍, 与"任何人能接手"主 00:56 哲学冲突.
3. **P1 是清晰度改进**: #5 模块级 vs 综合级分层 + #6 R11-SEC-001/002 串联 — 不改不构成错位, 但加上能让 R12 团队在 §0 + §1.1 一眼区分"模块自身 gate=False"与"V1138 综合 gate=True"两层, 避免误把 dashboard yellow 读成"五项不假装没通过".
4. **P2 是闭环细节**: #7 §2.1 row 4 与 §5.A #3 优先级呼应, 不加不构成错位.
5. 数字偏差: 9 (keys_present) / 3 (fake_kpi_attempts) / 2 (n_threats) / 0 (runner_confusion_attempts) / 3 (v03_v04_confusion) / 4 (R11-SEC-002) / 9/9 (V3 9 键) / 5/5 (五项不假装) 均能在 T1 报告 §2.1 命令 2 实测输出找到同值, 字段与数值无偏差, 错位仅为资产漏盖与清晰度.
6. 修正 4 类 P0 资产 + 1-2 条 P1 清晰度后, 下一团队 (R12 接手第一步) 可无歧义复用附录 N 作为 R12 安全快照; 不修正, 需回到附录 M §1.2 推 V1132/V1121/serve.py 三类资产, 浪费"第一分钟跑 §5.B 6 命令"约定的预算.
