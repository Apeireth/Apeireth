# N10 宽松文本工具协议层 — 自审报告（backend_engineer2）

- **任务 ID**: 77b9efce-edba-4452-b7b3-92b7fa3debda
- **角色**: 后端工程师2（backend_engineer2）
- **日期**: 2026-08-17
- **任务包边界**: 只改 `apeireth-tool-runtime`；禁动 tool-approval / tool-registry / context-fold

## 一、交付内容

新增 `crates/apeireth-tool-runtime/src/text_protocol.rs`（~650 行，含 19 单测），对 VCP
`research/source/vcptoolbox/modules/vcpLoop/`（toolCallParser.js / toolMarkerFuzzyMatcher.js /
toolExecutor.js）做**字段级移植**。与既有严格 parser（parser.rs, ASCII
`<<<[TOOL_REQUEST]>>>` + `key:<<<value>>>`）**并存，零改动既有行为**（向后兼容）。

### 五大机制 → 实现映射

| # | 机制 | 实现 | VCP 对照 |
|---|---|---|---|
| ① | 始末语法解析 | 块 `<<<[TOOL_REQUEST]>>>`…`<<<[END_TOOL_REQUEST]>>>`；字段 `key:「始」v「末」` + ASCII `<<<v>>>` 兼容形态 | `MARKERS` + `_scanFields` |
| ② | ESCAPE 转义防注入 | `「始ESCAPE」…「末ESCAPE」` 字段可携带字面量结束标记（块结束扫描跳过 escape 区）；`<<<[*_ESCAPE]>>>` 字面量映射还原 | `_findBlockEnd` + `ESCAPED_LITERAL_MAP` + `_restoreEscapedLiterals` |
| ③ | 模糊标记匹配 | 块标记 `(?i)<{2,4}\s*\[\s*TOOL_REQUEST\s*\]\s*>{2,4}`（大小写/空白/尖括号数容错）；字段标记 4 括号变体 `「始」/{始}/{始」/「始}` | `toolMarkerFuzzyMatcher.js` 候选表 |
| ④ | archery 式解析执行分离 | `TextToolProtocol::separate` 分流 + `ToolExecutor::execute_separated` → normal 顺序 await / archery `tokio::spawn` fire-and-forget（`ArcheryHandle{tool_name, no_reply, join}`） | `toolExecutor.js` archery no-reply（结果不回灌） |
| ⑤ | 思考块剥离 | `strip_reasoning_blocks`: `<think>/<thinking>` 大小写/属性/嵌套容错；**未闭合开始标签丢弃其后全部内容**（防潜藏工具调用误执行）；孤立结束标签仅删自身 | `stripReasoningBlocks` |

公开 API：`TextToolProtocol::parse / separate / MARKER_START / MARKER_END`、
`parse_block`（人类直调复用口）、`strip_reasoning_blocks`、`restore_escaped_literals`、
`SeparatedCalls`、`ToolExecutor::execute_separated`、`ArcheryHandle`。全部自 lib.rs re-export。

### 变更文件（均属本任务包）

- `crates/apeireth-tool-runtime/src/text_protocol.rs`（新增）
- `crates/apeireth-tool-runtime/src/parser.rs`（`parse_field_value` 私有 → `pub(crate)`，行为不变）
- `crates/apeireth-tool-runtime/src/lib.rs`（注册模块 + re-export + `ArcheryHandle` 导出）
- `crates/apeireth-tool-runtime/src/executor.rs`（`execute_separated` + `ArcheryHandle` + 2 测）

## 二、验收结果

**`cargo test -p apeireth-tool-runtime -j 4` 全绿**（2026-08-17 实测）：

```
lib 单测:       test result: ok. 112 passed; 0 failed  （含 N10 新增 17 项 text_protocol + 2 项 executor）
tests/parser.rs: test result: ok. 20 passed; 0 failed
Doc-tests:      test result: ok. 2 passed; 0 failed
```

N10 新增 19 测试覆盖（正常/畸形/ESCAPE/模糊/思考剥离失败路径五类齐全）：
- 正常：canonical 块 + 「始」/「末」字段、多块顺序、`parse_block` 直调口
- 模糊：小写 + 标签内空白 + 4 尖括号块标记；`{始}`/`{始」`/`「始}` 字段变体；ASCII 兼容形态
- ESCAPE：值内字面量结束标记不终止块（防注入核心）；`*_ESCAPE` 字面量还原；未闭合 ESCAPE → 整块静默跳过
- 思考剥离：think/thinking + 大小写 + 属性 + 嵌套；**未闭合 think 内潜藏调用不执行**（安全失败路径）；孤立 `</think>` 仅删标签；闭合 think 内调用不执行
- 畸形：空输入/无标记 → 空 Vec；有始无终 → 空；缺 tool_name 块跳过且不影响后续块；valet→maid 镜像
- archery：separate 分流（true / no_reply 标志）；`execute_separated` 实测 archery 50ms 工具 <40ms 返回（fire-and-forget 不阻塞）+ 句柄可观测 join 成功 + no_reply 透传；全 normal 场景 handles 空

**0 新依赖**（regex/thiserror/serde/tokio 均既有）。`cargo build -p apeireth-tool-runtime --lib` 我的文件零警告。

## 三、过程中的真问题与处置（诚实记录）

1. **字节边界 panic（自查发现）**：首版 escape 检测用 `find` 未锚定，匹配到后续字段的
   ESCAPE 标记后误 `cursor += 1`，在多字节字符（「）内部切片 panic，2 测失败。
   修复：escape 必须 `start()==0` 锚定，否则 fall-through 到「始」分支；全绿。
2. **外部阻塞（非本任务引入）**：验收中途 `apeireth-tool-approval/src/rule.rs`
   （他人任务包，禁动）出现未提交 WIP 编译错误 E0521，连累本 crate dev-dep 构建。
   处置：① 不越界修改；② 临时注释 dev-dep 验证本 crate 112 测全绿后**原样还原**
   Cargo.toml/Cargo.lock；③ 其后该包被属主/整合流程修复（`best_match` 已正常），
   完整 `cargo test -p apeireth-tool-runtime -j 4` 复跑全绿。
3. **流水线收编**：我暂存的 4 个代码文件被整合流水线提交 `fad23d81`（commit message 为
   N5 docs）一并收编入库。代码内容经 `git show fad23d81 --stat` 核实完整
   （text_protocol.rs 649 行 + executor.rs 114 行 + lib.rs 8 行 + parser.rs 4 行）。
   本报告与 backlog 回填为该提交的补记录。
4. **顺手还原非我改动**：`cargo fmt -p apeireth-tool-runtime` 顺带格式化了
   `examples/r131_llm_tool_call.rs`（纯空白），为保 diff 纯净已 `git checkout` 还原。

## 四、不假装（0 装 PASS 声明）

- ❌ 未做：VCP vref 语义检索注入（属 toolExecutor 编排，超出协议层范畴）；运行期
  fuzzy 开关（VCP 有 configure(enabled)，本层模糊常开 + 严格路径走既有 parser）
- ❌ 未接 tool-bridge 注册口（任务标注"必要时"——本轮未必要：`TextToolProtocol::parse`
  输出 `Vec<ParsedToolCall>` 与既有 `ToolExecutor/ToolCallPipeline` 类型完全一致，
  companion 侧一行切换即可接入；接线建议记 backlog 由 Leader 派）
- ✅ 所有机制均有真实现 + 真跑测试，失败路径（畸形/未闭合/潜藏注入）全覆盖

## 五、文档同步

- `docs/maintenance-guide.md` 模块地图：+1 行（text_protocol.rs 条目，merge 后幸存）
- `docs/backlog.md`：N10 ⬜→✅（本行，注明提交归属 fad23d81）

## 六、建议（新发现，不顺手做，交 Leader 派活）

1. **companion 侧接线**：多轮循环/serve 的 LLM 输出解析点从严格 parser 切换/降级到
   `TextToolProtocol::parse`（宽松层永不 Err，畸形静默跳过，语义安全）。
2. **archery 回灌策略**：`execute_separated` 的 normal 结果回灌已就位；archery
   no_reply 的审计留痕可接 `RecordStore`（当前句柄仅观测，未自动记录）。
