# V1200 ASI V0.6.10 鍙?dim 鐪?lift 缁煎悎鎶ュ憡

**snapshot_id**: v1200-6e2ab173
**dim_version**: 0.6.10
**timestamp**: 1785761165
**elapsed_seconds**: 0.0001

## 1. V1200 3-formula honest presentation (涓?17:43 瀹炰簨姹傛槸)

| formula | value | gap to 0.98 | vs north_star % |
|---|---|---|---|
| formula_1 additive (continuity) | 0.9518 | +0.0282 | 97.12% |
| formula_2 recompute (V1153 std) | **0.9518** | +0.0282 | 97.12% |
| formula_3 corrected (rebuild) | **0.9518** | +0.0282 | 97.12% |

**inflation_gap_additive_vs_recompute**: +0.0000
**inflation_gap_additive_vs_corrected**: +0.0000

## 2. V1200 Baselines (continuity)

- V1197 additive = 1.0199
- V1197 recompute = 0.9148
- V1197 corrected = 0.9148

## 3. V1200 2 dim lifts (V1198 + V1199)

| dim | baseline | new_value | delta | weight | contribution | source |
|---|---|---|---|---|---|---|
| v2_philosophy | 0.7200 | **0.8800** | +0.1600 | 0.0500 | +0.0080 | V1198 v2_philosophy lift (淇?V1161 attribute 鏌ユ壘婕?AL |
| real_llm_benchmark | 0.4160 | **0.9960** | +0.5800 | 0.0500 | +0.0290 | V1199 real_llm_benchmark lift (V1166 鎺?V1190 鏇夸唬 V1 |

**total_lift_螖**: +0.0370

## 4. V1200 vs ASI 鍘嗗彶

| 鐗堟湰 | ASI recompute | 螖 vs V1200 | source |
|---|---|---|---|
| V1194 V0.6.6 | 0.9457 | -0.0043 | 3-dim lift (world_model/re_production/self_improving 闄嶈繃, V1197 recovery) |
| V1197 V0.6.9 | 0.9148 | +0.0352 | 3-formula honest recovery |
| **V1200 V0.6.10** | **0.9518** | **+0.0370** | 2-dim 鐪?lift (v2_philosophy + real_llm_benchmark) |

## 5. V3 鍝插瀹堥棬 (涓?17:58 + 涓?20:46 涓嶅亣瑁?

- 涓嶅亣瑁?V1200 = ASI 缁堟瀬 (V1200 = V0.6.10 涓棿, 鍖楁瀬鏄?0.98)
- 涓嶅亣瑁?V1200 = V1198+V1199 鍏ㄦ浛浠?(V1200 = 缁煎悎鎶ュ憡, V1198/V1199 浠?own)
- 涓嶅亣瑁?V1200 lift = ASI V1.0 (V1200 = V0.6.10 涓棿鐗堟湰)
- 涓嶅亣瑁?additive = recompute (3-formula 濡傚疄鎶? V1200 additive = recompute 鏄湡 lift 鐗瑰緛, V1197 additive=1.02 鏄?inflation artifact)
- 涓嶅亣瑁?ASI V0.6.10 = ASI 鐪熸閫?(鍙槸娴嬮噺淇 + 琛ョ湡娴嬫ā鍧?
- 涓嶅亣瑁?V1200 = V1194 鍏ㄦ浛浠?(V1194 鏄?V0.6.6, V1200 鏄?V0.6.10 鍗囩骇璺緞)
- 涓嶅亣瑁?V1200 0.95 = 鍖楁瀬鏄?0.98 (gap=-0.03, 96.94%)

## 6. Honest note (涓?17:43 瀹炰簨姹傛槸)

V1200 3-formula 濡傚疄鎶?(涓嶉瓟鏀?. additive=0.9518 | recompute=0.9518 | corrected=0.9518. V1200 inflation_gap_additive_vs_recompute = +0.0000 (V1197 鏄?+0.1051 inflation artifact; V1200 鏄湡 lift, inflation_gap 鈮?0). ASI 鍖楁瀬鏄?0.98 gap: +0.0282. V1200 鈮?ASI (gap < 0).

## 7. Usage (涓?00:56 浠讳綍浜洪兘鑳芥帴鎵?

```python
from apeireth.v1200_asi_v0610_dual_dim_lift import (
    measure_v1200, measure_v1200_additive, measure_v1200_recompute,
    measure_v1200_corrected, compute_v1200_lift,
    render_report_md, write_artifact,
)

# 3-formula tuple
f1, f2, f3 = measure_v1200()

# 鍗?formula
additive = measure_v1200_additive()
recompute = measure_v1200_recompute()
corrected = measure_v1200_corrected()

# Full report
report = compute_v1200_lift()
print(render_report_md(report))
write_artifact(report)
```

```bash
# CLI
python -m apeireth.v1200_asi_v0610_dual_dim_lift                       # 榛樿 3-formula + JSON
python -m apeireth.v1200_asi_v0610_dual_dim_lift --json                # JSON stdout
python -m apeireth.v1200_asi_v0610_dual_dim_lift --measure             # formula_2 recompute
python -m apeireth.v1200_asi_v0610_dual_dim_lift --measure-additive    # formula_1 additive
python -m apeireth.v1200_asi_v0610_dual_dim_lift --measure-corrected   # formula_3 corrected
python -m apeireth.v1200_asi_v0610_dual_dim_lift --report              # markdown
```

_artifact_path: artifacts\v1200_asi_v0610_dual_dim_lift.json_

_V1200 涓?22:33 + 涓?17:43 + 涓?19:33 + 涓?13:31 + 涓?17:58+20:46 + 涓?23:44 + 涓?00:56 + 涓?00:44._
