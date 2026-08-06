# M2.5-SEC — R11 附录 M §1.2 安全声明校验

- 任务：`804ddd6d-1e8e-4519-ad5f-1d4c772cc960`
- 性质：read-only 文档数据校验；未修改代码、草案或工程手册
- 被核对稿：`reports/apeireth-omnibus-appendix-m-r11-wrapup-draft.md`
- 依据：`reports/r11-security-review.md`、`reports/r11-philosophy-guardian.md`
- 判定口径：数字或归属范围错位按 P0 文档硬错；措辞可由依据直接支持则为 ✓

## 7 项核对表

| # | 草案安全声明 | 判定 | 依据与误差源 |
|---|---|---|---|
| 1 | V1121 security guard v01：R11-SEC-001 fake-KPI regex 重写 + path traversal + secret-leak；56 passed、2 skipped、0 failed、84% line coverage | ❌ **P0 范围错位** | 三类修复与 `r11-security-review.md` §1、§2.1–2.3 一致；数字本身也与 §3.1、§4 一致。但 56/2/0 是 **V1121 + V1132 两个测试文件的联合子集**，84% 是 **V1121 + V1132 两个核心文件的联合覆盖率**（两者各 84%，总计 84%），不是 V1121 单模块专属结果。草案把联合真测放在“V1121 安全守门”单行，归属范围失真。 |
| 2 | R11-SEC-002 ASI self-claim coverage 4/4 | ✓ | `r11-philosophy-guardian.md` §3.1 明示 `covered / total: 4 / 4`、missed 为空。该项未在草案 §1.2 独立成行，而包含于 §1.1 V1138 行的“R11-SEC-002 补充”；数值一致。 |
| 3 | V1121 ASI 9 键复用 | ✓ | `r11-philosophy-guardian.md` §3 明示 V1121 ASI 九键复用、`keys_present: 9`。注意其 `gate_passed: False`，不可写成 V1121 九键门通过；草案仅写“复用”，未越界。 |
| 4 | dashboard yellow（V1138 五不假装 + V3 9 键 + R11-SEC-002 4/4） | ✓ | `r11-philosophy-guardian.md` §1 五项规则均达阈值，§2 V3 为 9/9 且 gate passed，§3.1 SEC-002 为 4/4，§0/§5 dashboard 均为 yellow。yellow 的直接原因口径是 V1121 漂移或 self-test 漏报；V1121 §3 同时为 `gate_passed: False`。 |
| 5 | V1132 `_LOOPBACK_HOSTS` + `_LOOPBACK_PORTS`（含 8765），file:// / gopher:// / 169.254.169.254 全拒 | ✓ | `r11-security-review.md` Dashboard、§2.4 完全支持；端口表明确含 8765，scheme 仅许 http/https，host 仅许 loopback，因此三类目标均拒绝。草案附带“外部 host/port 仍拒绝”也与 allowlist 逻辑一致。 |
| 6 | serve.py：Content-Length 1 MiB cap + 100 messages + 32 KiB 单消息 + Content-Type 415/411/413 | ❌ **P0 范围错位** | 三个上限与 `r11-security-review.md` Dashboard、§2.7 一致。但“Content-Type 415/411/413”错误归并状态码：§2.7 的实际映射为 **非 JSON Content-Type→415；缺 Content-Length→411；body 超限→413**（非法长度→400，消息数量/单条超限也→400）。建议改为“HTTP 边界显式返回 415/411/413”。 |
| 7 | OWASP A05 DoS 防护 + multipart 旁路防护 | ✓ | `r11-security-review.md` §2.7 将 body cap 定位为 A05:2021 Security Misconfiguration / DoS；§2.8 明示拒绝 `application/x-www-form-urlencoded` / `multipart/form-data` 以防绕过 JSON-only schema validation；§5 A05 行再次确认。 |

## P0 文档硬错及最小修正

1. **测试/覆盖归属错位**：将第 1 项真测状态改为：`V1121 + V1132 R11-SEC 联合子集：56 passed, 2 skipped, 0 failed；两核心文件各 84%，合计 84% line coverage`。若必须保持 V1121 单模块行，则不得把联合数字无说明地归给 V1121。
2. **HTTP 状态码触发边界错位**：将 `Content-Type 415/411/413 显式` 改为 `HTTP 边界显式：非 JSON→415、缺 Content-Length→411、body 超限→413`。
3. **无数字偏差**：56、2、0、84%、4/4、9、8765、1 MiB、100、32 KiB 均能在原报告找到同值；硬错仅为上述归属/范围表达。

## 简短结论

1. 七项中 **5 项完全一致，2 项存在 P0 文档范围错位**。
2. P0-1 不改数字，只需声明 56/2/0 与 84% 是 V1121+V1132 联合口径。
3. P0-2 不改状态码，只需拆清 415、411、413 的各自触发条件。
4. V1121 九键是“复用但 gate=False”，不得交接成已通过。
5. dashboard yellow、SEC-002 4/4、SSRF allowlist、A05/DoS 与 multipart 防绕过均可原样接手。
6. 修正两处措辞后，下一团队可无歧义复用该安全快照。
