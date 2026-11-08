#!/usr/bin/env python3
"""**能力大汇总表 · 生成器 + 守卫**（Apeireth v2）

# 用途

给主人一张**可再生**的「Apeireth 现在拥有哪些能力、每条什么状态」全景表。
表本体在 `docs/03-reference/capability-status-summary.md`，由本脚本**从机器真源重算**生成 ⇒
**手改即漂移 ⇒ `--check` 非 0 退出**。

# 真源（全部机器读，脚本内**不写死任何一条能力结论**）

| 真源 | 提供什么 |
|---|---|
| `crates/engine/runtime-assembly/src/canonical/tool_taxonomy.rs` | **能力 id 策展表**（生产注册面的权威清单）+ 库级保留面 + 动态族根 |
| `crates/` 全树扫描（复刻 `production_tool_capability_ids` 口径） | 哪些 `tool.*` id **真实注册在生产面**（剔除 `tests/` 与 `#[cfg(test)]` 门控段） |
| `crates/engine/runtime-assembly/src/canonical/production.rs` | **生产装配根**：哪些能力面真的被装配（注册即闸）+ 装配点行号 |
| `crates/engine/runtime-assembly/src/canonical/production_config.rs` | 装配配置的**默认档**（`Default` 实现逐字段） |
| `crates/engine/runtime-assembly/src/canonical/self_status_roster.rs` | **能力名册**：开关全表（名 → 生效值表达式） |
| `crates/adapters/cli/src/lib.rs` | **组合根**：env 开关名 ↔ 配置字段 ↔ 默认档 |
| `docs/architecture/env-registry.toml` | 每个 `APEIRETH_*` 开关的登记（default / level / evidence） |
| `docs/status/claims.yaml` | 对外承诺台账（level / default / switch_env / verified_at） |

# 生成口径

**四级阶梯**：`NOT IMPLEMENTED` < `IMPLEMENTED` < `PRODUCTION WIRED` < `HARDWARE VALIDATED`
（承 `docs/01-architecture/system-capabilities.md`）。`DEFAULT ENABLED` **不是一级**，是横切属性
⇒ 单独一列。**注册 ≠ 可调用**：授权/审批是另一层。

**表的行集合**由代码算出，不是手抄：

1. **生产装配根可达的静态能力 id** = 策展表键 ∩ 生产注册面 ∩ **装配根真的注册过**
   ∪ **1 个动态族** `tool.mcp.<server>.<tool>`（族根取自策展表 `DYNAMIC_FAMILY_ROOTS`，
   经常量回查得字面量）。
2. **有 id 但生产不可达** = 库级保留面（策展表 `LIBRARY_LEVEL_FACES`，仓内显式登记但**未注册
   为工具**）∪ 仓内有 `ToolCapability` 实现但**装配根无构造点**的 id ∪ **运行期拼出的 id**
   （`format!("tool.memory.{op}")` / `format!("tool.mesh.{suffix}")` —— 源码里不是字面量，
   静态扫描看不到，按 `OPERATIONS` 常量 / `Kind::suffix()` 枚举派生）。
3. **库级模块面**（有实现、无能力 id）= `crates/capabilities/tools/src/` 里**不产出任何能力 id**
   且**不托管**任何能力的模块。
4. **认知 / 行为模块族**（不是 `tool.*` 能力 id）= `self_status_roster.rs` 名册登记的
   **行为面**开关（排除工具面开关与注入槽参数）。

# 每次生成都会回核（**致命** ⇒ `--check` 非 0 退出）

① **开关登记**：每个开关名必须在 env 登记表里（或明示「无 env 开关」），默认档与登记表一致。
② **台账等级**：claims 的 level 必须在四级口径词表内。
③ **证据存活**：每个 `file:line` 必须仍指向活文件且行号在范围内 ⇒ 既有面被搬走 ⇒ 红。

# 每次生成都会**报出但不判红**（上游缺口）

④ **两向完整性**：策展表每个键都要在生产注册面里；生产注册面每个 id 都要在策展表里 ——
   与 Rust 侧 `tool_taxonomy.rs` 的完整性闸门**同口径**。这是**策展表自己**的缺口
   （表已如实反映现状）⇒ 守卫在 Rust 侧 `cargo test -p apeireth-runtime-assembly --lib
   tool_taxonomy`，本脚本只如实报出。

# 确定性

输出**跨进程稳定**（同一棵树 ⇒ 同一张表）：所有集合迭代点都排序过
（`set` 迭代序随 `PYTHONHASHSEED` 变 ⇒ 未排序的集合会让 `--gen`/`--check` 互相对不上）。

# 用法

    python3 scripts/gen_capability_summary.py --gen      # 重算并重写生成块
    python3 scripts/gen_capability_summary.py --check     # 重算并比对（不一致 ⇒ 非 0 退出）
    python3 scripts/gen_capability_summary.py --report    # 只打印生成的表体 + 统计

# 退出码

    0 = 表与真源一致
    1 = 不一致（逐条打印差在哪 + 怎么修）
"""

from __future__ import annotations

import argparse
import difflib
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PAGE = ROOT / "docs" / "03-reference" / "capability-status-summary.md"

TAXONOMY = ROOT / "crates/engine/runtime-assembly/src/canonical/tool_taxonomy.rs"
PRODUCTION = ROOT / "crates/engine/runtime-assembly/src/canonical/production.rs"
PROD_CONFIG = ROOT / "crates/engine/runtime-assembly/src/canonical/production_config.rs"
ROSTER = ROOT / "crates/engine/runtime-assembly/src/canonical/self_status_roster.rs"
CLI_ROOT = ROOT / "crates/adapters/cli/src/lib.rs"
ENVREG = ROOT / "docs/architecture/env-registry.toml"
CLAIMS = ROOT / "docs/status/claims.yaml"
TOOLS_LIB = ROOT / "crates/capabilities/tools/src/lib.rs"

BEGIN = "<!-- BEGIN:generated -->"
END = "<!-- END:generated -->"

#: 四级口径词表（承 system-capabilities.md）
CALIBERS = ("NOT IMPLEMENTED", "IMPLEMENTED", "PRODUCTION WIRED", "HARDWARE VALIDATED")

#: 库级保留面在表里的族名（策展表 `LIBRARY_LEVEL_FACES` 逐条派生，见 taxonomy_keys）
LIBRARY_FACE_LABEL = {
    "tool.process": "Process（库级保留面）",
    "tool.std_sub_supervisor": "Supervisor（库级保留面）",
    "tool.supervisor": "Supervisor（库级保留面）",
}


# ---------------------------------------------------------------- 词法工具


#: 开关口径**人工覆盖**（自动推导对不齐的少数族；每条仍受下方「env 必须已登记 +
#: 默认档与登记表一致」两道回核约束 ⇒ 覆盖表本身也漂移必红）。
SWITCH_OVERRIDES = {
    # 进程树面**复用** shell 开关（注册即闸 ⇒ shell 关 = 无进程可看 ⇒ 0 新 env 名）。
    "tool.process_tree": ("APEIRETH_ENABLE_SHELL", "OFF"),
}

def read_bytes(p: Path) -> bytes:
    try:
        return p.read_bytes()
    except OSError:
        return b""


def read(p: Path) -> str:
    try:
        return p.read_text(encoding="utf-8")
    except OSError:
        return ""


def skip_ws_and_comments(b: bytes, i: int) -> int:
    """复刻 Rust 侧 `skip_ws_and_comments`。"""
    n = len(b)
    while i < n:
        c = b[i]
        if c in (32, 9, 13, 10):
            i += 1
        elif c == 47 and i + 1 < n and b[i + 1] == 47:  # //
            while i < n and b[i] != 10:
                i += 1
        elif c == 47 and i + 1 < n and b[i + 1] == 42:  # /*
            i += 2
            while i + 1 < n and not (b[i] == 42 and b[i + 1] == 47):
                i += 1
            i += 2
        else:
            break
    return i


def skip_string_literal(b: bytes, i: int) -> int:
    j = i + 1
    while j < len(b):
        if b[j] == 92:
            j += 2
            continue
        if b[j] == 34:
            return j + 1
        j += 1
    return j


def char_literal_end(b: bytes, i: int):
    nxt = b[i + 1] if i + 1 < len(b) else None
    if nxt is None:
        return None
    if nxt == 92:
        c = i + 2
        while c < len(b) and b[c] != 39:
            c += 1
        return c + 1 if c < len(b) else None
    if i + 2 < len(b) and b[i + 2] == 39:
        return i + 3
    return None


def matching_paren(b: bytes, open_: int):
    """返回 `)` 之后的位置（与 Rust `matching_paren` 同）。"""
    depth = 0
    i = open_
    while i < len(b):
        c = b[i]
        if c == 34:
            i = skip_string_literal(b, i)
            continue
        if c == 40:
            depth += 1
        elif c == 41:
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return None


def matching_brace(b: bytes, open_: int):
    """返回配对 `}` 的下标（与 Rust `matching_brace` 同）。"""
    depth = 0
    i = open_
    while i < len(b):
        c = b[i]
        if c == 47 and i + 1 < len(b) and b[i + 1] in (47, 42):
            i = skip_ws_and_comments(b, i)
            continue
        if c == 34:
            i = skip_string_literal(b, i)
            continue
        if c == 39:
            e = char_literal_end(b, i)
            i = e if e is not None else i + 1
            continue
        if c == 123:
            depth += 1
        elif c == 125:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return None


def is_test_gated(expr: bytes) -> bool:
    """复刻 Rust 侧 `is_test_gated`：`not(test)` 不算门控，其余含 `test` 标识符即算。"""
    compact = bytes(c for c in expr if c not in b" \t\r\n")
    if b"not(test)" in compact:
        return False
    return any(m.group(0) == b"test" for m in re.finditer(rb"[A-Za-z_][A-Za-z0-9_]*", compact))


def gated_item_extent(b: bytes, frm: int):
    """复刻 Rust 侧 `gated_item_extent`：返回 (kind, at)，kind 为 `{` 或 `;`。"""
    depth = 0
    i = frm
    while i < len(b):
        c = b[i]
        if c == 47 and i + 1 < len(b) and b[i + 1] in (47, 42):
            i = skip_ws_and_comments(b, i)
            continue
        if c == 34:
            i = skip_string_literal(b, i)
            continue
        if c == 39:
            e = char_literal_end(b, i)
            i = e if e is not None else i + 1
            continue
        if c in (40, 91):
            depth += 1
        elif c in (41, 93):
            if depth > 0:
                depth -= 1
        elif depth == 0 and c in (123, 59):
            return (c, i)
        i += 1
    return None


def strip_test_gated(raw: bytes) -> bytes:
    """复刻 Rust 侧 `strip_test_gated`：把 `#[cfg(test)]` 门控的整段代码抹白。"""
    out = bytearray(raw)
    cursor = 0
    n = len(raw)
    while True:
        m = raw.find(b"#[cfg(", cursor)
        if m < 0:
            break
        paren = raw.find(b"(", m + 5)
        if paren < 0:
            cursor = m + 1
            continue
        after = matching_paren(raw, paren)
        if after is None:
            cursor = m + 1
            continue
        if not is_test_gated(raw[paren + 1:after - 1]):
            cursor = m + 1
            continue
        item = skip_ws_and_comments(raw, after)
        if item < n and raw[item] == 93:
            item = skip_ws_and_comments(raw, item + 1)
        while raw[item:item + 2] == b"#[":
            p2 = raw.find(b"(", item)
            if p2 < 0:
                break
            e2 = matching_paren(raw, p2)
            if e2 is None:
                break
            item = skip_ws_and_comments(raw, e2)
            if item < n and raw[item] == 93:
                item = skip_ws_and_comments(raw, item + 1)
        ext = gated_item_extent(raw, item)
        if ext is None:
            cursor = m + 1
            continue
        kind, at = ext
        if kind == 123:
            close = matching_brace(raw, at)
            if close is None:
                cursor = m + 1
                continue
            end = close
        else:
            end = at
        for x in range(m, min(end + 1, n)):
            if out[x] != 10:
                out[x] = 32
        cursor = end + 1
    return bytes(out)


# ---------------------------------------------------------------- 真源读取


def collect_production_sources() -> list[Path]:
    """复刻 Rust 侧 `collect_production_sources`：跳 `tests/benches/examples/fuzz/target` + `tests.rs`。"""
    out = []
    for p in sorted((ROOT / "crates").rglob("*.rs")):
        if any(part in ("tests", "benches", "examples", "fuzz", "target") for part in p.parts):
            continue
        if p.name in ("tests.rs", "test.rs"):
            continue
        out.append(p)
    return out


def runtime_format_ids() -> dict[str, list[tuple[str, int]]]:
    """**运行期生成**的能力 id（`CapabilityId::new(format!("tool.<族>.{…}"))`）。

    这类 id 在源码里不是字面量 ⇒ 上面的字面量扫描看不到，但它们**确实存在**。
    从两处真源派生成员清单：

    * `tool.memory.*` —— `memory_module.rs` 的 `OPERATIONS` 常量逐项；
    * `tool.mesh.*` —— `mesh.rs` 的 `Kind::suffix()` 逐个变体。

    返回 id ⇒ [(相对路径, 行号)]，行号取**族根的 format! 调用处**。
    """
    out: dict[str, list[tuple[str, int]]] = {}

    # ---- tool.memory.* ----
    mem = ROOT / "crates/engine/runtime-assembly/src/canonical/memory_module.rs"
    if mem.is_file():
        src = read(mem)
        om = re.search(r"const OPERATIONS:\s*\[&str;\s*\d+\]\s*=\s*\[(.*?)\];", src, re.S)
        cap = re.search(r'CapabilityId::new\(format!\("tool\.memory\.\{', src)
        if om and cap:
            line = src[:cap.start()].count("\n") + 1
            rel = str(mem.relative_to(ROOT))
            for op in re.findall(r'"([^"]+)"', om.group(1)):
                out.setdefault(f"tool.memory.{op}", []).append((rel, line))

    # ---- tool.mesh.* ----
    mesh = ROOT / "crates/engine/runtime-assembly/src/mesh.rs"
    if mesh.is_file():
        src = read(mesh)
        cap = re.search(r'CapabilityId::new\(format!\("tool\.mesh\.\{', src)
        if cap:
            line = src[:cap.start()].count("\n") + 1
            rel = str(mesh.relative_to(ROOT))
            # `fn suffix(self) -> &'static str { match self { Self::Connect => "connect", … } }`
            sm = re.search(r"fn suffix\([^)]*\)\s*->\s*&'?static str\s*\{(.*?)\n\s*\}", src, re.S)
            suffixes: list[str] = []
            if sm:
                suffixes = re.findall(r'=>\s*"([^"]+)"', sm.group(1))
            if not suffixes:
                # 兜底：`Self::X => "x"` / `Kind::X => "x"` 形态
                suffixes = re.findall(r'(?:Self|Kind)::\w+\s*=>\s*"([^"]+)"', src)
            for sfx in dict.fromkeys(suffixes):
                out.setdefault(f"tool.mesh.{sfx}", []).append((rel, line))

    return out


def extract_capability_ids(text: bytes) -> list[tuple[str, int]]:
    """复刻 Rust 侧 `extract_tool_capability_ids`：两种注册写法，返回 (id, 字节偏移)。"""
    found: list[tuple[str, int]] = []
    cursor = 0
    while True:
        m = text.find(b"CapabilityId::new(", cursor)
        if m < 0:
            break
        at = skip_ws_and_comments(text, m + len(b"CapabilityId::new("))
        if at < len(text) and text[at] == 34:
            end = skip_string_literal(text, at)
            lit = text[at + 1:end - 1].decode("utf-8", "replace")
            if lit.startswith("tool."):
                found.append((lit, at))
        cursor = m + 1
    cursor = 0
    while True:
        m = text.find(b"CAPABILITY_ID", cursor)
        if m < 0:
            break
        at = skip_ws_and_comments(text, m + len(b"CAPABILITY_ID"))
        if at >= len(text) or text[at] != 58:
            cursor = m + 1
            continue
        at = skip_ws_and_comments(text, at + 1)
        if text[at:at + 4] != b"&str":
            cursor = m + 1
            continue
        at = skip_ws_and_comments(text, at + 4)
        if at >= len(text) or text[at] != 61:
            cursor = m + 1
            continue
        lit_at = skip_ws_and_comments(text, at + 1)
        if lit_at < len(text) and text[lit_at] == 34:
            end = skip_string_literal(text, lit_at)
            lit = text[lit_at + 1:end - 1].decode("utf-8", "replace")
            if lit.startswith("tool."):
                found.append((lit, lit_at))
        cursor = m + 1
    return found


def scan_registered_ids() -> dict[str, list[tuple[str, int]]]:
    """扫 `crates/` ⇒ 真实注册在生产面的 `tool.*` id ⇒ [(相对路径, 行号)]。"""
    out: dict[str, list[tuple[str, int]]] = {}
    for p in collect_production_sources():
        prod = strip_test_gated(read_bytes(p))
        for lit, at in extract_capability_ids(prod):
            line = prod[:at].count(b"\n") + 1
            out.setdefault(lit, []).append((str(p.relative_to(ROOT)), line))
    return out


def taxonomy_entries() -> list[dict]:
    """策展表条目：key / tags / source（逐条从 `TOOL_TAXONOMY` 数组解析）。"""
    src = read(TAXONOMY)
    m = re.search(r"pub const TOOL_TAXONOMY[^=]*=\s*&\[", src)
    if not m:
        return []
    blk = src[m.end():src.index("\n];", m.end())]
    entries = []
    for em in re.finditer(
        r"key:\s*\"([^\"]+)\",\s*tags:\s*&\[([^\]]*)\],\s*source:\s*ToolSource::(\w+)", blk
    ):
        tags = re.findall(r"\"([^\"]+)\"", em.group(2))
        entries.append({"key": em.group(1), "tags": tags, "source": em.group(3)})
    return entries


def taxonomy_faces() -> tuple[set[str], set[str]]:
    """(库级保留面, 动态族根)。

    动态族根在策展表里写作**常量引用**（`DYNAMIC_FAMILY_ROOTS = &[MCP_FAMILY_ROOT]`），
    其字面量的单一真源在工具侧 ⇒ 这里顺着常量名回查它的 `&str` 字面量。
    """
    src = read(TAXONOMY)
    lib: set[str] = set()
    m = re.search(r"LIBRARY_LEVEL_FACES[^=]*=\s*&\[(.*?)\];", src, re.S)
    if m:
        lib = set(re.findall(r'"([^"]+)"', m.group(1)))
    dyn: set[str] = set()
    m = re.search(r"DYNAMIC_FAMILY_ROOTS[^=]*=\s*&\[(.*?)\];", src, re.S)
    if m:
        for tok in re.findall(r"(\w+)", m.group(1)):
            lit = const_string_value(tok, search_dirs=("crates/capabilities/tools/src",))
            if lit:
                dyn.add(lit)
    return lib, dyn


def const_string_value(name: str, search_dirs: tuple[str, ...] = ()) -> str:
    """按常量名回查它的 `&str` 字面量（跨文件、限定搜索目录）。"""
    roots = [ROOT / d for d in search_dirs] if search_dirs else [ROOT / "crates"]
    pat = re.compile(
        r"(?:pub(?:\(crate\))?\s+)?const\s+" + re.escape(name) + r"\s*:\s*&str\s*=\s*\"([^\"]+)\""
    )
    for base in roots:
        if not base.exists():
            continue
        for p in sorted(base.rglob("*.rs")):
            m = pat.search(read(p))
            if m:
                return m.group(1)
    return ""


def taxonomy_tags() -> dict[str, list[str]]:
    return {e["key"]: e["tags"] for e in taxonomy_entries()}


def env_registry() -> dict:
    """env 登记表 ⇒ name → entry。"""
    try:
        import tomllib
    except Exception:
        return {}
    try:
        data = tomllib.loads(read(ENVREG))
    except Exception:
        return {}
    return {e.get("name", ""): e for e in data.get("entries", []) if isinstance(e, dict)}


def claims() -> dict:
    """claims.yaml ⇒ id → claim（轻量解析，只取顶层 claims 列表的若干字段）。"""
    try:
        import yaml
        data = yaml.safe_load(read(CLAIMS)) or {}
    except Exception:
        return {}
    out = {}
    for c in data.get("claims", []) or []:
        if isinstance(c, dict) and c.get("id"):
            out[c["id"]] = c
    return out


# ---------------------------------------------------------------- 装配面派生


def provider_modules() -> dict[str, str]:
    """`impl CapabilityProvider for X` ⇒ X → 模块 id（如 `module.tool.filesystem`）。"""
    out: dict[str, str] = {}
    for mod, (_p, mid, _body) in provider_impls().items():
        out[mod] = mid
    return out


def provider_impls() -> dict[str, tuple[Path, str, str]]:
    """`impl CapabilityProvider for X` ⇒ X → (文件, 模块 id, 模块实现体)。

    模块实现体 = 同名 `impl X { … }` 块（构造器所在），用于反查它构造了哪些工具。
    """
    out: dict[str, tuple[Path, str, str]] = {}
    for p in collect_production_sources():
        src = read(p)
        for m in re.finditer(r"impl CapabilityProvider for (\w+)", src):
            name = m.group(1)
            head = src[m.end():m.end() + 400]
            mid = re.search(r'"(module\.[^"]+)"', head)
            if not mid:
                continue
            body = ""
            ci = re.search(r"impl\s+" + re.escape(name) + r"\s*\{", src)
            if ci:
                start = src.index("{", ci.end())
                depth = 0
                j = start
                while j < len(src):
                    if src[j] == "{":
                        depth += 1
                    elif src[j] == "}":
                        depth -= 1
                        if depth == 0:
                            break
                    j += 1
                body = src[start:j]
            out[name] = (p, mid.group(1), body)
    return out


def assembly_sites() -> list[dict]:
    """生产装配根里 `collect_tool_module(...)` 的装配点（模块名 → 行号）。

    provider 构造写法有三种，都要认：
    `let provider = X::new(…)` / `let provider =\n    X::new(…)`（换行）/
    `let provider = super::tool_modules::X::new(…)` ⇒ 取路径**最后一段**的模块名。
    """
    src = read(PRODUCTION)
    lines = src.splitlines()
    sites = []
    for i, ln in enumerate(lines, 1):
        if "collect_tool_module(" not in ln:
            continue
        provider = None
        # 向上找最近的 `let provider =` 绑定（允许构造调用在下一行）
        for j in range(i - 1, max(0, i - 40), -1):
            pm = re.search(r"let provider\s*=\s*([\w:]+)\s*::", lines[j - 1])
            if pm:
                provider = pm.group(1).split("::")[-1]
                break
            if re.search(r"let provider\s*=\s*$", lines[j - 1]):
                pm2 = re.search(r"([\w:]+)\s*::", lines[j]) if j < len(lines) else None
                if pm2:
                    provider = pm2.group(1).split("::")[-1]
                break
        sites.append({"line": i, "provider": provider})
    return sites


def wired_capability_ids(registered: set[str]) -> tuple[dict[str, tuple[str, int]], list[str]]:
    """生产装配根**真正注册**的能力 id ⇒ id → (装配点文件, 行号)，外加装配点说明。

    生产注册面（能力 id 在仓内被声明）≠ 生产装配根可达（能力面在装配根真的注册）。
    装配根有三条注册路径，都要认：

    ① `collect_tool_module(…, &provider)` —— 绝大多数工具面；
    ② `capabilities.extend(module.capabilities())` —— 生物族（play / ritual）经
       `bio_production_wiring::install_bio_*` 注册；
    ③ `mcp_tool_bag` —— MCP 协议桥（登记账袋，运行期发现的工具登记进这只袋）。

    ②③ 的模块由 `impl CapabilityProvider for X` 的**模块 id** 族前缀定位：
    `module.tool.play_*` ⇒ `tool.play_*`，`module.mcp` ⇒ `tool.mcp.*`。
    """
    prov2ids = provider_to_ids()
    sites = assembly_sites()
    site_by_provider = {s["provider"]: s for s in sites if s["provider"]}

    wired: dict[str, tuple[str, int]] = {}
    # ① collect_tool_module 路径
    for mod, ids in prov2ids.items():
        site = site_by_provider.get(mod)
        if not site:
            continue
        for cid in ids:
            wired.setdefault(cid, ("production.rs", site["line"]))

    # ② 生物族（play / ritual）：装配根经 `install_bio_play` / `install_bio_ritual_tools`
    #    把模块能力 extend 进注册面 ⇒ 该族模块算装配根可达。
    #    每个 id 归属**自己的**装配点：play 族 ⇒ `install_bio_play` 调用行；
    #    ritual 族 ⇒ `install_bio_ritual_tools` 调用行。
    #    （`sorted` 保证跨进程稳定 —— `set` 迭代序随 PYTHONHASHSEED 变。）
    prod_src = read(PRODUCTION)
    bio_calls = sorted(set(re.findall(r"bio_production_wiring::(install_bio_\w+)\(", prod_src)))
    for fn in bio_calls:
        site_line = prod_src.find(fn)
        if site_line < 0:
            continue
        line = prod_src[:site_line].count("\n") + 1
        for mod, ids in sorted(prov2ids.items()):
            stem = "Play" if fn.endswith("install_bio_play") else "Ritual"
            if mod.startswith(stem):
                for cid in ids:
                    wired.setdefault(cid, ("production.rs", line))

    # ③ MCP 协议桥（登记账袋）
    if "McpModule" in prov2ids:
        src = read(PRODUCTION)
        idx = src.find("McpModule::new()")
        line = src[:idx].count("\n") + 1 if idx >= 0 else 0
        for cid in prov2ids["McpModule"]:
            wired.setdefault(cid, ("production.rs", line))

    # ④ 外部适配器：装配根用 `ProcessModule::new(external_config)` 注册 10 件外部工具
    ext_idx = read(PRODUCTION).find("ProcessModule::new(")
    if ext_idx >= 0:
        line = read(PRODUCTION)[:ext_idx].count("\n") + 1
        for cid in registered:
            if cid.startswith("tool.external."):
                wired.setdefault(cid, ("production.rs", line))

    # ⑤ 自述面（`tool.self_status`）：装配根 `SelfStatusModule::new_with_bus` 注册
    if "SelfStatusModule" in prov2ids:
        src = read(PRODUCTION)
        idx = src.find("SelfStatusModule::new_with_bus(")
        line = src[:idx].count("\n") + 1 if idx >= 0 else 0
        for cid in prov2ids["SelfStatusModule"]:
            wired.setdefault(cid, ("production.rs", line))

    # ⑥ 直接 `capabilities.extend(provider.capabilities())` 形态（如 `tool.repo_map`）
    src = read(PRODUCTION)
    lines = src.splitlines()
    for i, ln in enumerate(lines, 1):
        if "capabilities.extend(" not in ln:
            continue
        if ".capabilities())" not in ln:
            continue
        pm = re.search(r"capabilities\.extend\(\s*([\w:]+)\.capabilities\(\)\)", ln)
        mod = None
        if pm:
            target = pm.group(1).split("::")[-1]
            if target in prov2ids:
                mod = target
            else:
                # 目标是局部变量 ⇒ 向上找它的 `let provider = X::…` 绑定
                for j in range(i - 1, max(0, i - 12), -1):
                    bm = re.search(
                        r"let\s+" + re.escape(target) + r"\s*=\s*([\w:]+)\s*::", lines[j - 1]
                    )
                    if bm:
                        mod = bm.group(1).split("::")[-1]
                        break
        if not mod:
            continue
        for cid in prov2ids.get(mod, []):
            wired.setdefault(cid, ("production.rs", i))

    return wired, [s["line"] for s in sites]


def provider_to_ids() -> dict[str, list[str]]:
    """模块 ⇒ 它产出的能力 id。

    派生口径：模块的实现体里构造了哪些**工具结构体**（`XxxTool`）⇒ 该结构体所在
    文件里声明的 `tool.*` id 就是它产出的能力。同名族前缀（`module.tool.recon` ⇒
    `tool.recon.*`）作为兜底，覆盖「一个模块产出一族」的情形。
    """
    registered = set(scan_registered_ids())
    # 结构体名 ⇒ 所在文件（工具结构体多在 tools crate）
    struct_file: dict[str, Path] = {}
    for p in collect_production_sources():
        src = read(p)
        for m in re.finditer(r"^(?:pub(?:\(crate\))?\s+)?struct\s+(\w+)", src, re.M):
            struct_file.setdefault(m.group(1), p)

    out: dict[str, list[str]] = {}
    for mod, (p, mid, body) in sorted(provider_impls().items()):
        ids: set[str] = set()
        # 该模块实现体里构造的工具结构体
        for t in sorted(set(re.findall(r"\b(\w+Tool)\b", body))):
            tf = struct_file.get(t)
            if tf is None:
                continue
            prod = strip_test_gated(read_bytes(tf)).decode("utf-8", "replace")
            # **只取该结构体自己的 id** —— 同文件里常有多个兄弟工具结构体
            # （如 play_tools.rs 的 4 件）⇒ 必须按结构体名定位到它的 `impl` 块。
            for cid in struct_own_ids(prod, t):
                ids.add(cid)
        # 族前缀兜底（模块 id 后缀 = 族名）：`module.tool.recon` ⇒ `tool.recon.*`
        suffix = mid.split(".")[-1]
        fam = "tool." + suffix
        ids.update(i for i in registered if i == fam or i.startswith(fam + "."))
        ids = {i for i in ids if i in registered}
        if ids:
            out[mod] = sorted(ids)
    return out


def struct_own_ids(src: str, struct: str) -> list[str]:
    """一个工具结构体**自己**声明的能力 id。

    定位口径：`impl <Struct> { … }` 块里出现的 `CapabilityId::new("tool.x")` 或
    `CapabilityId::new(SOME_CAPABILITY_ID)`（常量引用 ⇒ 回查同文件的常量字面量）。
    ⇒ 同文件的兄弟结构体的 id 不会被误算进来。
    """
    out: list[str] = []
    # 同文件的 `X_CAPABILITY_ID` 常量 ⇒ 字面量
    consts = dict(re.findall(r'const (\w*CAPABILITY_ID)\s*:\s*&str\s*=\s*"([^"]+)"', src))
    for m in re.finditer(r"impl\s+" + re.escape(struct) + r"\s*\{", src):
        start = src.index("{", m.end())
        depth = 0
        j = start
        while j < len(src):
            if src[j] == "{":
                depth += 1
            elif src[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        blk = src[start:j]
        for im in re.finditer(r'CapabilityId::new\(\s*"([^"]+)"', blk):
            if im.group(1).startswith("tool."):
                out.append(im.group(1))
        for im in re.finditer(r"CapabilityId::new\(\s*(\w+)\s*\)", blk):
            lit = consts.get(im.group(1))
            if lit and lit.startswith("tool."):
                out.append(lit)
    return list(dict.fromkeys(out))


def tool_side_env(cid: str) -> str:
    """工具侧自带「注册即闸」的能力 id ⇒ 它读的 env 名（无则空）。

    派生口径：在**该能力 id 的实现文件**里找 `APEIRETH_ENABLE_*` 常量声明
    （`const X_ENABLE_ENV: &str = "APEIRETH_…"`）⇒ 取其字面量。
    ⇒ 不写死任何开关名（新增能力自带闸 ⇒ 自动纳管）。
    """
    reg = _registered_cache()
    rel = reg.get(cid, [])
    if not rel:
        return ""
    p = ROOT / rel[0][0]
    src = read(p)
    for m in re.finditer(r'const\s+\w*ENABLE\w*_ENV\s*:\s*&str\s*=\s*"(APEIRETH_[A-Z0-9_]+)"', src):
        return m.group(1)
    # 工具侧可能是 `std::env::var("APEIRETH_…")` 直接读
    m = re.search(r'env::var\(\s*"(APEIRETH_ENABLE_[A-Z0-9_]+)"', src)
    return m.group(1) if m else ""


#: 生产注册面扫描结果缓存（一次扫描，全表复用 ⇒ 避免重复扫全仓）
_REGISTERED_CACHE: dict[str, list[tuple[str, int]]] | None = None


def _registered_cache() -> dict[str, list[tuple[str, int]]]:
    global _REGISTERED_CACHE
    if _REGISTERED_CACHE is None:
        _REGISTERED_CACHE = scan_registered_ids()
    return _REGISTERED_CACHE


def config_defaults() -> dict[str, str]:
    """`ProductionModulesConfig::default()` ⇒ 字段 → 默认值文本。"""
    src = read(PROD_CONFIG)
    m = re.search(r"impl Default for ProductionModulesConfig\s*\{", src)
    if not m:
        return {}
    blk = src[m.end():src.index("\n    }\n", m.end())]
    out = {}
    for fm in re.finditer(r"^\s{12}(\w+):\s*([^,\n]+)", blk, re.M):
        out[fm.group(1)] = fm.group(2).strip().rstrip(",")
    return out


def cli_field_exprs() -> dict[str, str]:
    """组合根 `CognitiveModuleConfig { … }` ⇒ 字段 → 装配表达式。"""
    src = read(CLI_ROOT)
    i = src.find("let config = CognitiveModuleConfig {")
    if i < 0:
        return {}
    j = src.find("..CognitiveModuleConfig::default()", i)
    if j < 0:
        j = len(src)
    blk = src[i:j]
    out = {}
    # 字段 → 装配表达式。表达式可能跨行（如 `http_write: http_write_config_from_env()?,`
    # 后面还跟着续行），这里按「字段名: 起点」切分，取到下一个字段名之前的内容。
    marks = [(fm.start(), fm.group(1)) for fm in re.finditer(r"^\s{8}(\w+):", blk, re.M)]
    for k, (pos, name) in enumerate(marks):
        end = marks[k + 1][0] if k + 1 < len(marks) else len(blk)
        seg = blk[pos:end]
        # 去掉字段名与冒号，取到第一个逗号（或整段）
        val = seg.split(":", 1)[1]
        val = val.split(",")[0]
        out[name] = val.strip()
    return out


def cli_env_consts() -> dict[str, str]:
    """`const X_ENV: &str = "APEIRETH_…"` ⇒ 常量名 → env 名。"""
    src = read(CLI_ROOT)
    return dict(re.findall(r'const (\w+_ENV)\s*:\s*&str\s*=\s*"([^"]+)"', src))


def cli_env_fns() -> dict[str, list[str]]:
    """组合根里的「读 env 的函数」⇒ 函数名 → 它读的 env 名。

    复刻「默认档」语义所需的最小信息：函数体里出现哪些 env 常量。
    覆盖两种返回型：
    * `fn X_enabled_from_env() -> bool`（开关判定）
    * `fn X_config_from_env() -> Result<Option<…>>`（配置槽；无配置 ⇒ 默认关）
    """
    src = read(CLI_ROOT)
    consts = cli_env_consts()
    out: dict[str, list[str]] = {}
    for m in re.finditer(r"fn (\w+_(?:enabled|config)_from_env)\s*\(\s*\)\s*->[^{]*\{", src):
        start = m.end()
        depth = 1
        i = start
        while i < len(src) and depth:
            if src[i] == "{":
                depth += 1
            elif src[i] == "}":
                depth -= 1
            i += 1
        body = src[start:i]
        envs = []
        for c in re.findall(r"\b(\w+_ENV)\b", body):
            if c in consts and consts[c] not in envs:
                envs.append(consts[c])
        out[m.group(1)] = envs
    return out


def core_memory_knobs() -> set[str]:
    """走「默认开」语义（`core_memory_knob_enabled`）的 env 函数名集合。"""
    src = read(CLI_ROOT)
    out = set()
    for m in re.finditer(r"fn (\w+_enabled_from_env)\(\)\s*->\s*bool\s*\{", src):
        start = m.end()
        depth = 1
        i = start
        while i < len(src) and depth:
            if src[i] == "{":
                depth += 1
            elif src[i] == "}":
                depth -= 1
            i += 1
        if "core_memory_knob_enabled" in src[start:i]:
            out.add(m.group(1))
    return out


def roster_switches() -> dict[str, str]:
    """`self_status_roster.rs` 名册 ⇒ 开关名 → (生效值表达式, 行号)。"""
    src = read(ROSTER)
    out: dict[str, tuple[str, int]] = {}
    for m in re.finditer(r'CapabilitySwitch::new\(\s*"([^"]+)",\s*', src):
        start = m.end()
        depth = 1
        i = start
        while i < len(src) and depth:
            if src[i] == "(":
                depth += 1
            elif src[i] == ")":
                depth -= 1
            i += 1
        line = src[:m.start()].count("\n") + 1
        out[m.group(1)] = (" ".join(src[start:i].split()), line)
    return out


def tools_crate_modules() -> list[tuple[str, Path]]:
    """`crates/capabilities/tools/src/lib.rs` 声明的模块 ⇒ (名, 路径)。"""
    src = read(TOOLS_LIB)
    out = []
    for m in re.finditer(r"^\s*pub(?:\(crate\))?\s+mod\s+(\w+)\s*;", src, re.M):
        name = m.group(1)
        f = TOOLS_LIB.parent / f"{name}.rs"
        d = TOOLS_LIB.parent / name
        if f.is_file():
            out.append((name, f))
        elif d.is_dir():
            out.append((name, d))
    return out


def module_declares_capability(path: Path) -> bool:
    """模块自身（递归）是否产出任何 `tool.*` 能力 id。"""
    if path.is_file():
        files = [path]
    else:
        files = [p for p in sorted(path.rglob("*.rs")) if p.name not in ("tests.rs", "test.rs")]
    for p in files:
        prod = strip_test_gated(read_bytes(p))
        if extract_capability_ids(prod):
            return True
    return False


def module_has_tool_capability(path: Path) -> bool:
    """模块自身（递归）是否有 `impl ToolCapability for`。"""
    if path.is_file():
        files = [path]
    else:
        files = [p for p in sorted(path.rglob("*.rs")) if p.name not in ("tests.rs", "test.rs")]
    for p in files:
        if re.search(r"impl ToolCapability for", read(p)):
            return True
    return False


def production_constructs(type_name: str) -> list[tuple[str, int]]:
    """某个类型在生产装配根（`production.rs`）有没有构造点。"""
    src = read(PRODUCTION)
    out = []
    for m in re.finditer(r"\b" + re.escape(type_name) + r"\s*::", src):
        line = src[:m.start()].count("\n") + 1
        out.append(("production.rs", line))
    return out


def anywhere_constructs(type_name: str) -> list[tuple[str, int]]:
    """某个类型在 `crates/` 生产面（剔除 tests 与 cfg(test)）有没有构造点。"""
    out = []
    for p in collect_production_sources():
        prod = strip_test_gated(read_bytes(p)).decode("utf-8", "replace")
        for m in re.finditer(r"\b" + re.escape(type_name) + r"\s*::", prod):
            out.append((str(p.relative_to(ROOT)), prod[:m.start()].count("\n") + 1))
    return out


def tool_names() -> dict[str, str]:
    """能力 id ⇒ 模型面工具名（`NormalizedTool::new(…)`）。

    两种源形态，各自按**同一文件内**的结构体配对：

    * **常量形态**：`X_CAPABILITY_ID` 常量与 `X_TOOL_NAME` 常量同名前缀 ⇒ 直接配对。
    * **实现形态**：`impl <Struct> { … CapabilityId::new("tool.x") … }` 给出能力 id，
      紧邻的 `impl ToolCapability for <Struct> { … NormalizedTool::new(…) }` 给出模型面名
      —— 两者**按结构体名**配对（不按出现顺序，避免错配）。
    """
    out: dict[str, str] = {}
    for p in collect_production_sources():
        src = read(p)
        # 形态 A：常量配对（同名前缀）
        consts = dict(re.findall(r'const (\w+_CAPABILITY_ID)\s*:\s*&str\s*=\s*"([^"]+)"', src))
        names = dict(re.findall(r'const (\w+_TOOL_NAME)\s*:\s*&str\s*=\s*"([^"]+)"', src))
        for cname, cid in consts.items():
            stem = cname[: -len("_CAPABILITY_ID")]
            tname = names.get(stem + "_TOOL_NAME")
            if tname:
                out[cid] = tname
        # 形态 B：按结构体名配对
        struct_id: dict[str, str] = {}
        for m in re.finditer(r"impl\s+(\w+)\s*\{", src):
            body = src[m.end():m.end() + 4000]
            im = re.search(r'CapabilityId::new\(\s*"([^"]+)"', body)
            if im:
                struct_id.setdefault(m.group(1), im.group(1))
        for m in re.finditer(r"impl ToolCapability for (\w+)", src):
            struct = m.group(1)
            blk = src[m.end():m.end() + 4000]
            nm = re.search(r'NormalizedTool::new\(\s*"([^"]+)"', blk)
            if not nm:
                nm = re.search(r"NormalizedTool::new\(\s*(\w+_TOOL_NAME)\s*\)", blk)
                if nm:
                    lit = names.get(nm.group(1))
                    if lit:
                        out.setdefault(struct_id.get(struct, ""), lit)
                        continue
                continue
            cid = struct_id.get(struct)
            if cid:
                out[cid] = nm.group(1)
    return {k: v for k, v in out.items() if k}


def evidence_alive(ev: str) -> tuple[bool, str]:
    """证据串里的 file:line 是否仍指向活文件（取第一处）。"""
    for m in re.finditer(r"([A-Za-z0-9_./-]+\.rs):(\d+)(?:-(\d+))?", ev):
        rel, ln = m.group(1), int(m.group(2))
        for cand in (rel, f"crates/engine/runtime-assembly/src/canonical/{rel}",
                     f"crates/engine/runtime/src/canonical/{rel}"):
            p = ROOT / cand
            if p.is_file():
                total = len(read(p).splitlines())
                if ln <= total:
                    return True, f"{cand}:{ln}/{total}"
                return False, f"{cand}:{ln} 超出文件行数 {total}"
    return True, ""


def switch_env(sw: str) -> str:
    """从开关串里抽出完整 env 名（若无则空）。"""
    m = re.search(r"(APEIRETH_[A-Z0-9_]+)", sw)
    return m.group(1) if m else ""


# ---------------------------------------------------------------- 生成


def build_table() -> tuple[str, list[str], list[str]]:
    """生成表体 + 致命问题清单 + 提示清单。

    * **致命**（`problems`）：表自身与真源不一致 ⇒ `--check` 非 0 退出。
    * **提示**（`advisories`）：上游真源自己的缺口（策展表漏登/幽灵条目）⇒
      照实报出，但**不**判红（表已如实反映现状；守卫在 Rust 侧）。
    """
    problems: list[str] = []

    registered = _registered_cache()
    reg_ids = set(registered)
    entries = taxonomy_entries()
    tags = taxonomy_tags()
    lib_faces, dyn_roots = taxonomy_faces()
    envs = env_registry()
    claim_map = claims()
    defaults = config_defaults()
    cli_exprs = cli_field_exprs()
    cli_fns = cli_env_fns()
    cli_consts = cli_env_consts()
    core_knobs = core_memory_knobs()
    roster = roster_switches()
    names = tool_names()

    # ---- ① 两向完整性（与 Rust 侧完整性闸门同口径）----
    # 分两类：
    #   **致命** —— 表自身与真源不一致（证据失效 / 开关未登记 / 默认档冲突 /
    #     台账等级越界）⇒ `--check` 非 0 退出。
    #   **提示** —— **上游策展表**与生产面不一致（漏登 / 幽灵条目）。这不是本表的
    #     漂移（表已如实反映现状），而是上游真源自己的缺口 ⇒ 照实报出，但
    #     不因此判红（Rust 侧 `tool_taxonomy` 完整性闸门才是它的守卫）。
    curated = {e["key"] for e in entries}
    advisories: list[str] = []
    for key in sorted(curated):
        if key in reg_ids or key in dyn_roots or key in lib_faces:
            continue
        advisories.append(
            f"策展表收了生产未注册的能力 id（幽灵条目）: {key} ⇒ "
            f"要么它已退库（改策展表），要么它真注册了（补进本表）"
        )
    for cid in sorted(reg_ids):
        if cid in curated:
            continue
        if any(cid.startswith(root + ".") for root in dyn_roots):
            continue
        advisories.append(
            f"生产已注册但策展表漏登的能力 id: {cid} ⇒ "
            f"先登记 tool_taxonomy.rs（Rust 完整性闸门会红），再跑 --gen"
        )

    # ---- 生产注册面 → 装配根可达性 ----
    prov2ids = provider_to_ids()
    wired, _sites = wired_capability_ids(reg_ids)

    # 生产注册面里但**装配根不可达**的 id
    #     —— 生产注册面 = 全树扫描命中；但**装配根可达** = 该 id 所属面在装配根
    #     真的被注册过。两者之差 = 「有 id 但装配根不可达」。
    unwired = sorted(reg_ids - set(wired))

    # ---- ② 默认档 / 开关名派生 ----
    # 组合根表达式 → (开关 env 名, 默认档)
    def field_switch(field: str) -> tuple[str, str]:
        expr = cli_exprs.get(field)
        if expr is None:
            # 未在组合根覆盖 ⇒ 取 `ProductionModulesConfig::default()` 的默认值
            dv = defaults.get(field, "")
            if dv == "true":
                return "", "ON"
            if dv in ("false", "None"):
                return "", "OFF"
            return "", "OFF"
        m = re.search(r"\b(\w+_enabled_from_env)\(\)", expr)
        if m:
            fn = m.group(1)
            envs_used = cli_fns.get(fn, [])
            env = envs_used[0] if envs_used else ""
            if fn in core_knobs:
                return env, "ON"
            return env, "OFF"
        m = re.search(r"\b(\w+)\.then\(", expr)
        if m:
            # `X_enabled.then(…)` —— X 由 env 派生
            fn = m.group(1)
            envs_used = cli_fns.get(fn, [])
            env = envs_used[0] if envs_used else ""
            if fn in core_knobs:
                return env, "ON"
            return env, "OFF"
        m = re.search(r"\b(\w+_config_from_env)\(\)", expr)
        if m:
            # `X_config_from_env()?` —— 配置槽：env 给了才 Some ⇒ 默认无配置 ⇒ 关
            fn = m.group(1)
            envs_used = cli_fns.get(fn, [])
            env = envs_used[0] if envs_used else ""
            return env, "OFF"
        if expr.startswith("Some("):
            return "", "ON"
        if expr.startswith("None"):
            return "", "OFF"
        if expr == "true":
            return "", "ON"
        if expr == "false":
            return "", "OFF"
        # 形如 `env_switches::recon_config_from_env()?` ⇒ 默认无配置 ⇒ 关
        return "", "OFF"

    # 能力 id ⇒ 族（模块 id 后缀）
    id_family: dict[str, str] = {}
    for mod, ids in prov2ids.items():
        mid = provider_modules().get(mod, "")
        fam = mid.split(".")[-1] if mid else ""
        for cid in ids:
            id_family[cid] = fam

    # 模块 ⇒ 装配点行号 ⇒ 能力 id ⇒ 门控字段
    gate_by_family: dict[str, str] = {}
    for mod, ids in prov2ids.items():
        mid = provider_modules().get(mod, "")
        fam = mid.split(".")[-1]
        gate_by_family[fam] = fam

    # 能力 id ⇒ 门控配置字段（从装配根 `if config.X` / `if let Some(..) = &config.X` 反查）
    prod_src = read(PRODUCTION)
    prod_lines = prod_src.splitlines()
    gate_field: dict[str, str] = {}
    for site in assembly_sites():
        mod = site["provider"]
        if not mod:
            continue
        # 向上找最近的 `if config.<field>` / `if let Some(..) = &config.<field>` 守卫
        # （跳过不含 `config.` 的 `if let`，否则会抓到嵌套的无关守卫）
        field = ""
        for j in range(site["line"] - 2, max(0, site["line"] - 60), -1):
            lm = prod_lines[j]
            if not re.match(r"\s*(if|else if)\b", lm):
                continue
            fm = re.search(r"\bconfig\.(\w+)", lm)
            if not fm:
                continue
            field = fm.group(1)
            break
        for cid in prov2ids.get(mod, []):
            gate_field[cid] = field

    def switch_of(cid: str) -> tuple[str, str]:
        # 显式覆盖：自动推导对不齐的少数族（口径：开关**复用**既有 env 名）。
        # `tool.process_tree` 复用 `APEIRETH_ENABLE_SHELL`（注册即闸 ⇒ shell 关 = 无进程可看）。
        if cid in SWITCH_OVERRIDES:
            return SWITCH_OVERRIDES[cid]
        fam = id_family.get(cid, "")
        field = gate_field.get(cid, "") or fam
        env, dv = field_switch(field)
        if not env and dv == "OFF":
            # 组合根无该字段且默认关 ⇒ 工具侧可能自带注册即闸
            # （如 `calendar` 的 `APEIRETH_ENABLE_CALENDAR`）⇒ 从工具源码反查。
            env = tool_side_env(cid)
        return env, dv

    lines: list[str] = []

    # ================= ① 生产装配根可达 · 静态能力 id =================
    lines.append("### ① 生产装配根可达 · 静态能力 id")
    lines.append("")
    lines.append(
        "| # | 能力 id | 模型面工具名 | 所属族/域 | 四级口径 | 默认 ON/OFF | 开关名 | 证据（file:line，回核过） | 真源出处 |"
    )
    lines.append("|---:|---|---|---|:---:|:---:|---|---|---|")

    static_ids = sorted(cid for cid in reg_ids if cid in curated and cid in wired)
    row_no = 0
    on_count = 0
    for cid in static_ids:
        row_no += 1
        tname = names.get(cid, "（未在源中派生）")
        tg = tags.get(cid, [])
        fam = id_family.get(cid, "")
        fam_label = fam or "—"
        if tg:
            fam_label = f"{fam_label}（域：{'/'.join(tg)}）"
        env, dv = switch_of(cid)
        if dv == "ON":
            on_count += 1
        # 证据：注册点 + 装配点
        reg_sites = registered.get(cid, [])
        ev_bits = []
        if reg_sites:
            rel, ln = reg_sites[0]
            ev_bits.append(f"注册 {rel}:{ln}")
        if cid in wired:
            rel, ln = wired[cid]
            ev_bits.append(f"装配 {rel}:{ln}")
        if not ev_bits:
            ev_bits.append("未证实")
        evidence = "；".join(ev_bits)
        alive, anchor = evidence_alive(evidence)
        if not alive:
            problems.append(f"{cid}: 证据已失效（{anchor}）⇒ 更新实现或改本表")
        # 真源出处
        src_bits = ["策展表 tool_taxonomy.rs"]
        if cid in claim_map:
            src_bits.append(f"台账 claims.yaml#{cid}")
        if env:
            src_bits.append(f"env 登记表 {env}")
        if cid in wired:
            src_bits.append(f"装配根 production.rs:{wired[cid][1]}")
        provenance = "；".join(src_bits)
        # 开关回核：env 必须登记
        if env and env not in envs:
            problems.append(f"{cid}: 开关 {env} 未在 env 登记表里 ⇒ 补登记或改实现")
        if env and env in envs:
            reg_default = str(envs[env].get("default", "")).lower()
            if reg_default in ("on", "off") and dv.lower() != reg_default:
                problems.append(
                    f"{cid}: 默认档冲突（本表={dv} · env 登记表 {env}.default={reg_default}）"
                    f"⇒ 以登记表为准并改实现"
                )
        lines.append(
            f"| {row_no} | `{cid}` | {tname} | {fam_label} | PRODUCTION WIRED | {dv} | "
            f"{env if env else '（无 env 开关）'} | {evidence} | {provenance} |"
        )

    # 动态族（族根取自策展表 DYNAMIC_FAMILY_ROOTS）
    for root in sorted(dyn_roots):
        dyn_id = root + ".<server>.<tool>"
        dyn_evidence = "族根 tool_taxonomy.rs"
        lines.append(
            f"| — | **`{dyn_id}`**（动态族，逐个外部工具生成） | mcp:<server>:<tool> | Mcp | "
            f"IMPLEMENTED+PRODUCTION WIRED｜DEFAULT ENABLED 不成立｜HARDWARE VALIDATED 不成立 | OFF | "
            f"{'APEIRETH_ENABLE_MCP'} | {dyn_evidence} | 策展表 tool_taxonomy.rs（DYNAMIC_FAMILY_ROOTS） |"
        )

    # ================= ② 有 id 但生产装配根不可达 =================
    lines.append("")
    lines.append("### ② 有能力 id 形态，但生产装配根**不可达**")
    lines.append("")
    lines.append(
        "| # | 能力 id | 模型面工具名 | 所属族/域 | 四级口径 | 默认 ON/OFF | 开关名 | 证据（file:line） | 为什么不可达 | 真源出处 |"
    )
    lines.append("|---:|---|---|---|:---:|:---:|---|---|---|---|")

    # ②a 库级保留面（策展表显式登记，但仓内无 ToolCapability 实现）
    #     证据 = 该面的**实现文件**（库级面在 tools crate 下），行号取其
    #     结构体/函数声明行；找不到 ⇒ 如实写「未证实」。
    for i, fid in enumerate(sorted(lib_faces), 1):
        mod_name = fid.split(".")[-1]
        # 结构体名按驼峰匹配（`std_sub_supervisor` ⇒ `StdSubSupervisor`）
        camel = "".join(w.capitalize() for w in mod_name.split("_"))
        ev = "未证实"
        for p in collect_production_sources():
            rel = str(p.relative_to(ROOT))
            if "/canonical/" in rel:
                continue
            if not re.search(r"/" + re.escape(mod_name) + r"(\.rs|/)", rel):
                continue
            src = read(p)
            dm = re.search(
                r"^(?:pub(?:\(crate\))?\s+)?(?:struct|fn|mod|trait|enum)\s+"
                + re.escape(camel) + r"\w*\b",
                src, re.M,
            )
            if dm:
                ev = f"{rel}:{src[:dm.start()].count(chr(10)) + 1}"
                break
        alive, anchor = evidence_alive(ev)
        if not alive:
            problems.append(f"{fid}: 证据已失效（{anchor}）")
        lines.append(
            f"| {i} | `{fid}` | 无（库级面，非工具） | {LIBRARY_FACE_LABEL.get(fid, '库级保留面')} | "
            f"IMPLEMENTED（库级）｜PRODUCTION WIRED 不成立（永不可达）｜HARDWARE VALIDATED 不成立 | 不适用 | "
            f"无（策略分类器按 id 形态引用） | {ev} | 库级保留面（**无 `ToolCapability` 实现**） | "
            f"策展表 tool_taxonomy.rs（LIBRARY_LEVEL_FACES） |"
        )

    # ②b 有 `ToolCapability` 实现但生产装配根无构造点的 id
    #     （由「注册面 ∩ 装配点」的差集与实现位置推出）
    unwired_with_impl = []
    for cid in unwired:
        # 该 id 的注册点文件里是否有 ToolCapability 实现 → 有实现却没装配点 ⇒ 不可达
        rel = registered[cid][0][0]
        p = ROOT / rel
        if module_has_tool_capability(p.parent if p.is_file() else p):
            unwired_with_impl.append(cid)
    row_no2 = len(lib_faces)
    for cid in sorted(unwired_with_impl):
        row_no2 += 1
        rel, ln = registered[cid][0]
        alive, anchor = evidence_alive(f"{rel}:{ln}")
        if not alive:
            problems.append(f"{cid}: 证据已失效（{anchor}）")
        lines.append(
            f"| {row_no2} | `{cid}` | {names.get(cid, '（未在源中派生）')} | {id_family.get(cid, '—')} | "
            f"IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | "
            f"注册 {rel}:{ln} | 生产装配根无构造点 | 仓内实现 |"
        )

    # ②c **运行期生成**的能力 id（`format!("tool.<族>.{…}")`）—— 源码里不是字面量，
    #     静态扫描看不到，但它们**确实存在**（memory 7 件 / mesh 3 件）。
    #     这些 id 的模块在生产装配根**无构造点** ⇒ 不可达。
    runtime_ids = runtime_format_ids()
    for cid in sorted(runtime_ids):
        row_no2 += 1
        rel, ln = runtime_ids[cid][0]
        alive, anchor = evidence_alive(f"{rel}:{ln}")
        if not alive:
            problems.append(f"{cid}: 证据已失效（{anchor}）")
        fam = cid.rsplit(".", 1)[0]
        lines.append(
            f"| {row_no2} | `{cid}` | {cid.split('.')[-1]}（运行期生成） | {fam}（运行期生成） | "
            f"IMPLEMENTED｜PRODUCTION WIRED 不成立｜HARDWARE VALIDATED 不成立 | OFF | 无（无 env 开关） | "
            f"id 生成 {rel}:{ln} | 能力 id 运行期拼出（`format!`）⇒ 静态策展表不收 | "
            f"运行期拼装 + 装配根无构造点 |"
        )
    unwired_count = len(lib_faces) + len(unwired_with_impl) + len(runtime_ids)

    # ================= ③ 库级模块面（无能力 id）=================
    lines.append("")
    lines.append("### ③ 库级模块面（**有实现、无能力 id ⇒ 不是能力**）")
    lines.append("")
    lines.append("| 库级面 | 所属族/域 | 用途 | 证据（file:line） | 真源出处 |")
    lines.append("|---|---|---|---|---|")
    # 库级面 = tools crate 里**不产出任何能力 id**的模块。三条排除规则：
    # ① 自身产出能力 id ⇒ 那是能力面（不是库级面）；
    # ② 自身 `impl ToolCapability` ⇒ 它**托管**某个能力（如 `exec_pipeline` 包装
    #    内层能力、`mcp_bridge` 承载动态族）⇒ 是能力面，不是库级面；
    # ③ 库级保留面（`LIBRARY_LEVEL_FACES`）已在 ② 列出 ⇒ 不重复列。
    lib_face_mods = {f.split(".")[-1] for f in lib_faces}
    libmod = []
    for name, path in tools_crate_modules():
        if module_declares_capability(path):
            continue
        if module_has_tool_capability(path):
            continue
        if name in lib_face_mods:
            continue
        if name in ("plugin", "test_support"):
            continue
        libmod.append((name, path))
    for name, path in sorted(libmod):
        rel = str(path.relative_to(ROOT))
        # 证据 = 该模块里第一个 `pub struct` / `pub fn` 声明行
        src = read(path) if path.is_file() else ""
        dm = re.search(r"^(?:pub(?:\(crate\))?\s+)?(?:struct|fn|enum|trait)\s+\w+", src, re.M)
        ev = f"{rel}:{src[:dm.start()].count(chr(10)) + 1}" if dm else f"{rel}:1"
        alive, anchor = evidence_alive(ev)
        if not alive:
            problems.append(f"{name}: 证据已失效（{anchor}）")
        lines.append(
            f"| `{name}` | 库级面（无能力 id） | 中间件/守卫/投影类实现，**不产出能力 id** | "
            f"{ev}（无 ToolCapability 实现） | tools crate `pub mod {name}` |"
        )

    # ================= ④ 认知 / 行为模块族 =================
    lines.append("")
    lines.append("### ④ 认知 / 行为模块族（**不是 `tool.*` 能力 id**，登记在 `tool.self_status` 名册里）")
    lines.append("")
    lines.append("| 模块 | 所属族/域 | 默认 ON/OFF | 开关名 | 证据（file:line） | 真源出处 |")
    lines.append("|:---:|---|:---:|---|---|---|")
    # 行为面 = 装配根 `modules.push(…)` 的**认知/行为模块**（不是工具面）。
    # 由装配根的模块注册点反查，再按名册登记的开关归集。
    behavior_mods: list[tuple[str, int]] = []
    prod_lines_all = read(PRODUCTION).splitlines()
    for i, ln in enumerate(prod_lines_all, 1):
        m = re.search(r"modules\.push\(Arc::new\((\w+)", ln)
        if m:
            behavior_mods.append((m.group(1), i))
        m2 = re.search(r"modules\.push\(Arc::new\(\s*$", ln)
        if m2 and i < len(prod_lines_all):
            m3 = re.search(r"(\w+)::new\(", prod_lines_all[i])
            if m3:
                behavior_mods.append((m3.group(1), i + 1))
    # 行为面的配置字段（名册表达式里的 config 字段）
    # 判定「工具面开关」：名册开关名本身对应某个**已装配的能力族**（① 里列过），
    # 或其 config 字段在装配根被 `collect_tool_module` 门控（工具面门控字段集）。
    tool_families = set(id_family.values())
    tool_gate_fields = {gate_field.get(c, "") for c in static_ids} - {""}
    tool_roster_names = set()
    for c in static_ids:
        fam = id_family.get(c, "")
        if fam:
            tool_roster_names.add(fam)
    beh_rows = []
    for sw, (expr, line) in sorted(roster.items()):
        fm = re.search(r"config\.(\w+)", expr)
        if not fm:
            continue
        field = fm.group(1)
        # 工具面开关 ⇒ 已在 ①（族名同名，或其门控字段是工具面门控字段）
        if sw in tool_roster_names or field in tool_gate_fields:
            continue
        if sw in ("voice_emotion_tune", "typed_recall", "self_tuning", "retrieval_tuning"):
            continue  # 注入槽参数（不是行为族开关）
        if sw in ("mcp", "mcp_bridge", "play_registered", "ritual_registered"):
            continue  # 协议桥/生物族**工具面**的注册事实（工具面本身在 ①）
        env, dv = field_switch(field)
        ev = f"crates/engine/runtime-assembly/src/canonical/self_status_roster.rs:{line}"
        alive, anchor = evidence_alive(ev)
        if not alive:
            problems.append(f"{sw}: 证据已失效（{anchor}）")
        beh_rows.append((sw, dv, env, ev))
    for sw, dv, env, ev in beh_rows:
        lines.append(
            f"| `{sw}` | 行为面（无 `tool.*` 能力 id） | {dv} | "
            f"{env if env else '（无 env 开关）'} | {ev} | 名册 self_status_roster.rs |"
        )
    beh_count = len(beh_rows)

    # ================= ⑤ 台账交叉核验 =================
    conflicts = []
    for cid, c in claim_map.items():
        lvl = str(c.get("level", ""))
        if lvl and lvl not in CALIBERS:
            conflicts.append(f"{cid}: 台账等级 `{lvl}` 不在四级口径词表")
    lines.append("")
    lines.append("### ⑤ 对外承诺台账（`docs/status/claims.yaml`）")
    lines.append("")
    dist: dict[str, int] = {}
    for c in claim_map.values():
        lvl = str(c.get("level", "?"))
        dist[lvl] = dist.get(lvl, 0) + 1
    lines.append("| 等级 | 条数 |")
    lines.append("|---|---:|")
    for k in sorted(dist, key=lambda x: -dist[x]):
        lines.append(f"| {k} | {dist[k]} |")
    lines.append(f"| **合计** | **{len(claim_map)}** |")
    if conflicts:
        lines.append("")
        lines.append("> ⚠️ **台账口径冲突**：")
        for c in conflicts:
            lines.append(f"> - {c}")
        problems.extend(conflicts)

    # ================= ⑥ 统计块 =================
    hw = sum(1 for c in claim_map.values() if str(c.get("level")) == "HARDWARE VALIDATED")
    off_count = len(static_ids) - on_count
    lines.append("")
    lines.append("### ⑥ 统计块")
    lines.append("")
    lines.append("| 指标 | 值 |")
    lines.append("|---|---:|")
    lines.append(f"| 生产可达静态能力 id | **{len(static_ids)}** |")
    lines.append(f"| 动态能力族 | **{len(dyn_roots)}**（成员数随运行期发现而变） |")
    lines.append(f"| 默认 ON | **{on_count}** |")
    lines.append(f"| 默认 OFF | **{off_count}** |")
    lines.append(f"| 有 id 但生产不可达 | **{unwired_count}** |")
    lines.append(f"| 无能力 id 的库级模块面 | **{len(libmod)}** |")
    lines.append(f"| 认知 / 行为模块族 | **{beh_count}** |")
    lines.append(f"| 对外承诺（claims） | **{len(claim_map)}** |")
    lines.append(f"| **HARDWARE VALIDATED 达成** | **{hw}** |")
    lines.append(f"| 策展表条目（`TOOL_TAXONOMY`） | **{len(curated)}** |")

    # ---- 上游真源缺口（提示，不判红）----
    if advisories:
        lines.append("")
        lines.append("### ⑦ 上游策展表缺口（**表已如实反映现状 ⇒ 不判红**）")
        lines.append("")
        lines.append("> 以下是**策展表自身**与生产面的出入（不是本表的漂移）。守卫在 Rust 侧")
        lines.append("> `tool_taxonomy` 的完整性闸门（`cargo test -p apeireth-runtime-assembly`）。")
        lines.append("")
        for a in advisories:
            lines.append(f"> - {a}")

    return "\n".join(lines), problems, advisories


def main() -> int:
    ap = argparse.ArgumentParser(description="能力大汇总表生成器 / 守卫")
    ap.add_argument("--gen", action="store_true", help="重算并重写生成块")
    ap.add_argument("--check", action="store_true", help="重算并比对（不一致 ⇒ 非 0 退出）")
    ap.add_argument("--report", action="store_true", help="只打印生成的表体 + 统计")
    args = ap.parse_args()

    body, problems, advisories = build_table()

    if args.report:
        print(body)
        if problems:
            print()
            print(f"[capability-summary] ⚠️ {len(problems)} 条真源问题：")
            for p in problems:
                print(f"  - {p}")
        return 0

    if args.gen:
        if not PAGE.exists():
            print("页面不存在 ⇒ 先建骨架（手写段）再 --gen", file=sys.stderr)
            return 1
        text = read(PAGE)
        if BEGIN not in text or END not in text:
            print("页面缺生成块标记 ⇒ 加 <!-- BEGIN:generated --> / <!-- END:generated -->", file=sys.stderr)
            return 1
        head, rest = text.split(BEGIN, 1)
        _, tail = rest.split(END, 1)
        PAGE.write_text(f"{head}{BEGIN}\n{body}\n{END}{tail}", encoding="utf-8")
        print(f"[capability-summary] 已重写生成块（{len(body.splitlines())} 行）")
        if advisories:
            print(f"[capability-summary] ℹ️ {len(advisories)} 条上游策展表缺口（不判红，守卫在 Rust 侧）：")
            for a in advisories:
                print(f"  - {a}")
        if problems:
            print(f"[capability-summary] ⚠️ {len(problems)} 条真源问题：")
            for p in problems:
                print(f"  - {p}")
            return 1
        print("[capability-summary] ✅ 真源回核全绿")
        return 0

    if args.check:
        if not PAGE.exists():
            print(f"页面不存在: {PAGE}", file=sys.stderr)
            return 1
        text = read(PAGE)
        if BEGIN not in text or END not in text:
            print("页面缺生成块标记", file=sys.stderr)
            return 1
        _, rest = text.split(BEGIN, 1)
        cur, _ = rest.split(END, 1)
        if cur.strip() != body.strip():
            print("[capability-summary] ❌ FAIL：生成块与真源不一致 ⇒ 跑 --gen")
            diff = list(
                difflib.unified_diff(
                    cur.strip().splitlines(), body.strip().splitlines(),
                    "页面", "真源", lineterm="", n=1,
                )
            )
            for line in diff[:60]:
                print("  " + line)
            print()
            print("怎么修：python3 scripts/gen_capability_summary.py --gen  （重算并重写生成块）")
            return 1
        if problems:
            print(f"[capability-summary] ❌ FAIL：{len(problems)} 条真源问题")
            for p in problems:
                print(f"  - {p}")
            print()
            print("怎么修：按上面每条指出的真源位置改实现/登记，再跑 --gen")
            return 1
        if advisories:
            print(f"[capability-summary] ℹ️ {len(advisories)} 条上游策展表缺口（不判红，守卫在 Rust 侧）：")
            for a in advisories:
                print(f"  - {a}")
        print("[capability-summary] ✅ PASS（表与真源一致）")
        return 0

    ap.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())
