#!/usr/bin/env python3
"""孤儿 API 基线的**共享定义**：头部说明、分组规则、格式常量。

两个工具共用这一份，避免「同一个概念两处定义、悄悄漂移」：

- `scripts/audit_unwired.py` —— 从代码事实更新**条目集合**（`--update` / `--check`）；
- `scripts/group_unwired_baseline.py` —— 按规则重排**分组**。

两者输出**逐字一致**的同一格式，因此可以互相验证：

```
python3 scripts/audit_unwired.py --update      # 更新集合（保留分组标记与人工注释）
python3 scripts/group_unwired_baseline.py       # 按规则重排
python3 scripts/group_unwired_baseline.py --check   # 校验两者一致
```

## 行内格式

```
orphan <symbol>  # <GROUP>  <可选理由>
```

**分组标记放行内**（不是靠行序）：这样「谁属于哪组」是**数据**而不是排版，
两个工具才能互相验证输出。
"""

from __future__ import annotations

import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parent.parent
BASELINE = ROOT / "scripts" / "unwired-baseline.txt"

# 扫描面 = 全部 crate 的 `src/` **和** `tests/`（集成测试）。
#
# 为什么必须扫 `tests/`：不扫的话，「集成测试专用的公开 API」会被判成 `orphan`
# （生产零引用 + 测试零引用），而它其实是 `test-only`。两类都表示生产零引用，
# 但**分类错了会误导还债方向**（把测试辅助当死代码删掉 ⇒ 集成测试编译失败）。
# `examples/` **必须扫**：它是 crate 的公开示例入口，CI 里常被
# `cargo run --example` 拉起（如 `scripts/guard_ml/train_shadow.py` 跑
# `stamp_guard_artifact`）。不扫的话，这些「被 example 调用的公开 API」
# 会被误报成孤儿 —— 2026-10-24 实测：`stamp_artifact_file` 因此假阳性。
SCAN_ROOTS = sorted(
    str(path.relative_to(ROOT))
    for pattern in ("crates/*/*/src", "crates/*/*/tests", "crates/*/*/examples")
    for path in ROOT.glob(pattern)
    if path.is_dir()
)

# 全部合法的分组标记
GROUPS = ("G1", "G3", "G4", "G5", "G6", "G7")

GROUP_DESC = {
    "G1": "集成测试辅助 API。保留合理，但**不应增长**。",
    "G3": "生物 / 认知 / 编排的**观测与查询 API**。生物族默认关 ⇒ 打开前不会走到。",
    "G4": "**注入点 / 探测口**（`with_*`）。生产装配走 `install_*`。",
    "G5": "**待判**：其余。逐条要人回答「真没人调，还是判据看不见」。**还债从这里开始。**",
    "G7": (
        "**owner 裁定不接**（有意保留，不是遗漏）："
        "W1 IM 通道（2026-10-23 裁定暂缓）、W5 哲学拦截层（2026-10-24 裁定不接）。"
        "保留代码是有意的 —— 解冻条件写在 `docs/status/unwired-ledger.md`。"
    ),
    "G6": (
        "**冗余 API 表面 / 宿主入口**：同一能力有**主路径在用**，这些是并行入口或便利包装；"
        "或设计上是给宿主/外部调用的公开 API。已逐条核查"
        "（见 `unwired-debt-taxonomy-2026-10-24.md` 与 `unwired-debt-audit-2026-10-24.md`）。"
    ),
}

HEADER = """# 孤儿 API 基线（scripts/audit_unwired.py --update 生成；**只减不增**）
#
# 每行: orphan <symbol>  # <分组>
#
# ============================================================================
# 覆盖面与语义（2026-10-24 第三轮修正）
# ============================================================================
# **覆盖全仓 30 个 crate**（`crates/*/*/src` + `crates/*/*/tests`）。
# 旧基线只扫 3 个目录 ⇒ `apeireth-memory` 等 crate 完全没进视野，
# 「教训对象三个写入 API 全仓零调用」因此一直没被发现。
#
# **只记 `orphan`（真零引用）**，不记 `test-only`（库 API 被集成测试覆盖 = 正常状态）。
#
# ============================================================================
# 判据可靠度
# ============================================================================
#   「生产零引用」        —— 高（已剔注释/字符串/`use` 导入，保留「作为值传递」形态）
#   「是否被 tests/ 覆盖」 —— 高（本版已扫 tests/）
# 已知盲区（都导致假阳性，方向安全）：宏展开、按名字字符串动态查找、
# 跨 crate 的 `pub use` 重导出链。
#
# **判据 v4（2026-10-24 第四轮）**：`use` 剔除从「逐行」改成「**整块**」——
# 多行 `pub use foo::{ A, B, C };` 以前只剔掉第一行，花括号里的每个类型名
# 都被漏成「被引用」⇒ **掩盖真孤儿**（实测：`JobBoard` 这类整块未接线的模块
# 因此看不出来）。整块剔除后**新增检出 15 条真孤儿**，条数上升是判据变准，
# 不是债变多。
#
# **另有一个会造成假阴性的盲区（2026-10-24 第四轮发现）**：
# 判据 v3 是**裸 token 计数**，所以**同名方法在不同类型上会互相遮蔽** ——
# `SnapshotStore::clear_session` 被调用，会让 `BehaviorChainGuardHook::clear_session`
# 也被判成「已接电」（其实它零调用）。
# ⇒ **新增同类方法时避开同名**（当时把 `SnapshotStore::clear_session` 改名成
# `remove_session`）。凡基线里有同名定义的条目，都值得人工再核一眼。
#
# ============================================================================
# 缩表纪律
# ============================================================================
# 1. **只减不增**：新增符号必须先进 main 并被生产调用，或在此写明理由；
# 2. **每还一条债就缩一行**：接线/删除后 `--update` 一次，让数字真实下降；
# 3. **不靠改判据缩表**（那是 cosmetic 抵扣）—— 判据改严必须说明「旧判据错在哪」；
# 4. **`--update` 保留人工资产**：头部说明与行内分组标记是人写的判断依据，工具不许冲掉；
# 5. **分组由 `scripts/group_unwired_baseline.py` 按规则生成**，规则集中维护、自动自校验。
#
# ============================================================================
# 已还的债（证明缩表在发生）
# ============================================================================
#   - `replaced_any`（egress_gate）→ 删（名字说「本次」实读全局计数，名不符实）；
#   - `loop_config_mut_for_test`（orchestrator）→ 删（零调用，连集成测试都不用）；
#   - 第四轮：13 条逐条确认的真死代码 → 删（清单见 `unwired-debt-audit-2026-10-24.md`）；
#   - 第四轮：`ToolDescAuditor::audit` / `audit_diff` 接入 MCP 目录 ⇒
#     **OWASP ASI-01 描述投毒防线从「文档宣称」变成「真在拦」**。
#
# ============================================================================
# 分组（每组条数写在组标题里）
# ============================================================================
"""

# ---------------------------------------------------------------------------
# 显式清单：已逐条核实过的归组判断。**每条都有人工核实的依据**（主路径的
# 文件:行号），不许凭印象加。加新条目前先读
# `docs/04-internal/backend-enhancement/unwired-debt-audit-2026-10-24.md`。
# ---------------------------------------------------------------------------

# G6：同一能力有主路径在用（零调用的是并行入口/便利包装/别名），
#     或设计上是供宿主/外部调用的公开 API。
VERIFIED_REDUNDANT_OR_HOST = {
    # --- engine/memory：主路径见 unwired-memory-audit-2026-10-24.md ---
    "consolidate_governed",  # 主路径 consolidate
    "default_min_count",  # 公开 API 的默认值常量，宿主面
    "ensure_governance",  # 主路径 ensure_governance_on_conn
    "fetch_governed",  # 主路径 fetch_governed_on_conn
    "get_for_subject",  # 承诺查询的其他入口
    "get_page",  # 主路径 compile_page
    "get_persona",  # persona 的 sweep/compress/consolidate
    "governance_cas_failure",  # 主路径 cas_failure_on_conn
    "is_episode_forgotten",  # 遗忘过滤在 coordinator 多处 get_governed
    "is_verbatim_zone",  # 逐字区由 PersonaMemory 字段分离保证
    "links_for_diary",  # 主路径 links_for_fact
    "recall_with_policy",  # 主路径 ProactiveRecallService::recall（coordinator.rs:854）
    "list_active",  # 承诺查询的其他入口
    "materialize_to_sink",  # 主路径 materialize_to_typed_sink
    "put_episode_full",  # 主路径 EpisodeStore::put_episode
    "query_as_of",  # 主路径 typed_recall.rs 的 store.query
    "query_recent",  # 主路径 recall_recent
    "query_with_time_range",  # 主路径 EpisodeQuery::in_range + query
    "retrieve_with_reranker",  # 主路径 retrieve_with_status
    "scalar",  # 主路径 AdaptiveBaseline::new
    "set_quiet_threshold",  # 宿主/daemon 配置入口（doc 自述）
    "streaks",  # observe 已返回 DriftAlarm.streak
    "upsert_hallway",  # 主路径 upsert_hallway_raw
    "without_as_of",  # 主路径 with_as_of(None)
    # --- foundation/orchestration ---
    "accumulator_mut",  # 主路径 add_signal / accumulator()
    "remove_job",  # JobBoard 模块整体仅集成测试使用
    "rule_count",  # 宿主注入型白名单的计数访问器
    "stream_len",  # 主路径 observe / recorded
    # --- engine/runtime ---
    "execute_streaming",  # 主路径 execute_outcome_streaming（已接网关 SSE）
    "register_provider",  # 主路径 RuntimeBuilder 批量注册
    "repetition_policy",  # 主路径 config().repetition_advisory
    # --- engine/guard ---
    "add_action",  # 主路径 add_action_with_id
    "extract_features",  # V1 特征在 V2 内计算
    "extract_features_v2",  # 主路径 from_chain / from_chain_with_cross_turn
    "recent_actions",  # 主路径 actions()
    # --- foundation/governance ---
    "claim_count",  # 主路径 record/verify/get（nightwatch 在用）
    "has_signal",  # 主路径 signals().first()
    "matches_word",  # 主路径 word_families
    # --- engine/storage（cache/rate_limit 是明确的库原语面）---
    "held_for",
    "put_default",
    "shard_count",
    "strategy_kind",
    # --- foundation/plugin（default-off 协议库面）---
    "minimax_default",
    "rebind",
    "total_subscriptions",
    # --- adapters/gateway ---
    "family_enforced",  # 主路径 enforce_remote_bind
    # --- adapters/sdk（显式桩 + 公开 API 面）---
    "apeireth_sdk_py",  # PyO3 模块入口，由 Python import 机制调用
    "is_compatible",  # 对外版本兼容性谓词
    "outbound_budget",  # 主路径 ImChannelKind::max_text_bytes
    "sdk_method_names",  # 主路径 SDK_TOOL_WHITELIST 常量
    "tool_paths",  # 主路径 TOOL_PATHS 常量
    # --- adapters/cli ---
    "active_profile",  # 主路径 active_profile_at
    "adapt_gate",  # 生产走 governance_chain::production()
    # --- engine/runtime-assembly ---
    "nervous_system_from_frame",  # 主路径 nervous_system_from_frame_with
    # --- capabilities/tools ---
    "allow_capability",  # 与 deny_capability 对称的宿主入口
    "beside_feeds",  # 主路径 FeedLifecycleStore::for_data_dir
    "explicit_allow_list",  # 宿主配置构造器（更严档位，不放宽默认）
    "is_satisfied_by",  # 主路径 missing_requirements
    "last_watch",  # 默认关守护族的诊断读口
    "library_root",  # 装配检查用访问器
    "not_wired_reason",  # 主路径 DATA_PROBE_NOT_WIRED 常量
    "reconnect_attempts",  # 重连退避主路径在用
    "session_state",  # 生命周期状态主路径在用
    # --- 2026-10-24 第四轮：判据 v4（use 整块剔除）后新揭出的 9 条 ---
    "constrain_to_trusted",  # 公开扩展点 IntentInterpreter 的宿主侧策略原语
    "decompose_default",  # 主路径 decompose（calibration_critic.rs:111 生产在用）
    "doc_ops_capability",  # 主路径 doc_ops_capability_shared（tool_modules.rs:955）
    "extract_experience_from_episode",  # 主路径 extract_experience（cognitive.rs:1738）
    "save_to_cluster",  # 一行包装（to_markdown + create_file）；落盘主路径在 dreaming.rs:300
    "serve_canonical",  # 3 参便利包装；生产入口是 serve_state_with_shutdown（cli/lib.rs:3463）
    "sheet_ops_capability",  # 主路径 sheet_ops_capability_shared（tool_modules.rs:1005）
    "trait_locks_path",  # 主路径 trait_locks_path_for（self_tuning_wire.rs:78）
    "validate_schedule",  # 一行别名；主路径 Schedule::validate（cron.rs:87）
}

# 按文件名归组（路径特征）
IM_FILES = ("im_bridge", "im_approval")
BIO_FILES = (
    "bio_",
    "care_heartbeat",
    "orchestrator",
    "cognitive",
    "causal_world_model",
    "barge_in",
    "organ/",
)

# 装配入口 / 生成器类命名 ⇒ 公开 API 表面
ASSEMBLY_PREFIXES = (
    "from_env",
    "from_env_value",
    "dispatch_",
    "generate_",
    "synthesize_",
    "build_",
    "install_",
    "produce_",
    "assemble_",
)


def find_file(symbol: str) -> str:
    """定位符号的定义所在文件（优先 src/ 下的定义）。"""
    for pattern in (f"fn {symbol}\\b", f"struct {symbol}\\b", f"trait {symbol}\\b"):
        out = subprocess.run(
            ["grep", "-rln", pattern, "crates/", "--include=*.rs"],
            capture_output=True,
            text=True,
            cwd=ROOT,
        ).stdout.splitlines()
        src = [line for line in out if "/src/" in line]
        if src:
            return src[0]
    return "?"


# owner 裁定不接的（W5 哲学拦截层：2026-10-24 owner「按你的建议来」⇒ 不接）。
# 这些代码**有意保留**，解冻条件写在 docs/status/unwired-ledger.md。
OWNER_DECLINED = {
    "verdict_keys_for_principle",  # W5 哲学拦截层
    "im_router",  # W1 IM 通道（owner 2026-10-23 裁定暂缓）
}


def classify(symbol: str, file_path: str) -> str:
    """按规则判定一个符号属于哪一组。"""
    if symbol in OWNER_DECLINED:
        return "G7"
    if symbol in VERIFIED_REDUNDANT_OR_HOST:
        return "G6"
    if symbol.endswith("_for_test") or "_for_test_" in symbol:
        return "G1"
    if any(key in file_path for key in IM_FILES):
        return "G7"
    if any(key in file_path for key in BIO_FILES):
        return "G3"
    if symbol.startswith("with_") or "probe" in symbol:
        return "G4"
    if symbol.startswith(ASSEMBLY_PREFIXES) or symbol.endswith("_from_env"):
        return "G6"
    return "G5"


def split_entry(line: str) -> tuple[str, str, str]:
    """拆一行基线条目 → (kind, symbol, group, reason)。

    行尾格式：`orphan <symbol>  # <GROUP>  <可选理由>`
    """
    stripped = line.strip()
    body, _, comment = stripped.partition("#")
    parts = body.split()
    if len(parts) < 2:
        return ("", stripped, "", "")
    kind, symbol = parts[0], parts[1]
    group = ""
    reason = ""
    comment = comment.strip()
    if comment:
        tokens = comment.split(None, 1)
        if tokens[0] in GROUPS:
            group = tokens[0]
            reason = tokens[1].strip() if len(tokens) > 1 else ""
        else:
            reason = comment
    return (kind, symbol, group, reason)


def render(entries: dict[str, str], groups: dict[str, list[str]]) -> str:
    """按统一格式渲染基线文件全文。"""
    out = [HEADER]
    for group in sorted(groups):
        items = groups[group]
        out.append(f"# --- {group}（{len(items)} 条）：{GROUP_DESC[group]}")
        out.append("")
        for symbol in items:
            reason = entries.get(symbol, "")
            out.append(
                f"orphan {symbol}  # {group}" + (f"  {reason}" if reason else "")
            )
        out.append("")
    return "\n".join(out).rstrip() + "\n"


# ---------------------------------------------------------------------------
# 判据 v3 的核心实现（**两个工具共用一份**）
# ---------------------------------------------------------------------------
#
# 判据 v3（2026-10-24 定稿）：
#   1. 剔**字符字面量** → 剔字符串 → 剔行/块注释 → 剔 `use` 导入行；
#   2. 对剩下的代码做**裸 token 计数**。
#
# 为什么裸 token 而不是「只认 `name(` / `name::` / `::name`」：
# 后者会漏掉「**函数作为值传递**」（`get(admin_config_get)`、`map(fn_name)`），
# 而路由注册、回调注册大量是这种形态 ⇒ 假阳性。
# 为什么裸 token 不会把「只提到名字」算成使用：
# 注释/字符串/`use` 行已在第 1 步剔掉。
#
# 已知盲区（**都导致假阳性**，方向安全）：
#   - 宏展开生成的调用；
#   - 反射 / 按名字字符串动态查找；
#   - **同名符号跨类型互相遮蔽**（造成假阴性，见基线头部说明）。

import re

IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
# `use` 导入行。**必须能吃多行声明** ——
# `pub use job_board::{ JobBoard, JobId, ... };` 这种写法若只剔第一行，
# 花括号里的每个类型名会漏成「被引用」⇒ 模块级判据把 re-export 当消费，
# 于是「整块未接线」的模块被掩盖（2026-10-24 实测：`JobBoard` 假阴性）。
USE_BLOCK = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?use\b[^;]*;", re.M | re.S)
LINE_COMMENT = re.compile(r"//[^\n]*")
BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.S)
# ⚠️ **不跨行**（不带 re.S）：Rust 普通字符串字面量不跨行。
# 带 re.S 遇到 `'"'` 这类**字符字面量**时会把其中那个 `"` 当成字符串起点，
# 一路吞到文件很后面的下一个 `"` ⇒ 把整段代码当字符串删掉（大片假阳性）。
CHAR_LIT = re.compile(r"'(?:\\.|[^'\\])'")
STRING_LIT = re.compile(r'"(?:\\.|[^"\\\n])*"')


def strip_noise(raw: str) -> str:
    """剔除字符字面量、字符串、行/块注释、`use` 导入行。"""
    # 顺序要紧：先剔**字符字面量**（防止 `'"'` 干扰字符串匹配），
    # 再剔字符串、行注释、块注释，最后剔 use 导入行。
    text = CHAR_LIT.sub("''", raw)
    text = STRING_LIT.sub('""', text)
    text = LINE_COMMENT.sub("", text)
    text = BLOCK_COMMENT.sub("", text)
    text = USE_BLOCK.sub("", text)
    return text


def usage_counts(texts) -> dict[str, int]:
    """裸 token 计数（判据 v3）。"""
    counts: dict[str, int] = {}
    for raw in texts:
        for word in IDENT.findall(strip_noise(raw)):
            counts[word] = counts.get(word, 0) + 1
    return counts
