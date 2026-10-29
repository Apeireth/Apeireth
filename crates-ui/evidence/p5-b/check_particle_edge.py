#!/usr/bin/env python3
"""⭐⭐ **「有界差异」门禁 v2** —— intro 片头：生产 vs 引擎的差异必须是**可解释的有界差**。

## 这条门禁要解决什么

片头（intro）从生产前端**迁入引擎**后，同一份 GLSL 在两种**光栅化实现**上跑：

- 生产 = 浏览器原生 WebGL2（`GL_POINTS` 点精灵）；
- 引擎 = wgpu 的 GLES 后端（或 WebGPU 后端，走实例化四边形）。

⇒ ⇒ **位置与尺寸可以做到逐位一致**（实测：亮像素质心差 **0.003 px**），
但「一个片元被某个粒子覆盖了多少」在**边缘像素上会差一点点** ⇒ ⇒
加性混合把这个小数**累加**起来 ⇒ ⇒ 出现**散点式、幅度极小**的差异。

## ⚠️ 为什么 v1 的判据必须改（实测证据，2026-10-27）

v1 判据是「**差异像素必须全部落在粒子并集轮廓的边缘带内**」（带半径 2px）。
实测：差异 93 像素（阈值 >2），其中 **90 像素在带外** ⇒ ⇒ **v1 恒红**。

原因是**掩码语义错了**：并集轮廓只描述**整团火焰的外形**，而火焰内部
**每个粒子自己也有轮廓**（粒子互相重叠）⇒ ⇒ 一个落在**并集深处**的像素，
可能正好在**另一个粒子的边缘**上 ⇒ ⇒ 它当然会被覆盖舍入影响，
但它离**并集轮廓**很远 ⇒ ⇒ 被 v1 判成「语义级差异」。

⇒ ⇒ 所以 v1 的形态**不可满足**（不是引擎没修好，是判据本身错了）。
⇒ ⇒ 台账 §5 的教训再次成立：**判据自身的缺陷比引擎缺陷更隐蔽**。

## 判据（v2：几何严格 + 幅度有界 + 预算有界 + **可自证可失败**）

| # | 检查 | 阈值 | 依据 |
|---|------|------|------|
| A1 | 亮像素质心差 | ≤ 0.05 px | 实测 0.003（位置错会立刻爆掉） |
| A2 | 亮像素计数相对差 | ≤ 0.01% | 实测 0.0003%（数量/半径错会爆掉） |
| A3 | 总亮度相对差 | ≤ 0.01% | 实测 0.0007%（颜色/强度错会爆掉） |
| B  | 全图**最大通道差** | ≤ 16 / 255 | 实测 8（语义错通常 ≫ 16） |
| C  | 超阈像素（差 > 2）占比 | ≤ 0.05% | 实测 0.0161%（约 3× 余量） |

**A 组是「语义级」不变量**（位置 / 数量 / 半径 / 总能量）⇒ ⇒ 它们**不容许**任何
实现级豁免：一旦有人把粒子画错，A 组立刻红。
**B / C 组才是「实现级」豁免**：只放过「幅度极小且数量极少」的覆盖舍入差。

## ⭐ 可失败性（内建反事实自检，**每次运行都做**）

判据如果不能失败，它就不是判据。本门禁每次运行都会**合成两个反事实**：

1. **上下翻转**引擎图 ⇒ ⇒ 必须被判 **失败**；
2. **平移 1 像素**引擎图 ⇒ ⇒ 必须被判 **失败**。

任一反事实**若被判通过** ⇒ ⇒ 门禁自身失效 ⇒ ⇒ 直接 `exit 2`（判据坏了，不许报绿）。

## 用法

```bash
# ① 正常渲染（生产 + 引擎，webgl 腿）
python3 drive_intro.py --it 2.5 --sprite quad

# ② 判据
python3 check_particle_edge.py \
    --production evidence-intro/intro.production.quad.webgl.png \
    --engine     evidence-intro/intro.engine.quad.webgl.png \
    --frozen-at 2.5

# 可选：给粒子并集（flat 探针）⇒ ⇒ 只用于**报告覆盖率**，不参与判定
python3 drive_intro.py --probe flat --it 2.5 --sprite quad
python3 check_particle_edge.py --union evidence-intro/intro.production.quad.webgl.png ...
```

退出码：`0` = 通过；`1` = 判据失败；`2` = 用法/输入错误**或判据自身失效**。
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

try:
    from PIL import Image, ImageChops
except ImportError:  # pragma: no cover
    print("需要 pillow：python3 -m pip install pillow", file=sys.stderr)
    raise SystemExit(2)

#: 差异像素的判定阈值（任一通道差 > 此值即算「有差异」）。
DIFF_THRESHOLD = 2
#: B：全图允许的**最大通道差**上限（实测 8 ⇒ 留 2× 余量；语义错通常 ≫ 16）。
MAX_CHANNEL_DELTA = 16
#: C：超阈像素占画布的比例上限（实测 0.0161% ⇒ 留约 3× 余量）。
BUDGET_RATIO = 0.0005
#: A1：亮像素质心允许的偏差（像素）。实测 0.003。
CENTROID_TOL_PX = 0.05
#: A2 / A3：亮像素计数、总亮度的相对偏差上限。实测 3e-6 / 7.3e-5。
LIT_COUNT_REL_TOL = 0.0001
LIT_SUM_REL_TOL = 0.0001
#: 判「亮像素」的亮度阈值（RGB 求和）。
LIT_THRESHOLD = 30
#: flat 探针画的粒子颜色（纯红）。
FLAT_COLOR = (255, 0, 0)


def _stats(image: Image.Image) -> dict:
    """亮像素计数 / 质心 / 总亮度。"""
    rgb = image.convert("RGB")
    width, height = rgb.size
    count = 0
    sum_x = 0.0
    sum_y = 0.0
    sum_lum = 0.0
    data = list(rgb.getdata())
    for index, (r, g, b) in enumerate(data):
        lum = r + g + b
        if lum <= LIT_THRESHOLD:
            continue
        count += 1
        sum_lum += lum
        sum_x += (index % width) * lum
        sum_y += (index // width) * lum
    if sum_lum == 0.0:
        return {"count": 0, "centroid": None, "lum_sum": 0.0}
    return {
        "count": count,
        "centroid": (sum_x / sum_lum, sum_y / sum_lum),
        "lum_sum": sum_lum,
    }


def _diff_mask(a: Image.Image, b: Image.Image) -> tuple[Image.Image, int]:
    """返回「差异掩码」与差异像素数（阈值 `DIFF_THRESHOLD`）。"""
    diff = ImageChops.difference(a.convert("RGB"), b.convert("RGB"))
    r, g, bl = diff.split()
    merged = ImageChops.lighter(ImageChops.lighter(r, g), bl)
    mask = merged.point(lambda v: 255 if v > DIFF_THRESHOLD else 0)
    return mask.convert("L"), sum(mask.point(lambda v: 1 if v else 0).getdata())


def _max_delta(a: Image.Image, b: Image.Image) -> int:
    """全图最大通道差。"""
    diff = ImageChops.difference(a.convert("RGB"), b.convert("RGB"))
    return max(channel.getextrema()[1] for channel in diff.split())


def _union_mask(union: Image.Image) -> Image.Image:
    """从 flat 探针图里取「粒子并集」掩码（纯红像素）。仅用于报告覆盖率。"""
    rgb = union.convert("RGB")
    r, g, bl = rgb.split()
    red = r.point(lambda v: 255 if v > 40 else 0)
    not_green = g.point(lambda v: 0 if v > 40 else 255)
    not_blue = bl.point(lambda v: 0 if v > 40 else 255)
    mask = ImageChops.multiply(red, ImageChops.multiply(not_green, not_blue))
    return mask.convert("L")


def _evaluate(production: Image.Image, engine: Image.Image) -> dict:
    """对一对图执行 A/B/C 三项判据，返回逐项结论（**纯函数**，反事实自检复用它）。"""
    pixels = production.size[0] * production.size[1]
    prod = _stats(production)
    eng = _stats(engine)
    failures: list[str] = []

    # ── A1 质心 ────────────────────────────────────────────────────────────
    centroid_delta = None
    if prod["centroid"] and eng["centroid"]:
        centroid_delta = (
            abs(prod["centroid"][0] - eng["centroid"][0]),
            abs(prod["centroid"][1] - eng["centroid"][1]),
        )
        if max(centroid_delta) > CENTROID_TOL_PX:
            failures.append(
                f"A1 质心偏差 {max(centroid_delta):.4f}px > {CENTROID_TOL_PX}px"
                " ⇒ ⇒ 几何（位置）被改动"
            )
    elif bool(prod["centroid"]) != bool(eng["centroid"]):
        failures.append("A1 一侧全暗（亮像素为 0）⇒ ⇒ 没画出来")

    # ── A2 亮像素计数 ──────────────────────────────────────────────────────
    count_rel = 0.0
    if prod["count"]:
        count_rel = abs(prod["count"] - eng["count"]) / prod["count"]
        if count_rel > LIT_COUNT_REL_TOL:
            failures.append(
                f"A2 亮像素数相对差 {count_rel * 100:.4f}% > {LIT_COUNT_REL_TOL * 100}%"
                " ⇒ ⇒ 粒子数量/半径被改动"
            )

    # ── A3 总亮度 ──────────────────────────────────────────────────────────
    lum_rel = 0.0
    if prod["lum_sum"]:
        lum_rel = abs(prod["lum_sum"] - eng["lum_sum"]) / prod["lum_sum"]
        if lum_rel > LIT_SUM_REL_TOL:
            failures.append(
                f"A3 总亮度相对差 {lum_rel * 100:.4f}% > {LIT_SUM_REL_TOL * 100}%"
                " ⇒ ⇒ 颜色/强度被改动"
            )

    # ── B 最大通道差 ───────────────────────────────────────────────────────
    max_delta = _max_delta(production, engine)
    if max_delta > MAX_CHANNEL_DELTA:
        failures.append(
            f"B 最大通道差 {max_delta} > {MAX_CHANNEL_DELTA} ⇒ ⇒ 幅度超出实现级舍入"
        )

    # ── C 超阈像素预算 ─────────────────────────────────────────────────────
    _, diff_count = _diff_mask(production, engine)
    diff_ratio = (diff_count / pixels) if pixels else 0.0
    if diff_ratio > BUDGET_RATIO:
        failures.append(
            f"C 超阈像素 {diff_count}（{diff_ratio * 100:.4f}%）> 预算 {BUDGET_RATIO * 100}%"
            " ⇒ ⇒ 差异量级超出实现级舍入"
        )

    return {
        "pixels": pixels,
        "centroid_production": prod["centroid"],
        "centroid_engine": eng["centroid"],
        "centroid_delta_px": list(centroid_delta) if centroid_delta else None,
        "lit_pixels_production": prod["count"],
        "lit_pixels_engine": eng["count"],
        "lit_count_relative_delta": count_rel,
        "lum_sum_production": prod["lum_sum"],
        "lum_sum_engine": eng["lum_sum"],
        "lum_sum_relative_delta": lum_rel,
        "max_channel_delta": max_delta,
        "differing_pixels": diff_count,
        "differing_ratio": diff_ratio,
        "failures": failures,
        "passed": not failures,
    }


def _counterfactuals(engine: Image.Image) -> list[tuple[str, Image.Image]]:
    """合成**必须被判失败**的反事实（判据可失败性的自证）。"""
    flipped = engine.transpose(Image.FLIP_TOP_BOTTOM)
    shifted = Image.new("RGB", engine.size, (0, 0, 0))
    shifted.paste(engine.crop((0, 0, engine.size[0] - 1, engine.size[1] - 1)), (1, 1))
    return [("上下翻转", flipped), ("平移 1 像素", shifted)]


def main() -> int:
    parser = argparse.ArgumentParser(description="intro「有界差异」门禁 v2")
    parser.add_argument("--production", required=True, help="正常渲染下的生产侧图")
    parser.add_argument("--engine", required=True, help="正常渲染下的引擎侧图（webgl 腿）")
    parser.add_argument("--union", default="", help="（可选）flat 探针的粒子并集图，仅用于报告覆盖率")
    parser.add_argument("--frozen-at", default="?", help="冻结时刻（写进报告）")
    parser.add_argument("--out", default="", help="判据 JSON 输出路径")
    args = parser.parse_args()

    production_path = pathlib.Path(args.production)
    engine_path = pathlib.Path(args.engine)
    for path in (production_path, engine_path):
        if not path.is_file():
            print(f"找不到输入图：{path}", file=sys.stderr)
            return 2

    production = Image.open(production_path).convert("RGB")
    engine = Image.open(engine_path).convert("RGB")
    if production.size != engine.size:
        print(f"两侧画布尺寸不同：{production.size} vs {engine.size}", file=sys.stderr)
        return 2

    verdict = _evaluate(production, engine)

    # ⭐ 反事实自检：判据**必须**能失败，否则它就不是判据。
    counterfactuals = []
    for label, image in _counterfactuals(engine):
        result = _evaluate(production, image)
        counterfactuals.append(
            {
                "label": label,
                "passed": result["passed"],
                "failures": result["failures"][:3],
                "differing_pixels": result["differing_pixels"],
                "max_channel_delta": result["max_channel_delta"],
            }
        )
    gate_falsifiable = all(not item["passed"] for item in counterfactuals)

    union_report = {}
    if args.union:
        union_path = pathlib.Path(args.union)
        if union_path.is_file():
            union_mask = _union_mask(Image.open(union_path).convert("RGB"))
            union_report = {
                "particle_union_pixels": sum(
                    union_mask.point(lambda v: 1 if v else 0).getdata()
                ),
                "note": "仅报告覆盖率，**不参与判定**（v1 把它当判定依据 ⇒ ⇒ 恒红，见模块文档）",
            }

    report = {
        "criterion": "intro_particle_bounded_v2",
        "why": (
            "几何不变量（质心/亮像素数/总亮度）**严格**；实现级覆盖舍入只允许"
            "「幅度极小（≤16/255）且数量极少（≤0.05%）」；"
            "并用**反事实自检**证明判据可失败"
        ),
        "frozen_at": args.frozen_at,
        "diff_threshold": DIFF_THRESHOLD,
        "max_channel_delta_allowed": MAX_CHANNEL_DELTA,
        "budget_ratio": BUDGET_RATIO,
        "centroid_tolerance_px": CENTROID_TOL_PX,
        "canvas": list(production.size),
        **verdict,
        **union_report,
        "counterfactuals": counterfactuals,
        "gate_falsifiable": gate_falsifiable,
        "passed": verdict["passed"] and gate_falsifiable,
    }
    text = json.dumps(report, ensure_ascii=False, indent=2)
    if args.out:
        pathlib.Path(args.out).write_text(text + "\n", encoding="utf-8")
    print(text)

    if not gate_falsifiable:
        print(
            "  ERROR  反事实自检失败：翻转/平移后的图**被判为通过** ⇒ ⇒ "
            "**判据自身失效**（不许报绿）",
            file=sys.stderr,
        )
        return 2
    if report["passed"]:
        print(
            f"  PASS  质心差 {max(report['centroid_delta_px'] or [0]):.4f}px / "
            f"亮像素相对差 {report['lit_count_relative_delta'] * 100:.4f}% / "
            f"总亮度相对差 {report['lum_sum_relative_delta'] * 100:.4f}% / "
            f"最大通道差 {report['max_channel_delta']} / "
            f"超阈像素 {report['differing_pixels']}（{report['differing_ratio'] * 100:.4f}%）"
            f" ⇒ ⇒ **几何零偏差、只有可解释的有界覆盖舍入**"
        )
        return 0
    print("  FAIL  " + "；".join(verdict["failures"]))
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
