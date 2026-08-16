# 自审报告: toolResultPrivacyGuard 增强 (env 行级 + 密钥模式脱敏)

- 任务 ID: ae12d9eb-fe0c-4267-8f23-b225880430d1
- 角色: 安全审查2 (security_reviewer2)
- 日期: 2026-08 (提交时)
- 范围: `crates/apeireth-guard` (唯一改动 crate; tool-approval/tool-runtime 未触碰)

## 1. 背景与定位

team-work-doc §8.3 toolResultPrivacyGuard 行: "guard 已借鉴 → 补 env 行级+sk- 模式"。

grep 定位结论:
- VCP toolResultPrivacyGuard 的**完整递归复刻**在 `apeireth-tool-runtime/src/privacy.rs` (含 env 行级 + 7 类高置信 token), 但该 crate 在任务边界中**禁改**。
- 真正被出站链路调用的文本级 guard 是 `apeireth-guard` (tool_bridge.rs / gateway guard_bridge.rs / daemon.rs 均调用 `detect_pii` + `redact_text`), 此前只有 6 类 PII regex, **缺 env 行级与密钥 token 模式** → 本任务增量落地点。

## 2. 改动文件 (4 个, 均在 apeireth-guard)

| 文件 | 改动 |
|---|---|
| `src/pii.rs` | + `PiiKind::SecretToken` / `PiiKind::EnvSecret` 两个变体 (ALL 6→8); + 3 组静态正则 (SECRET_TOKEN_RE 7 类前缀 / ENV_ASSIGN_RE 赋值行 / SENSITIVE_KEY_RE 敏感键名, 与 tool-runtime 同源); + `env_value_maskable` 误报控制 (长度≥8, 排除布尔/null/纯数字); detect_pii 增两个检测循环 (EnvSecret 只报 value 段, KEY= 前缀保留) |
| `src/redactor.rs` | redact_text 修复**重叠匹配错乱**: EnvSecret 值内嵌 SecretToken/Email 时跳过被包含匹配, 防止输出重复拼接 (新增回归测试) |
| `src/organ_kani_proofs.rs` | 不变量断言 6→8 类 (property 2/10 + kani proof 2), 测试名 six→eight |
| `src/lib.rs` | + 门面级集成测试 2 个 (env+token 脱敏含审计; 正常文本不误伤); 头部注释说明 |

## 3. 实装细节 (对照任务 4 个方向)

1. **env 赋值行级脱敏**: `(?m)^[ \t]*(?:export[ \t]+)?KEY[=:]['"]?VALUE['"]?[ \t\r]*(?:#.*)?$` + 敏感键名过滤 (16 类: api_key/secret/token/password/credential/private_key/... 词边界锚定) → 只报 value 段, 键名保留 (脱敏策略作用于值部)。
2. **高置信密钥 token**: sk- (≥24) / sk-proj- (≥24) / xoxb|xoxp|xoxa|xoxr- (≥24) / ghp_ (≥30) / github_pat_ (≥40) / glpat- (≥20) / AKIA+16 位, 全部词边界 + 最小长度阈值。
3. **保持已有规则**: 6 类原 PII 检测、4 策略脱敏、审计 ring buffer 全部不动 (增量补强不重写)。注: "首尾保留 4 字符"属 tool-runtime mask_secret 规则 (禁改未动); apeireth-guard Mask 策略保持自身既有规则 (首尾各 1 字符)。
4. **误报控制** (均有测试证据):
   - 短 token 不检 (`sk-abc` / `sk-ab 短 token 保留`)
   - 非敏感键不检 (HOME=/PATH=/monkey=/keyword=)
   - 值过短/纯数字/布尔不检 (API_KEY=abc / TOKEN_COUNT=42 / PASSWORD=123456789)
   - 词边界防子串误伤 (flask-mode / risk-taking 不误报)
   - 多行块只检敏感行 (LOG_LEVEL=debug 不检, DATABASE_PASSWORD=... 检)

## 4. 验收结果 (0 装 PASS)

- ✅ `cargo test -p apeireth-guard -j 4`: **59 passed, 0 failed** (新增 17 个用例: SecretToken 7 类 / EnvSecret 等号+冒号+export引号注释+多行 / 4 组误报控制 / 嵌套有序 / 重叠脱敏不损坏 / 门面级 2 个)
- ✅ 下游回归 `cargo test -p apeireth-gateway -j 4`: **84 + 7 passed, 0 failed** (guard_bridge 调用方)
- ✅ 0 新依赖 (Cargo.toml 无改动, 复用既有 regex/once_cell)
- ✅ 失败路径: 短值/数字值/非敏感键/短 token 均显式断言"不检出"
- ⚠️ `cargo test -p apeireth-companion` **未跑通** — 与本任务无关: 工作区存在他人未提交改动 (memory_graph/capability/reflection 等) 导致 companion lib 编译失败 (SqliteMemoryStore 缺 put_episode/recent_episodes + unstable map_or_default)。已登记 backlog N14, 待 Leader 指派。
- ✅ 文档同步: maintenance-guide 模块地图 +1 行; backlog N13 登记 (✅); team-work-doc §8.3 该行标 ✅。

## 5. 0 假装标注

- 做了什么: 文本级 guard 的 env 行级 + 密钥 token 检测/脱敏/测试/文档。
- 没做什么: ① JSON 结构化递归脱敏 (那是 tool-runtime 已有能力, 禁改); ② data:image base64 白名单 (apeireth-guard 不处理 JSON 值, 无需该白名单; tool-runtime 已有); ③ Kani 真跑 (kani proof 仅更新断言, 未执行 kani 工具链); ④ CRLF 全面验证 (正则已容忍行尾 \r, 未加专门用例)。

## 6. 新发现 (已记 backlog)

- **N14**: companion 编译失败源于工作区脏树 (他人未提交改动), 非本任务引入, 建议 Leader 核实后指派。
- redact_text 原有重叠匹配缺陷已顺手修复并加回归测试 (同 crate 内, 不越界)。

## 7. 给守门员的合并提示

- 改动集中在 apeireth-guard 4 文件, 无 API 破坏 (PiiKind 加变体, serde 新数据兼容; audit.rs 只存字段无穷举 match)。
- 下游 tool_bridge/gateway/daemon 无需改动 — 新检测类别经 detect_pii 自动生效, 出站文本护栏即刻增强。
- 工作区有大量他人未提交改动, 合并时请只挑 guard 4 文件 + 3 个 docs 文件 + 本报告。
