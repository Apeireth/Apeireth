#!/usr/bin/env python3
"""架构守卫（Architecture Guard）— 读取架构契约并执行 fitness functions。

唯一真源: docs/architecture/contract.toml
棘轮基线: scripts/arch-baseline.txt（只减不增）
例外登记: docs/architecture/exceptions.toml（带权重/到期）

用法:
  python scripts/arch_guard.py                    # report: 报告现状/违规/可缩表项, exit 0
  python scripts/arch_guard.py --strict           # 拦截: 新增违规/例外到期/超预算 -> exit 1
  python scripts/arch_guard.py --update-baseline  # 评审后初始化/缩表（谨慎）
  python scripts/arch_guard.py --wave 2           # 指定架构波次
  python scripts/arch_guard.py --json             # 机器可读输出

退出码: 0 = 通过/仅报告; 1 = strict 下有违规; 2 = fatal（契约/metadata 不可解析）

0 装口径: 只做本文件 §6 声明已覆盖的检查；未覆盖项如实标注在 docs/architecture/README.md §6。
依赖: Python >= 3.11（tomllib）、cargo（仅 metadata）。
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    sys.exit("arch_guard: Python >= 3.11 required (tomllib)")

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CONTRACT = ROOT / "docs" / "architecture" / "contract.toml"
DEFAULT_EXCEPTIONS = ROOT / "docs" / "architecture" / "exceptions.toml"
DEFAULT_BASELINE = ROOT / "scripts" / "arch-baseline.txt"
DEFAULT_LOG = ROOT / "logs" / "arch-guard.log"

MIN_SIZE_TRACK = 1200  # 基线只追踪 >= 该行数的 src 文件（棘轮对象）


# --------------------------------------------------------------------- utils

def _read_toml(path: Path) -> dict:
    try:
        with open(path, "rb") as fh:
            return tomllib.load(fh)
    except Exception as exc:  # noqa: BLE001 - fatal 需要原样报因
        raise SystemExit(f"arch_guard: cannot parse {path}: {exc}")


def _run(cmd: list[str]) -> tuple[int, str]:
    proc = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                          encoding="utf-8", errors="replace")
    return proc.returncode, (proc.stdout or "") + (proc.stderr or "")


def _norm(path: str) -> str:
    return path.replace("\\", "/")


# ------------------------------------------------------------------- contract

class Contract:
    def __init__(self, data: dict, path: Path):
        self.data = data
        self.path = path
        self.meta = data.get("meta", {})
        self.layers: dict[str, list[str]] = data.get("layers", {})
        self.crate_layer: dict[str, str] = {}
        for layer, crates in self.layers.items():
            for crate in crates:
                self.crate_layer[crate] = layer
        self.deps = data.get("deps", {})
        self.budgets = data.get("budgets", {})
        self.env = data.get("env", {})
        self.exceptions_cfg = data.get("exceptions", {})
        self.concepts = data.get("concepts", [])
        self.ext_points = data.get("extension_points", {})
        self.flagged_edges = [(e["from"], e["to"]) for e in self.deps.get("flagged_edges", [])]
        self.composition_roots = data.get("composition_roots", [])

    @property
    def current_wave(self) -> int:
        return int(self.meta.get("current_wave", 1))

    def allowed_layer_pair(self, src: str, dst: str, layer_of: dict[str, str]) -> bool:
        a = layer_of.get(src)
        b = layer_of.get(dst)
        if a is None or b is None:
            return False
        # 尾部的 bench->* 规则
        for rule in self.deps.get("allowed_layer_edges", []):
            if "->" not in rule:
                continue
            left, right = rule.split("->", 1)
            left, right = left.strip(), right.strip()
            if left != a:
                continue
            if right == "*" or right == b or right in b.split(","):
                return True
        return False


def load_contract(path: Path) -> Contract:
    c = Contract(_read_toml(path), path)
    if not c.layers:
        raise SystemExit(f"arch_guard: {path} missing [layers]")
    if not c.deps.get("allowed_layer_edges"):
        raise SystemExit(f"arch_guard: {path} missing [deps].allowed_layer_edges")
    return c


# ------------------------------------------------------------------- metadata

def workspace_metadata() -> tuple[dict[str, Path], dict[str, set[str]], dict[str, set[str]]]:
    """返回 (package->crate_root, prod_deps, all_deps)。仅读 metadata，不构建。"""
    code, out = _run(["cargo", "metadata", "--locked", "--no-deps", "--offline",
                      "--format-version", "1"])
    if code != 0:
        raise SystemExit(f"arch_guard: cargo metadata failed:\n{out[-2000:]}")
    data = json.loads(out)
    names = {p["name"] for p in data["packages"]}
    roots: dict[str, Path] = {}
    prod: dict[str, set[str]] = {}
    all_deps: dict[str, set[str]] = {}
    for pkg in data["packages"]:
        name = pkg["name"]
        roots[name] = Path(pkg["manifest_path"]).parent
        prod.setdefault(name, set())
        all_deps.setdefault(name, set())
        for dep in pkg.get("dependencies", []):
            dep_name = dep.get("name")
            if dep_name not in names:
                continue
            all_deps[name].add(dep_name)
            kind = dep.get("kind")
            if kind != "dev":  # kind=None(normal) / "build" 计入生产图
                prod[name].add(dep_name)
    return roots, prod, all_deps


# ------------------------------------------------------------------ scanners

ENV_RE = re.compile(r"APEIRETH_[A-Z0-9_]+")
DEF_RE = re.compile(r"^\s*pub(?:\([^)]*\))?\s+(?:struct|trait|enum|type)\s+([A-Za-z_][A-Za-z0-9_]*)\b",
                    re.MULTILINE)


def _crate_src_files(roots: dict[str, Path], crates: set[str]) -> list[tuple[str, Path]]:
    files: list[tuple[str, Path]] = []
    for crate in crates:
        root = roots.get(crate)
        if root is None:
            continue
        src = root / "src"
        if not src.exists():
            continue
        for path in src.rglob("*.rs"):
            files.append((crate, path))
    return files


def scan_env_and_defs(roots: dict[str, Path], crates: set[str]) -> tuple[set[str], dict[str, list[str]]]:
    env_names: set[str] = set()
    defs: dict[str, list[str]] = {}
    for _crate, path in _crate_src_files(roots, crates):
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        env_names.update(ENV_RE.findall(text))
        for match in DEF_RE.finditer(text):
            defs.setdefault(match.group(1), []).append(_norm(str(path.relative_to(ROOT))))
    return env_names, defs


def scan_sizes(roots: dict[str, Path], crates: set[str]) -> dict[str, int]:
    sizes: dict[str, int] = {}
    for _crate, path in _crate_src_files(roots, crates):
        try:
            loc = len(path.read_text(encoding="utf-8", errors="replace").splitlines())
        except OSError:
            continue
        rel = _norm(str(path.relative_to(ROOT)))
        sizes[rel] = loc
    return sizes


# --------------------------------------------------------------------- cycles

def sccs(nodes: list[str], edges: dict[str, set[str]]) -> list[list[str]]:
    """Tarjan SCC（迭代实现），返回所有 SCC（含单点）。"""
    index: dict[str, int] = {}
    low: dict[str, int] = {}
    on_stack: set[str] = set()
    stack: list[str] = []
    result: list[list[str]] = []
    counter = [0]

    for start in nodes:
        if start in index:
            continue
        work = [(start, iter(sorted(edges.get(start, set()))))]
        index[start] = low[start] = counter[0]
        counter[0] += 1
        stack.append(start)
        on_stack.add(start)
        while work:
            node, it = work[-1]
            advanced = False
            for nxt in it:
                if nxt not in index:
                    index[nxt] = low[nxt] = counter[0]
                    counter[0] += 1
                    stack.append(nxt)
                    on_stack.add(nxt)
                    work.append((nxt, iter(sorted(edges.get(nxt, set())))))
                    advanced = True
                    break
                if nxt in on_stack:
                    low[node] = min(low[node], index[nxt])
            if advanced:
                continue
            work.pop()
            if work:
                parent = work[-1][0]
                low[parent] = min(low[parent], low[node])
            if low[node] == index[node]:
                comp: list[str] = []
                while True:
                    cur = stack.pop()
                    on_stack.discard(cur)
                    comp.append(cur)
                    if cur == node:
                        break
                result.append(sorted(comp))
    return result


def cycle_fingerprints(edges: dict[str, set[str]]) -> set[str]:
    nodes = sorted(edges)
    fps: set[str] = set()
    for comp in sccs(nodes, edges):
        if len(comp) > 1:
            fps.add(",".join(sorted(comp)))
    return fps


# ------------------------------------------------------------------- baseline

def load_baseline(path: Path) -> set[str]:
    if not path.exists():
        return set()
    entries: set[str] = set()
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        entries.add(line)
    return entries


def write_baseline(path: Path, entries: set[str]) -> None:
    groups: dict[str, list[str]] = {}
    for entry in entries:
        groups.setdefault(entry.split(" ", 1)[0], []).append(entry)
    order = ["edge", "devcycle", "cycle", "env", "size", "concept", "ext"]
    lines = [
        "# arch-guard baseline v1 — 架构违规棘轮基线（只减不增）",
        "# 生成: python scripts/arch_guard.py --update-baseline",
        "# 规则: 新指纹不在本文件 = 违规; 本文件中已消失的指纹 = 可缩表。",
        "",
    ]
    for group in order:
        rows = sorted(groups.get(group, []))
        if not rows:
            continue
        lines.append(f"# --- {group} ({len(rows)}) ---")
        lines.extend(rows)
        lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


# ----------------------------------------------------------------- exceptions

EXC_REQUIRED = ("id", "fingerprint", "weight", "owner", "reason",
                "created_wave", "expires_wave", "removal_task")


def load_exceptions(path: Path) -> list[dict]:
    if not path.exists():
        return []
    data = _read_toml(path)
    items = data.get("exceptions", [])
    if not isinstance(items, list):
        raise SystemExit(f"arch_guard: {path} [exceptions] must be a list")
    for item in items:
        missing = [k for k in EXC_REQUIRED if k not in item]
        if missing:
            raise SystemExit(f"arch_guard: exception {item.get('id', '?')} missing {missing}")
    return items


# ------------------------------------------------------------------- checks

class Report:
    def __init__(self) -> None:
        self.violations: list[dict] = []
        self.stale: list[str] = []
        self.notes: list[str] = []
        self.metrics: dict[str, object] = {}

    def violation(self, weight: int, category: str, fingerprint: str, detail: str) -> None:
        self.violations.append(
            {"weight": weight, "category": category, "fingerprint": fingerprint, "detail": detail}
        )

    def add_stale(self, fingerprint: str) -> None:
        self.stale.append(fingerprint)

    def note(self, text: str) -> None:
        self.notes.append(text)


def check(contract: Contract, baseline: set[str], wave: int, dry_metrics: bool = False) -> Report:
    rep = Report()
    roots, prod, all_deps = workspace_metadata()
    workspace_crates = set(prod)
    layer_of = {crate: contract.crate_layer.get(crate) for crate in workspace_crates}
    rep.metrics["workspace_crates"] = len(workspace_crates)

    # 1. 层登记完整性
    for crate in sorted(workspace_crates):
        if layer_of[crate] is None:
            rep.violation(10, "structural", f"layer {crate}",
                          f"workspace crate {crate} 未登记在 contract [layers]")
    max_crates = int(contract.budgets.get("max_workspace_crates", 10**9))
    if len(workspace_crates) > max_crates:
        rep.violation(1, "size", f"crates {len(workspace_crates)}",
                      f"workspace members {len(workspace_crates)} > budget {max_crates}")

    # 2. 依赖边 + 方向
    edge_count = 0
    layer_allowed_fn = contract.allowed_layer_pair
    for src in sorted(prod):
        for dst in sorted(prod[src]):
            edge_count += 1
            fp = f"edge {src} -> {dst}"
            if fp in baseline:
                continue
            if not layer_allowed_fn(src, dst, layer_of):
                rep.violation(10, "direction", fp,
                              f"跨层方向违规/未登记层: {layer_of.get(src)} -> {layer_of.get(dst)}")
            else:
                rep.violation(3, "edge", fp, "新增依赖边（层规则允许但未入基线；需 ADR 或登记例外）")
    rep.metrics["production_edges"] = edge_count
    max_edges = int(contract.budgets.get("max_production_edges", 10**9))
    if edge_count > max_edges:
        rep.violation(1, "size", f"edges {edge_count}",
                      f"生产依赖边 {edge_count} > budget {max_edges}")
    # 基线中已消失的边 → 可缩表
    for entry in sorted(baseline):
        if entry.startswith("edge ") and entry.count(" -> ") == 1:
            _, rest = entry.split(" ", 1)
            src, dst = [x.strip() for x in rest.split("->")]
            if dst not in prod.get(src, set()):
                rep.add_stale(entry)
    # flagged 边消失提示
    for src, dst in contract.flagged_edges:
        if dst not in prod.get(src, set()):
            rep.note(f"flagged edge {src} -> {dst} 已不存在（可更新契约 flagged_edges）")

    # 3. 循环
    prod_cycles = cycle_fingerprints(prod)
    dev_cycles = cycle_fingerprints(all_deps) - prod_cycles
    for fp in sorted(prod_cycles):
        if f"cycle {fp}" not in baseline:
            rep.violation(10, "direction", f"cycle {fp}", "新增生产依赖环")
    for fp in sorted(dev_cycles):
        if f"devcycle {fp}" not in baseline:
            rep.violation(10, "direction", f"devcycle {fp}", "新增 dev 依赖环（dev 也不允许成环）")
    for entry in sorted(baseline):
        if entry.startswith("cycle ") and entry[6:] not in prod_cycles:
            rep.add_stale(entry)
        if entry.startswith("devcycle ") and entry[9:] not in dev_cycles:
            rep.add_stale(entry)

    # 4. env 开关（基线 + 登记表双通道；登记表由 R3a 建立，见 ADR-0004）
    env_names, defs = scan_env_and_defs(roots, workspace_crates)
    rep.metrics["env_names"] = len(env_names)
    registered: set[str] = set()
    reg_rel = str(contract.env.get("registry_file", "")).strip()
    if reg_rel:
        reg_path = Path(reg_rel)
        if not reg_path.is_absolute():
            reg_path = ROOT / reg_path
        if reg_path.exists():
            reg_data = _read_toml(reg_path)
            for entry in reg_data.get("entries", []):
                if isinstance(entry, dict) and entry.get("name"):
                    registered.add(str(entry["name"]))
    rep.metrics["env_registered"] = len(registered)
    for name in sorted(env_names):
        if f"env {name}" in baseline or name in registered:
            continue
        rep.violation(3, "env", f"env {name}", "新增 APEIRETH_* 开关未登记（需默认关+零变化测试+登记）")
    for entry in sorted(baseline):
        if entry.startswith("env ") and entry[4:] not in env_names:
            rep.add_stale(entry)
    max_env = int(contract.env.get("max_distinct_names", 10**9))
    if len(env_names) > max_env:
        rep.violation(1, "size", f"env-count {len(env_names)}",
                      f"开关名 {len(env_names)} > budget {max_env}")

    # 5. 文件尺寸
    sizes = scan_sizes(roots, workspace_crates)
    max_file = int(contract.budgets.get("max_file_loc", 10**9))
    for rel, loc in sorted(sizes.items()):
        if loc < MIN_SIZE_TRACK and f"size {rel}" not in baseline:
            continue
        fp = f"size {rel} {loc}"
        if fp in baseline:
            continue
        old = None
        for entry in baseline:
            if entry.startswith(f"size {rel} "):
                try:
                    old = int(entry.rsplit(" ", 1)[1])
                except ValueError:
                    old = None
                break
        if old is not None and loc > old:
            rep.violation(1, "size", f"size-grow {rel}",
                          f"{rel} {old} -> {loc} 行（尺寸增长）")
        elif old is not None and loc < old:
            rep.add_stale(f"size {rel} {old}")  # 可缩表（缩到新值）
        elif loc > max_file:
            rep.violation(1, "size", f"size-new {rel}",
                          f"新增超大文件 {rel} = {loc} 行 > budget {max_file}")
    for entry in sorted(baseline):
        if entry.startswith("size "):
            parts = entry.split(" ")
            if len(parts) == 3 and parts[1] not in sizes:
                rep.add_stale(entry)

    # 6. 同名概念
    for concept in contract.concepts:
        name = concept["name"]
        canonical = _norm(concept.get("canonical", ""))
        known = [_norm(p) for p in concept.get("known_debt", [])]
        found = defs.get(name, [])
        if canonical not in found:
            rep.violation(10, "structural", f"concept-canonical {name} {canonical}",
                          f"契约 canonical 路径无 {name} 定义（契约漂移）")
        for path in found:
            if path in (canonical, *known):
                continue
            if f"concept {name} {path}" not in baseline:
                rep.violation(3, "duplicate", f"duplicate {name} {path}",
                              f"同名概念 {name} 出现新定义 {path}（canonical={canonical}）")
        for path in known:
            if path not in found:
                rep.add_stale(f"concept {name} {path}")

    # 7. 扩展点定义唯一性
    for key, ref in contract.ext_points.items():
        match = re.search(r"::([A-Za-z_][A-Za-z0-9_]*)", ref)
        if not match:
            continue
        name = match.group(1)
        for path in defs.get(name, []):
            if f"ext {name} {path}" not in baseline:
                rep.violation(10, "structural", f"ext {name} {path}",
                              f"扩展点 {key} ({name}) 出现新定义 {path}（结构变更需 ADR）")

    # 8. 例外登记
    exc_cfg = dict(contract.exceptions_cfg)
    exc_path = Path(exc_cfg.get("register_file", "docs/architecture/exceptions.toml"))
    if not exc_path.is_absolute():
        exc_path = ROOT / exc_path
    exceptions = load_exceptions(exc_path)
    active = [
        e for e in exceptions
        if int(e["expires_wave"]) >= wave
    ]
    for exc in exceptions:
        if int(exc["expires_wave"]) < wave:
            rep.violation(10, "exception", f"exception-expired {exc['id']}",
                          f"例外 {exc['id']} 已到期（expires_wave={exc['expires_wave']}, wave={wave}）")
        if int(exc["weight"]) >= int(contract.exceptions_cfg.get("strict_requires_adr_above_weight", 10)):
            adr = str(exc.get("adr", "")).strip()
            if not adr or not (ROOT / adr).exists():
                rep.violation(10, "exception", f"exception-noadr {exc['id']}",
                              f"高权重例外 {exc['id']} 必须引用存在的 ADR 文件")
    per_wave = int(contract.exceptions_cfg.get("per_wave_weight_budget", 0))
    wave_weight = sum(int(e["weight"]) for e in exceptions if int(e["created_wave"]) == wave)
    if wave_weight > per_wave:
        rep.violation(10, "exception", f"exception-budget {wave_weight}",
                      f"本波例外权重 {wave_weight} > budget {per_wave}")
    max_active = int(contract.exceptions_cfg.get("active_exceptions_max", 0))
    if len(active) > max_active:
        rep.violation(10, "exception", f"exception-active {len(active)}",
                      f"活跃例外 {len(active)} > max {max_active}")
    if contract.exceptions_cfg.get("release_freeze") and exceptions:
        rep.violation(10, "exception", "exception-freeze",
                      "发版冻结窗口不允许任何活跃例外")
    rep.metrics["exceptions_active"] = len(active)

    # 8.5 例外抑制: 已登记、未到期且指纹匹配的违规从结果扣除（审计记录在 notes）。
    #     只有可债务化的类别可被抑制；exception-* / concept-canonical / ext / cycle 不允许。
    suppressible = {"edge", "direction", "env", "size", "duplicate"}
    active_by_fp = {e["fingerprint"]: e for e in exceptions if int(e["expires_wave"]) >= wave}
    if active_by_fp:
        kept: list[dict] = []
        for v in rep.violations:
            exc = active_by_fp.get(v["fingerprint"])
            if exc is not None and v["category"] in suppressible:
                rep.note(f"例外抑制: {v['fingerprint']} <- {exc['id']} "
                         f"(owner={exc['owner']}, expires_wave={exc['expires_wave']})")
            else:
                kept.append(v)
        rep.violations = kept

    # 9. 装配根 / flagged / locked 文件存在性
    for root in contract.composition_roots:
        path = ROOT / "crates"
        found = list(path.glob(f"**/{root['file']}"))
        if not found:
            rep.note(f"composition root 文件未找到: {root['crate']} {root['file']}")
    for key, rel in contract.data.get("locked", {}).items():
        if rel and not rel.startswith("legacy") and "(只引用" not in rel:
            if not (ROOT / rel).exists():
                rep.note(f"locked 引用路径不存在（历史口径）: {key}={rel}")

    rep.metrics["max_src_file_loc"] = max(sizes.values()) if sizes else 0
    return rep


# --------------------------------------------------------------------- main

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Apeireth architecture guard")
    parser.add_argument("--contract", default=str(DEFAULT_CONTRACT))
    parser.add_argument("--exceptions", default=None)
    parser.add_argument("--baseline", default=str(DEFAULT_BASELINE))
    parser.add_argument("--log", default=str(DEFAULT_LOG))
    parser.add_argument("--wave", type=int, default=None)
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--report", action="store_true")
    parser.add_argument("--update-baseline", action="store_true")
    parser.add_argument("--print-vendored", action="store_true",
                        help="输出契约 [vendored].image_family（供 gate 脚本生成 --exclude）")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    contract = load_contract(Path(args.contract))
    if args.print_vendored:
        for name in contract.data.get("vendored", {}).get("image_family", []):
            print(name)
        return 0
    if args.exceptions:
        contract.exceptions_cfg["register_file"] = args.exceptions
    baseline_path = Path(args.baseline)
    baseline = load_baseline(baseline_path)
    wave = args.wave if args.wave is not None else contract.current_wave

    rep = check(contract, baseline, wave)

    if args.update_baseline:
        roots, prod, all_deps = workspace_metadata()
        crates = set(prod)
        env_names, defs = scan_env_and_defs(roots, crates)
        sizes = scan_sizes(roots, crates)
        entries: set[str] = set()
        for src in prod:
            for dst in prod[src]:
                entries.add(f"edge {src} -> {dst}")
        for fp in cycle_fingerprints(prod):
            entries.add(f"cycle {fp}")
        prod_cycles = cycle_fingerprints(prod)
        for fp in cycle_fingerprints(all_deps) - prod_cycles:
            entries.add(f"devcycle {fp}")
        for name in env_names:
            entries.add(f"env {name}")
        for rel, loc in sizes.items():
            if loc >= MIN_SIZE_TRACK:
                entries.add(f"size {rel} {loc}")
        for concept in contract.concepts:
            for path in defs.get(concept["name"], []):
                entries.add(f"concept {concept['name']} {path}")
        for key, ref in contract.ext_points.items():
            match = re.search(r"::([A-Za-z_][A-Za-z0-9_]*)", ref)
            if match:
                for path in defs.get(match.group(1), []):
                    entries.add(f"ext {match.group(1)} {path}")
        write_baseline(baseline_path, entries)
        print(f"[arch_guard] baseline written: {baseline_path} ({len(entries)} entries)")
        return 0

    # 输出
    lines: list[str] = []
    lines.append(f"[arch_guard] wave={wave} contract={contract.meta.get('version')} "
                 f"status={contract.meta.get('status')}")
    lines.append(f"[arch_guard] crates={rep.metrics.get('workspace_crates')} "
                 f"edges={rep.metrics.get('production_edges')} "
                 f"env={rep.metrics.get('env_names')} "
                 f"max_src_loc={rep.metrics.get('max_src_file_loc')} "
                 f"exceptions={rep.metrics.get('exceptions_active')}")
    if rep.violations:
        lines.append(f"[arch_guard] VIOLATIONS: {len(rep.violations)}")
        for v in sorted(rep.violations, key=lambda x: (-x["weight"], x["category"])):
            lines.append(f"  [W{v['weight']}] {v['category']}: {v['fingerprint']} — {v['detail']}")
    else:
        lines.append("[arch_guard] VIOLATIONS: 0")
    if rep.stale:
        lines.append(f"[arch_guard] STALE (可缩表): {len(rep.stale)}")
        for entry in rep.stale[:50]:
            lines.append(f"  - {entry}")
        if len(rep.stale) > 50:
            lines.append(f"  ... ({len(rep.stale) - 50} more)")
    for note in rep.notes:
        lines.append(f"[arch_guard] NOTE: {note}")

    out = "\n".join(lines)
    log_path = Path(args.log)
    try:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        log_path.write_text(out + "\n", encoding="utf-8")
    except OSError:
        pass

    if args.json:
        print(json.dumps({
            "wave": wave,
            "status": contract.meta.get("status"),
            "metrics": rep.metrics,
            "violations": rep.violations,
            "stale": rep.stale,
            "notes": rep.notes,
        }, ensure_ascii=False, indent=2))
    else:
        print(out)

    if args.strict and rep.violations:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
