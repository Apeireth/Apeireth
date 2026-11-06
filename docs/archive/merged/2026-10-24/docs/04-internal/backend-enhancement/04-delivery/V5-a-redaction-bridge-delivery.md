```yaml
[Document-Meta]
Document:        V5-a-redaction-bridge-delivery.md
Version:         1.0 (V5-a 出境脱敏双向闸交付说明: 占位格式规格 + 双闸时序图 + 测试证据)
Last-Modified:   2026-10-17
Status:          🟢 活跃 (交付随行文档)
Author:          子代理 V5-a 施工线
```

# V5-a 出境脱敏双向闸 — 交付说明

> 战场: `crates/capabilities/tools/src/redaction_bridge.rs`(新件) +
> `crates/capabilities/tools/src/guardrail.rs`(升级) +
> `crates/foundation/governance/src/input_security.rs`(升级)。
> 机制假说出处: [安全守望族机制卡 §3](../../philosophy-study/04-security-guard-family.md)
> + [V-forward-direction.md](../V-forward-direction.md) V5 行。
> 硬不变量: 占位不可逆 + 还原双闸 + 历史回扫 + 账本零真值。

## 1. 占位格式规格

```text
placeholder := "__AP_" CATEGORY "_" FINGERPRINT12 "__"

CATEGORY      := "EMAIL" | "PHONE" | "CREDENTIAL_KEY" | "SSN"
               | "CREDIT_CARD" | "IP_ADDRESS" | "CREDENTIAL_URL" | "ENV_SECRET"
FINGERPRINT12 := 12 位小写十六进制
              := hex12( HMAC-SHA256(会话随机密钥, 真值)[0..6] )
```

| 性质 | 定义 | 钉住测试 |
|---|---|---|
| 同会话同值恒同占位 | 指纹是密钥化 MAC, 同密钥同输入恒同输出 | `placeholder_determinism_same_value_same_alias_and_fixed_point` |
| 跨会话换别名 | 指纹随会话随机密钥 (每会话 32 字节) 变化 | `placeholder_aliases_differ_across_sessions` |
| 不可逆 | 占位只含 类别+12 位截断 MAC, 无真值槽位; 无密钥/登记簿不可反推 | `placeholder_is_irreversible_without_registry` + `placeholder_token_carries_no_true_value` |
| 替换面只认占位符族 | 严格全串解析; `[REDACTED_*]` 等字面量永不入还原面 | `placeholder_parse_is_strict_family_only` |
| 保真往返 | 登记值 出境→回程 = 逐字同一 | `restore_happens_only_on_local_surface_with_exact_roundtrip` |

- HMAC-SHA256 由 `sha2` 单包组合实现 (RFC 2104 构造), **0 引新外部依赖**;
  RFC 4231 向量 1/2/6 (含长密钥分支) 钉在 `session_fingerprint_matches_rfc4231_hmac_sha256_vectors`。
- 跨会话关联性被有意放弃 (换会话即换别名), 换取对出境面不可逆。

## 2. 双闸时序图

```text
 出境 (每一轮)                                   受控回程 (本地落盘前)
 ─────────────                                  ─────────────────────
 历史段 [工具轨迹/会话历史, 存真值]               模型结果 (携带占位符)
   │ ① 历史回扫 rescan_history                     │ ④ 还原 restore_for_local
   │    真值面 → __AP_<类>_<fp>__                  │    闸一: 只在受控回程还原
 当前文本                                         │    闸二: fp ∈ 本会话登记簿
   │ ② 出境脱敏 redact_for_egress                  │      ├ 命中   → 还原真值
   │    真值 → 占位符 + 登记簿 + 账本               │      └ 未登记 → 照实留痕
   │ ③ (可选) 既有凭据绊线补单向标签                │         (账本记 skipped, 不还原)
   ▼                                              ▼
 出境文本 (只见占位族 [+ 单向标签])               本地文本 (真值) + 账本
```

- **闸一 (受控回程)**: `restore_for_local` 是全库唯一占位→真值出口; 出境面
  (`redact_for_egress` / `rescan_history`) 永不吐真值。
- **闸二 (登记簿匹配)**: 只还原本会话登记簿登记过的指纹; 跨会话占位符、
  伪造占位符一律不还原、文本照实保留、账本留痕。

## 3. 历史回扫 (堵历史搭车)

工具轨迹/会话历史以**真值**形态存储, 明文可能来自早年轮次。每次出境前调
`rescan_history(&mut [String])` 逐段就地回扫: 检测器命中 + 本会话已登记值
的裸/变体形态全部换占位 (命中打 `history_rescan` 面标签)。重复回扫幂等
(输出是固定点), 明文永不复活。

## 4. 变体绕过防御面 (0 装: 只挡声明的族)

对**本会话已登记的值**额外清扫 (换回同一占位符):

| 族 | 形态 | 钉住测试 |
|---|---|---|
| V1 | 裸形态逐字 | `explicit_registration_preseeds_alias_for_later_egress` |
| V2 | base64 (标准字母表, 带/不带 `=` 填充) | `encoding_variant_bypass_defended_in_history` |
| V3 | 百分号编码 (大写/小写 hex) | 同上 |
| V4 | 十六进制串 (大写/小写) | 同上 |
| V5 | 拆分串 (字符间插空白/零宽字符, 归一影子串扫描) | `split_string_bypass_defended_in_history` |

**已知边界 (不声称防御)**: 未登记值的变体形态、多层编码链、字符集转写
(全角/音译)、值内分隔符替换。检测器盲区由后续阶段接, 本件不假称覆盖。

## 5. 脱敏账本 (redaction.jsonl)

一行一条 JSONL (`LedgerEntry`), 字段: `ts_unix_ms` / `category` / `fingerprint`
/ `event`(redacted | restored | restore_skipped_unregistered) / `restored`
/ `surface`(egress | history_rescan | local_restore | explicit_registration)。
**零真值入账**: 条目只有类别与 12 位指纹, 无真值槽位; `Debug` 面 (桥/登记簿/
还原报告) 同样永不打印真值。钉住测试:
`ledger_records_events_with_zero_true_values` /
`ledger_persists_to_redaction_jsonl_without_plaintext` /
`debug_output_never_leaks_true_values`。

## 6. 与既有脱敏共存 (零回归)

- 既有 `[REDACTED_*]` 单向标签**不属占位族**: 经双向闸不改不还原
  (`coexists_with_legacy_one_way_redaction` /
  `legacy_redaction_labels_pass_through_bridge_unchanged`);
- guardrail 出境双层扫 `ToolGuardrail::sweep_output_for_egress`: ①占位族脱敏
  (登记可还原) ②既有凭据绊线 (单向), 同一份输出两族共存
  (`guardrail_combined_sweep_covers_both_families`);
- 既有 194 套件口径全量回归见 §8 校验真账。

## 7. 顺手修报备 (确证级泄漏债, 本战场内)

- **PEM 私钥块只换头部**: 既有绊线把 `-----BEGIN ... KEY-----` 头部换
  `[REDACTED_PEM_PRIVATE_KEY]` 后, base64 私钥体仍留在输出 —— 确证级泄漏。
  已整块收口 (头部→END 行), 无 END 行的截断块退化为旧行为并 0 装注明;
  钉 `pem_block_redaction_covers_key_body_not_just_header`, 既有 PEM 测试
  零改动通过。
- **占位符族豁免**: 口令赋值扫描对 `password=<占位符>` 不再单向截断
  (占位符不是明文口令, 截断会毁掉受控回程的还原锚点); 真口令照旧截断,
  两头都钉 (`password_scan_keeps_placeholder_family_and_still_redacts_real_passwords`)。
- **登记不实施**: `LeakedCredentialKind::JwtToken` 变体已声明但无对应扫描器
  (命名即承诺的空槽), 语义扩展超本批最小面, 债登记见 `00-backlog.md` 债栏。

## 8. 校验真账 (实测, 不复用旧值)

> 测试数字单一真源是 [test-baseline.md](../../../03-reference/test-baseline.md), 本页
> 只记「本批实测值 + 日期」供对账, 不代改真源。口径:
> `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` /
> `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` /
> `cargo fmt --all -- --check` / `scripts/check-neutral-terms.ps1`。

- **本批新增测试 24 项** (要求 ≥12): `redaction_bridge` 16 项 + `guardrail` 2 项
  + `input_security` 6 项; 2026-10-17 实测全绿。V5-a 相关合计 49 项绿
  (16 + guardrail 全 9 含既有 7 + input_security 全 24 含既有 18) ——
  与既有脱敏共存零回归由既有用例原样通过钉住。
- **governance crate 全套**: 191 passed / 0 failed (2026-10-17 实测);
  clippy `--all-targets -D warnings` 0 警告。
- **tools crate**: 18 个集成 target (除在途 `voice_speak`) 合计
  187 passed / 0 failed / 2 ignored (2026-10-17 实测, 含 `delete_path_parity`
  9/9); lib 面 374 passed / 1 failed —— 唯一红是 voice 线在途
  `self_status::tests::missing_data_sections_are_explicit_null_with_a_reason`
  (self_status.rs:1296), 与本批零关联; clippy 仅剩 `backup.rs` 1 条在途线
  finding, 本批文件 0 警告。
- **workspace 全量** (`--workspace --all-targets --locked --no-fail-fast -j 2`):
  2026-10-17 实测**未能出全量数** —— `apeireth-runtime-assembly` 编译红 5 条
  全在 voice/backup/G5 并发线的集成在途面 (voice_quiet / self_status_source /
  production 备份配置 / sqlite_task_queue), 非本批战场, 标红不代改;
  待在途线收口后由归口线复跑全量对账 (单一真源 test-baseline.md 不代改)。

## 9. 0 装诚实 (未接线面)

- **已接线〔四级口径：PRODUCTION WIRED；默认 OFF〕**: 机制面 (双向闸/回扫/账本) + tools 后置出站面
  (`ToolGuardrail::sweep_output_for_egress`)。
- **未接线**: 网关/补全链路的出境挂接、会话历史存储面接入、工具执行前的
  第二还原口 (执行口按本假说属另一回程, 本批只做受控回程/本地落盘前还原)。
  这些面分别属 orchestration/gateway 战场, 本批 0 触碰, 待归口线接。

## 10. 并发注记 (战场口径校正, 2026-10-17)

- `input_security.rs` 的真实位置是 `crates/foundation/governance/src/input_security.rs`
  (非 tools/src) —— 按「名字点名即战场」在真实路径升级; 依赖方向 (tools →
  governance) 顺势把会话指纹原语放在 governance 侧, 0 新外部依赖。
- 施工期 tools crate 同盘有 voice/backup 并发施工线 (`voice/**` / `backup.rs` /
  `self_status.rs`), 其在途红 (E0432/E0063/E0618 各一现) 按宪章 §六.3
  **标红不代改**, 本批改动与之零文件重叠 (lib.rs 各自 hunk 互不相扰)。
