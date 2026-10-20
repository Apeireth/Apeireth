#!/usr/bin/env python3
"""R3a：env 登记表播种 + wiring ledger 生成/漂移校验（ADR-0004）。

交付物（按 ADR-0004 R3a）：
  docs/architecture/env-registry.toml   —— 开关登记表（效率线维护；owner/level 起步 TODO）
  docs/status/wiring.yaml               —— 生成式接线账（禁手改；本次由 claims + 源码扫描生成）
  scripts/wiring_report.py              —— 本脚本（生成 + 校验；与 docs_truth.py 同模式）

用法:
  python scripts/wiring_report.py --seed-env-registry [--force]   # 一次性播种（committed HEAD 口径 + 在途名）
  python scripts/wiring_report.py --check-env-registry            # 覆盖/重复/TODO 统计（CI 用，缺登记即红）
  python scripts/wiring_report.py --gen                           # 生成 docs/status/wiring.yaml
  python scripts/wiring_report.py --verify                        # 重算并与盘上比对（drift≠0 即红，CI 用）
  python scripts/wiring_report.py --stats                         # 记分牌数字

口径:
- env 扫描口径与 arch_guard 对齐：workspace 包 `src/**/*.rs`（不含 tests），正则 APEIRETH_[A-Z0-9_]+。
- 播种取 **committed HEAD 内容**（未提交改动读 `git show HEAD:<path>`），避免把别线在途开关当既有；
  未提交树里多出的开关作为「注册先行」额外登记并标注。
- owner/level 起步为 TODO-OWNER / UNCLASSIFIED（不冒充 PRODUCTION WIRED），R3b 逐条填实。
- wiring.yaml 每项: id / capability / implemented(sources) / wired(production refs) /
  default_enabled / validated(tests+claims_id) / gaps（ADR-0004 §2）。
"""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONTRACT = ROOT / "docs" / "architecture" / "contract.toml"
REGISTRY = ROOT / "docs" / "architecture" / "env-registry.toml"
WIRING = ROOT / "docs" / "status" / "wiring.yaml"
CLAIMS = ROOT / "docs" / "status" / "claims.yaml"
MATRIX = ROOT / "docs" / "03-reference" / "capabilities-matrix.md"
ENV_RE = re.compile(r"APEIRETH_[A-Z0-9_]+")
FRAGMENT_RE = re.compile(r"<!--|-->|!\s*!\s*#|^\s*[!#]|^\s*[.·]{3,}")
SWITCH_HINT = re.compile(r"(ENABLE|DISABLE|FEATURE|ALLOW|STRICT|FORCE|SWITCH|TOGGLE|REQUIRE)")
PROD_PREFIXES = ("crates/", "frontend/companion-desktop/src-tauri/src/")
PROD_ROOTS = (
    "crates/engine/runtime-assembly/src/canonical/production.rs",
    "crates/adapters/cli/src/lib.rs",
    "crates/adapters/cli/src/care_heartbeat.rs",
    "crates/engine/runtime-assembly/src/canonical.rs",
    "crates/engine/runtime/src/canonical/canonical_entry.rs",
)

REGISTRY_HEADER = """# ============================================================================
# env 开关登记表（Env Registry）— ADR-0004 R3a
# ============================================================================
# 用途: 唯一开关登记真源。`arch_guard` 对 APEIRETH_* 的判定 = 基线 OR 本表。
#       R3a 阶段由效率线从实测自动播种；`owner = "TODO-OWNER"`、`level = "UNCLASSIFIED"`
#       起步（不冒充 PRODUCTION WIRED），逐波填实；TODO-OWNER 数量记分牌只减。
# 每条字段: name / default / layer / owner / purpose / level / status
#   level  : PRODUCTION WIRED | IMPLEMENTED | NOT IMPLEMENTED | UNCLASSIFIED（R3a 起步占位）
#   status : current | migrated | deprecated
# 纪律: 新增开关必须先登记（默认关 + 关时零变化测试），否则 arch_guard W3 违规；
#       deprecated 的开关不得再出现在 src（R3c 守卫收紧）。
# 生成/播种: 效率线维护（R3a）；格式由架构线定（ADR-0004），变更需 ADR。
# ============================================================================
"""


# --------------------------------------------------------------------- utils

def run(cmd: list[str]) -> tuple[int, str]:
    proc = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                          encoding="utf-8", errors="replace")
    return proc.returncode, (proc.stdout or "") + (proc.stderr or "")


def load_contract() -> dict:
    with open(CONTRACT, "rb") as fh:
        return tomllib.load(fh)


def crate_layers(contract: dict) -> dict[str, str]:
    mapping: dict[str, str] = {}
    for layer, crates in (contract.get("layers") or {}).items():
        for crate in crates or []:
            mapping[str(crate)] = str(layer)
    return mapping


def workspace_src_files() -> list[tuple[str, Path, str]]:
    """返回 (crate 名, 绝对路径, repo 相对路径)；口径 = workspace 包 src/**/*.rs。"""
    code, out = run(["cargo", "metadata", "--locked", "--no-deps", "--offline",
                     "--format-version", "1"])
    if code != 0:
        raise SystemExit(f"wiring_report: cargo metadata failed:\n{out[-1500:]}")
    data = json.loads(out)
    files: list[tuple[str, Path, str]] = []
    for pkg in data.get("packages", []):
        root = Path(pkg["manifest_path"]).parent
        src = root / "src"
        if not src.exists():
            continue
        for path in sorted(src.rglob("*.rs")):
            files.append((str(pkg["name"]), path, path.relative_to(ROOT).as_posix()))
    return files


def dirty_and_untracked() -> tuple[set[str], set[str]]:
    dirty: set[str] = set()
    code, out = run(["git", "diff", "--name-only", "HEAD"])
    if code == 0:
        dirty = {line.strip().replace("\\", "/") for line in out.splitlines() if line.strip()}
    untracked: set[str] = set()
    code, out = run(["git", "ls-files", "--others", "--exclude-standard"])
    if code == 0:
        untracked = {line.strip().replace("\\", "/") for line in out.splitlines() if line.strip()}
    return dirty, untracked


def committed_text(rel: str, dirty: set[str], untracked: set[str]) -> str | None:
    """committed HEAD 口径：未提交改动读 HEAD 版本；未跟踪文件跳过。"""
    if rel in untracked:
        return None
    if rel in dirty:
        code, out = run(["git", "show", f"HEAD:{rel}"])
        return out if code == 0 else None
    path = ROOT / rel
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None


def purpose_near(lines: list[str], idx: int) -> str:
    parts: list[str] = []
    j = idx - 1
    while j >= 0 and len(parts) < 3:
        s = lines[j].strip()
        if s.startswith("//") or s.startswith("#"):
            text = s.lstrip("/#").strip()
            if text and not set(text) <= {"=", "-"}:
                parts.append(text)
            j -= 1
            continue
        break
    return " ".join(reversed(parts))[:140].strip()


def doc_comment_above(lines: list[str], idx: int) -> str:
    """取该行上方最近的连续文档注释（`///` / `//!`）；无则取单行 `//` 注释。"""
    parts: list[str] = []
    j = idx - 1
    while j >= 0:
        s = lines[j].strip()
        if s.startswith("///") or s.startswith("//!"):
            parts.append(s.lstrip("/!").strip())
            j -= 1
            continue
        break
    if parts:
        return _clean_purpose(" ".join(reversed(parts)))
    if idx - 1 >= 0 and lines[idx - 1].strip().startswith("//"):
        return _clean_purpose(lines[idx - 1].strip().lstrip("/").strip())
    return ""


def module_doc(lines: list[str]) -> str:
    """文件级模块文档（`//!`）——条目附近无 doc 注释时的来源兜底（仍属源码文档注释）。"""
    parts: list[str] = []
    for line in lines[:80]:
        s = line.strip()
        if s.startswith("//!"):
            parts.append(s[3:].strip())
        elif not parts and (not s or s.startswith("#!")):
            continue
        elif parts:
            break
    return _clean_purpose(" ".join(parts))[:160]


def default_hint(lines: list[str], idx: int) -> str:
    """从读取点附近推断源码默认值（on/off）；无法判定返回空串。"""
    window = " ".join(lines[max(0, idx - 4):min(len(lines), idx + 5)])
    match = re.search(r'unwrap_or(?:_else)?\(\s*"([^"]*)"', window)
    if match:
        value = match.group(1).strip().lower()
        return "on" if value in {"1", "true", "on", "yes"} else "off"
    if re.search(r"\.is_some\(\)|\.is_ok\(\)", window):
        return "off"
    return ""


def scan_env(committed: bool) -> dict[str, dict]:
    """扫描 env 名 -> {crate, layer, files, reads[(rel,line,is_comment)], doc, purpose, default_hint}。"""
    contract = load_contract()
    layers = crate_layers(contract)
    dirty, untracked = dirty_and_untracked() if committed else (set(), set())
    found: dict[str, dict] = {}
    for crate, _path, rel in workspace_src_files():
        text = committed_text(rel, dirty, untracked) if committed else _path.read_text(
            encoding="utf-8", errors="replace")
        if text is None:
            continue
        lines = text.splitlines()
        file_m_doc = module_doc(lines)
        file_names: set[str] = set()
        for idx, line in enumerate(lines):
            for match in ENV_RE.finditer(line):
                name = match.group(0)
                file_names.add(name)
                rec = found.setdefault(name, {
                    "crate": crate,
                    "layer": layers.get(crate, "unknown"),
                    "files": set(),
                    "reads": [],
                    "symbols": set(),
                    "doc": "",
                    "module_doc": file_m_doc,
                    "purpose": "",
                    "default_hint": "",
                })
                stripped = line.strip()
                if stripped.startswith(("//", "/*", "*")):
                    kind = "comment"
                elif "env::var" in line or "var_os(" in line:
                    kind = "read"
                elif "const " in line and "= " in line:
                    kind = "const"
                    sym = re.search(r"const\s+([A-Z][A-Z0-9_]*)", line)
                    if sym:
                        rec["symbols"].add(sym.group(1))
                else:
                    kind = "code"
                rec["files"].add(rel)
                rec["reads"].append((rel, idx + 1, kind))
                if not rec["doc"]:
                    rec["doc"] = doc_comment_above(lines, idx)
                if not rec["purpose"]:
                    rec["purpose"] = purpose_near(lines, idx)
                if not rec["default_hint"] and not is_comment_line(line):
                    rec["default_hint"] = infer_default(lines, idx)
        # 第二遍：把 `env::var(CONST)` 的读取点也记为证据（常量名不含字面 env 名时仍可核读取路径）
        for name in file_names:
            rec = found[name]
            for symbol in rec.get("symbols") or ():
                pattern = re.compile(rf"\b(?:env::)?var(?:_os)?\(\s*{re.escape(symbol)}\b")
                for j, line2 in enumerate(lines):
                    if pattern.search(line2):
                        rec["reads"].append((rel, j + 1, "readref"))
    # 第三遍：跨文件找常量符号的使用点（如 `env_flag(COST_ROUTING_ENV)`），作为读取路径证据
    symbol_owner: dict[str, str] = {}
    for name, rec in found.items():
        for symbol in rec.get("symbols") or ():
            symbol_owner.setdefault(symbol, name)
    if symbol_owner:
        combined = re.compile(r"\b(" + "|".join(
            re.escape(symbol) for symbol in sorted(symbol_owner)) + r")\b")
        for _crate, _path, rel in workspace_src_files():
            text = committed_text(rel, dirty, untracked) if committed else _path.read_text(
                encoding="utf-8", errors="replace")
            if text is None:
                continue
            for j, line2 in enumerate(text.splitlines()):
                if line2.strip().startswith(("//", "/*", "*")):
                    continue
                for match in combined.finditer(line2):
                    found[symbol_owner[match.group(1)]]["reads"].append((rel, j + 1, "symboluse"))
    return found


def is_comment_line(line: str) -> bool:
    return line.strip().startswith(("//", "/*", "*"))


EVIDENCE_PRIORITY = {"readref": 0, "read": 1, "symboluse": 2, "const": 3, "code": 4, "comment": 5}


def evidence_of(rec: dict) -> str:
    """读取证据优先级：env::var(CONST) 读取点 > 字面读取 > const 定义 > 其它代码 > 注释。"""
    reads = list(rec.get("reads") or [])
    dedup: dict[tuple[str, str], tuple[str, int, str]] = {}
    for read in reads:
        dedup[(read[0], str(read[1]))] = read
    picked = sorted(dedup.values(), key=lambda r: (EVIDENCE_PRIORITY.get(r[2], 9), r[1]))[:3]
    return ";".join(f"{read[0]}:{read[1]}" for read in picked)


def infer_default(lines: list[str], idx: int) -> str:
    """从语句内（读取行起、到分号止）推断源码默认值；无法判定返回空串。"""
    seg = " ".join(lines[idx:min(len(lines), idx + 14)])
    seg = seg.split(";", 1)[0]
    for pattern in (
        r"unwrap_or\(\s*Duration::from_millis\((\d[\d_]*)\)",
        r"unwrap_or\(\s*Duration::from_secs\((\d[\d_]*)\)",
    ):
        match = re.search(pattern, seg)
        if match:
            return str(int(match.group(1).replace("_", "")))
    match = re.search(r'unwrap_or\(\s*"([^"]*)"', seg)
    if match:
        value = match.group(1).strip()
        return "on" if value.lower() in {"1", "true", "on", "yes"} else "off"
    match = re.search(r"unwrap_or\(\s*([0-9]+(?:\.[0-9]+)?)\s*\)", seg)
    if match:
        return match.group(1)
    if "BackendKind::Auto" in seg or "unwrap_or(Auto)" in seg:
        return "auto"
    if "unwrap_or_default()" in seg:
        return "default"
    return ""


def env_refs_committed() -> dict[str, list[str]]:
    """committed HEAD 口径 env -> 出现文件（wiring 生成物必须与 CI checkout 树一致，排除未提交文件）。"""
    scan = scan_env(committed=True)
    refs: dict[str, list[str]] = {}
    for name, rec in scan.items():
        paths = {read[0] for read in (rec.get("reads") or []) if read[2] != "comment"}
        refs[name] = sorted(paths)
    return refs


def default_of(name: str) -> str:
    return "off" if SWITCH_HINT.search(name) else "n/a"


def toml_str(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def registry_entries() -> list[dict]:
    if not REGISTRY.exists():
        return []
    try:
        with open(REGISTRY, "rb") as fh:
            data = tomllib.load(fh)
    except Exception as exc:  # noqa: BLE001 - 旧格式/损坏需原样报因（--force 可重播种）
        raise SystemExit(f"wiring_report: cannot parse {REGISTRY}: {exc}") from exc
    return [e for e in data.get("entries", []) if isinstance(e, dict) and e.get("name")]


def render_registry(entries: list[dict], stats_note: str) -> str:
    lines = [REGISTRY_HEADER, "", "version = 1",
             f'updated_at = "{_today()}"', "",
             f"# {stats_note}", ""]
    for e in entries:
        lines.append("[[entries]]")
        for key in ("name", "default", "layer", "owner", "purpose", "level", "status"):
            lines.append(f"{key} = {toml_str(str(e.get(key, '')))}")
        evidence = str(e.get("evidence", "")).strip()
        if evidence:
            lines.append(f"evidence = {toml_str(evidence)}")
        lines.append("")
    return "\n".join(lines).rstrip("\n") + "\n"


def _today() -> str:
    import datetime
    return datetime.date.today().isoformat()


# ------------------------------------------------------------------- commands

def cmd_seed(force: bool) -> int:
    committed = scan_env(committed=True)
    working = scan_env(committed=False)
    in_flight = sorted(set(working) - set(committed))
    try:
        existing = registry_entries()
    except SystemExit as exc:  # 旧格式/损坏：--force 允许重播种
        if not force:
            print(f"FAIL: {exc}\n如确需重播种请加 --force", file=sys.stderr)
            return 1
        existing = []
    if existing and not force:
        print(f"FAIL: {REGISTRY.relative_to(ROOT)} 已有 entries；如确需重播种请加 --force（会丢 owner 填实）",
              file=sys.stderr)
        return 1
    entries: list[dict] = []
    for name in sorted(set(committed) | set(working)):
        rec = committed.get(name) or working[name]
        entries.append({
            "name": name,
            "default": default_of(name),
            "layer": rec["layer"],
            "owner": "TODO-OWNER",
            "purpose": rec["purpose"] or "TODO-PURPOSE",
            "level": "UNCLASSIFIED",
            "status": "current",
        })
    note = (f"seeded: committed HEAD={len(committed)}，未提交树额外={len(in_flight)}"
            + (f"（{', '.join(in_flight)} — 注册先行）" if in_flight else "")
            + "；owner/level 为 TODO 占位，R3b 填实")
    REGISTRY.write_text(render_registry(entries, note), encoding="utf-8")
    print(f"seeded: {REGISTRY.relative_to(ROOT)}  entries={len(entries)}  "
          f"committed={len(committed)}  in_flight={len(in_flight)}")
    return 0


def cmd_check_env_registry() -> int:
    committed = scan_env(committed=True)
    entries = registry_entries()
    names = [str(e.get("name")) for e in entries]
    dup = sorted({n for n in names if names.count(n) > 1})
    missing = sorted(set(committed) - set(names))
    print(f"env registry: {_counts_line(entries)}")
    print(f"  coverage={len(set(committed) & set(names))}/{len(committed)}  committed={len(committed)}")
    if dup:
        print(f"FAIL: duplicate entries: {dup}", file=sys.stderr)
    for name in missing:
        print(f"FAIL: committed env not registered: {name}", file=sys.stderr)
    if dup or missing:
        return 1
    print("OK: env registry covers all committed APEIRETH_* names")
    return 0


def _short(text: str, limit: int = 150) -> str:
    text = re.sub(r"\s+", " ", str(text or "")).strip()
    return text[:limit] + ("…" if len(text) > limit else "")


def is_production_source(rel: str) -> bool:
    rel = rel.replace("\\", "/")
    return (rel.startswith(PROD_PREFIXES) and "/tests/" not in rel and "/test/" not in rel
            and not rel.endswith((".md", ".ps1", ".mjs", ".toml")))


def load_claims() -> list[dict]:
    sys.path.insert(0, str(ROOT / "scripts"))
    import docs_truth  # noqa: PLC0415 - reuse the DTS parser (single parse口径)
    data = docs_truth.load_claims(CLAIMS)
    return list(data.get("claims") or [])


def yaml_str(value) -> str:
    return json.dumps(value, ensure_ascii=False)


def render_block_list(lines: list[str], indent: str, items: list[str]) -> None:
    if not items:
        lines.append(f"{indent}[]")
        return
    for item in items:
        lines.append(f"{indent}- {yaml_str(item)}")


def render_wiring() -> str:
    claims = load_claims()
    refs = env_refs_committed()
    matrix_text = MATRIX.read_text(encoding="utf-8")
    digest = hashlib.sha256(CLAIMS.read_bytes()).hexdigest()
    out: list[str] = [
        "# GENERATED by scripts/wiring_report.py — do not edit（生成物禁手改）",
        "# source: docs/status/claims.yaml + workspace src 扫描 + capabilities-matrix（ADR-0004 R3a）",
        "# regenerate: python scripts/wiring_report.py --gen；drift check: --verify",
        "schema: adr-0004-r3a",
        "generator: scripts/wiring_report.py",
        f"claims_sha256: {digest}",
        f"claims_total: {len(claims)}",
        "items:",
    ]
    for claim in claims:
        cid = str(claim.get("id", ""))
        level = str(claim.get("level", ""))
        switch = claim.get("switch_env")
        sources = [str(s) for s in (claim.get("sources") or [])]
        tests = [str(t) for t in (claim.get("tests") or [])]
        notes = str(claim.get("notes") or "")
        wired = {s for s in sources if is_production_source(s)}
        if switch:
            wired.update(refs.get(str(switch), []))
        wired = sorted(wired)
        gaps: list[str] = []
        if level != "PRODUCTION WIRED":
            gaps.append(f"未达 PRODUCTION WIRED：{_short(notes)}")
        elif not wired:
            gaps.append("未找到生产引用（待核）")
        if switch and str(switch) not in matrix_text:
            gaps.append(f"能力矩阵未出现开关 {switch}（待补）")
        out.append(f"  - id: {yaml_str(cid)}")
        out.append(f"    capability: {yaml_str(str(claim.get('title', '')))}")
        out.append("    implemented:")
        out.append("      sources:")
        _render_items(out, sources, "        ")
        out.append("    wired:")
        out.append("      production_refs:")
        _render_items(out, wired, "        ")
        default = claim.get("default")
        default_enabled = "n/a" if default is None else (default == "ON")
        out.append(f"    default_enabled: {yaml_str(default_enabled)}")
        out.append("    validated:")
        out.append("      tests:")
        _render_items(out, tests, "        ")
        out.append(f"      claims_id: {yaml_str(cid)}")
        out.append("    gaps:")
        _render_items(out, gaps, "      ")
    return "\n".join(out) + "\n"


def _render_items(out: list[str], items: list[str], indent: str) -> None:
    if not items:
        out.append(f"{indent}[]")
        return
    for item in items:
        out.append(f"{indent}- {yaml_str(item)}")


def cmd_gen() -> int:
    text = render_wiring()
    WIRING.parent.mkdir(parents=True, exist_ok=True)
    WIRING.write_text(text, encoding="utf-8")
    claims = load_claims()
    print(f"generated: {WIRING.relative_to(ROOT)}  claims={len(claims)}  bytes={len(text.encode('utf-8'))}")
    return 0


def cmd_verify() -> int:
    if not WIRING.exists():
        print(f"FAIL: {WIRING.relative_to(ROOT)} missing; run --gen", file=sys.stderr)
        return 1
    expected = render_wiring()
    actual = WIRING.read_text(encoding="utf-8")
    if expected == actual:
        print(f"OK: {WIRING.relative_to(ROOT)} up to date（claims 全覆盖，生成物未手改）")
        return 0
    diff = list(difflib.unified_diff(actual.splitlines(), expected.splitlines(),
                                     fromfile="on-disk", tofile="regenerated", lineterm=""))
    print(f"FAIL: {WIRING.relative_to(ROOT)} drift（生成物禁手改；run --gen）", file=sys.stderr)
    for line in diff[:80]:
        print(line, file=sys.stderr)
    return 1


# R3b 第一批：与 claims 开关同源、应同 owner 的伴生配置（有证据的成对/同文件变量）
RELATED_OWNERS = {
    "APEIRETH_TLS_KEY": ("gateway", "TLS 私钥（与 APEIRETH_TLS_CERT 成对；B5 交付）"),
    "APEIRETH_COST_ROUTING_PREFERRED": (
        "core", "成本分级路由档位映射（H-P2；与 APEIRETH_COST_ROUTING 同源 cost_router.rs）"),
}


def _clean_purpose(text: str, limit: int = 120) -> str:
    """清洗启发式 purpose 的 HTML 注释碎片等噪音（不算能力声明，不写 level）。"""
    text = re.sub(r"<!--.*?-->", " ", str(text), flags=re.S)
    text = re.sub(r"<[^>]*>", " ", text)
    text = re.sub(r"[!#]{1,}\s*", " ", text)
    text = re.sub(r"\s+", " ", text).strip(" !#:;，。")
    return text[:limit]


def _registry_counts(entries: list[dict]) -> tuple[int, int, int, int]:
    fragments = sum(1 for e in entries if FRAGMENT_RE.search(str(e.get("purpose", ""))))
    return (
        len(entries),
        sum(1 for e in entries if e.get("owner") == "TODO-OWNER"),
        sum(1 for e in entries if e.get("level") == "UNCLASSIFIED"),
        fragments,
    )


def _counts_line(entries: list[dict]) -> str:
    entries_n, todo, unclassified, fragments = _registry_counts(entries)
    return (f"entries={entries_n}  TODO-OWNER={todo}  "
            f"UNCLASSIFIED={unclassified}  purpose_fragments={fragments}")


def cmd_fill_layer(layer: str) -> int:
    """R3b 按层填实：owner=读取该 env 的 crate（证据），purpose=源码文档注释，default=源码默认值。"""
    scan = scan_env(committed=True)
    claims = load_claims()
    claim_by_switch = {str(c.get("switch_env")): c for c in claims if c.get("switch_env")}
    entries = registry_entries()
    target = [e for e in entries if str(e.get("layer")) == layer]
    if not target:
        print(f"FAIL: no registry entries in layer={layer}", file=sys.stderr)
        return 1
    problems: list[str] = []
    for entry in target:
        name = str(entry.get("name"))
        rec = scan.get(name)
        if not rec:
            problems.append(f"{name}: committed HEAD 扫描无读取点")
            continue
        reads = rec.get("reads") or []
        if not reads:
            problems.append(f"{name}: 无读取行")
            continue
        evidence = evidence_of(rec)
        purpose = str(rec.get("doc") or "").strip() or _clean_purpose(str(entry.get("purpose", "")))
        if not purpose or FRAGMENT_RE.search(purpose) or purpose == "TODO-PURPOSE":
            purpose = str(rec.get("module_doc") or "").strip() or "TODO-PURPOSE"
        if FRAGMENT_RE.search(purpose):
            purpose = "TODO-PURPOSE"
        purpose = _clean_purpose(purpose)
        claim = claim_by_switch.get(name)
        level = str(claim.get("level")) if claim else "UNCLASSIFIED"
        default = str(rec.get("default_hint") or "")
        if claim and str(claim.get("default") or "") in ("ON", "OFF"):
            default = "on" if str(claim.get("default")) == "ON" else "off"
        if not default:
            if name.startswith("APEIRETH_TUNE_") and name != "APEIRETH_TUNE_":
                default = "baseline"  # 源码: self_tuning.rs 模块文档「缺省/非法 → baseline」
            elif name == "APEIRETH_KEYRING_BACKEND":
                default = "auto"  # 源码: keyring.rs from_env_str「缺省/未知 → Auto」
            elif SWITCH_HINT.search(name):
                default = "off"
            else:
                default = "n/a"
        entry.update({
            "owner": rec["crate"],
            "purpose": purpose,
            "default": default,
            "level": level,
            "status": "current",
            "evidence": evidence,
        })
    subset = [e for e in entries if str(e.get("layer")) == layer]
    subset_counts = _registry_counts(subset)
    note = (f"R3b layer={layer} 批: owner=crate 证据、purpose=源码文档注释、default=源码默认值；"
            f"evidence 路径:行 必填；level 保守（claims 覆盖按 claim，其余 UNCLASSIFIED）")
    REGISTRY.write_text(render_registry(entries, note), encoding="utf-8")
    print(f"fill-layer {layer}: subset entries={subset_counts[0]} TODO-OWNER={subset_counts[1]} "
          f"UNCLASSIFIED={subset_counts[2]} purpose_fragments={subset_counts[3]}")
    print(f"table: {_counts_line(entries)}")
    for item in problems:
        print(f"  INFO: {item}")
    if subset_counts[1] or subset_counts[3]:
        print("FAIL: foundation 子集要求 TODO-OWNER=0 且 purpose_fragments=0", file=sys.stderr)
        return 1
    return 0


def cmd_fill_owners() -> int:
    """R3b：按 claims 证据填 claims 覆盖开关的 owner/purpose（不改 level/status）。"""
    entries = registry_entries()
    names = {str(e.get("name")) for e in entries}
    claims = load_claims()
    wanted: dict[str, tuple[str, str]] = {}
    for claim in claims:
        switch = claim.get("switch_env")
        if not switch:
            continue
        owner = str(claim.get("owner") or "TODO-OWNER")
        purpose = f"{_clean_purpose(str(claim.get('title', '')))}（claims: {claim.get('id')}）"
        wanted.setdefault(str(switch), (owner, purpose))
    for name, (owner, purpose) in RELATED_OWNERS.items():
        if name in names:
            wanted.setdefault(name, (owner, purpose))
    missing = sorted(set(wanted) - names)
    if missing:
        print(f"FAIL: claims 开关未在登记表: {missing}", file=sys.stderr)
        return 1
    before = _registry_counts(entries)
    changed = 0
    for entry in entries:
        name = str(entry.get("name"))
        if name not in wanted:
            continue
        owner, purpose = wanted[name]
        if entry.get("owner") != owner or entry.get("purpose") != purpose:
            changed += 1
        entry["owner"] = owner
        entry["purpose"] = purpose
    note = (f"R3b 第一批: claims 覆盖开关 owner/purpose 已填（{changed} 条变更，来源 claims 证据）；"
            "level 仍 UNCLASSIFIED（ADR-0004 §7.1：登记先行占位，四级口径由 claims/wiring 承担）")
    REGISTRY.write_text(render_registry(entries, note), encoding="utf-8")
    after = _registry_counts(entries)
    print(f"fill-owners: changed={changed}  entries={after[0]}  "
          f"TODO-OWNER {before[1]} -> {after[1]}  UNCLASSIFIED {before[2]} -> {after[2]}")
    return 0


def cmd_crosscheck(strict: bool) -> int:
    """R3c：wiring ledger 与 claims 交叉校验（PRODUCTION WIRED 有生产引用；IMPLEMENTED 有 gaps 原因）。"""
    claims = load_claims()
    refs = env_refs_committed()
    problems: list[str] = []
    for claim in claims:
        cid = str(claim.get("id", ""))
        level = str(claim.get("level", ""))
        switch = claim.get("switch_env")
        sources = [str(s) for s in (claim.get("sources") or [])]
        wired = {s for s in sources if is_production_source(s)}
        if switch:
            wired.update(refs.get(str(switch), []))
        notes = str(claim.get("notes") or "").strip()
        if level == "PRODUCTION WIRED" and not wired:
            problems.append(f"{cid}: PRODUCTION WIRED 但无生产引用")
        if level in ("IMPLEMENTED", "NOT IMPLEMENTED") and not notes:
            problems.append(f"{cid}: {level} 但无 gaps 原因（notes 为空）")
        if not claim.get("tests") and "无自动测试" not in notes and level == "PRODUCTION WIRED":
            problems.append(f"{cid}: PRODUCTION WIRED 但 tests 为空且 notes 未说明")
    print(f"crosscheck: claims={len(claims)} problems={len(problems)} (mode={'strict' if strict else 'report-only'})")
    for item in problems:
        print(f"  VIOLATION: {item}")
    if problems and strict:
        return 1
    return 0


def cmd_stats() -> int:
    committed = scan_env(committed=True)
    working = scan_env(committed=False)
    entries = registry_entries()
    claims = load_claims()
    print(f"env names: committed={len(committed)} working_tree={len(working)} "
          f"in_flight={len(set(working) - set(committed))}")
    print(f"registry: {_counts_line(entries)}")
    print(f"wiring: claims={len(claims)}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="R3a env registry + wiring ledger (ADR-0004)")
    parser.add_argument("--seed-env-registry", action="store_true")
    parser.add_argument("--check-env-registry", action="store_true")
    parser.add_argument("--gen", action="store_true")
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--fill-owners-from-claims", action="store_true",
                        help="R3b：按 claims 证据填 claims 覆盖开关的 owner/purpose")
    parser.add_argument("--fill-layer", default="", help="R3b：按层填实 owner/evidence/purpose/default")
    parser.add_argument("--crosscheck", action="store_true",
                        help="R3c：wiring ↔ claims 交叉校验（默认 report-only；--strict 升 blocking）")
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--stats", action="store_true")
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args(argv)
    if args.seed_env_registry:
        return cmd_seed(args.force)
    if args.check_env_registry:
        return cmd_check_env_registry()
    if args.fill_owners_from_claims:
        return cmd_fill_owners()
    if args.fill_layer:
        return cmd_fill_layer(args.fill_layer)
    if args.gen:
        return cmd_gen()
    if args.verify:
        return cmd_verify()
    if args.crosscheck:
        return cmd_crosscheck(args.strict)
    if args.stats:
        return cmd_stats()
    parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
