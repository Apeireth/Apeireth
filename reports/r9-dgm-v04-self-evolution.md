# R9 DGM Archive v0.4 — 真演化 50 轮 + Track B Identity 串联

- version: `0.4.0`
- iterations_requested: **50**
- iterations_completed: **50**
- method_requested: **parent_child**
- identity_id: `ca_dev_17759e94ef71`
- bridge_v1072: **False**
- archive_size (P5 retain): **14**
- n_retain: **14** / n_discard: **36** / n_reject: **0**
- lift_max: **0.0289** / lift_mean: **0.0109**
- n_asi_pretend_total (V3 守门): **0**

## 3 方法对照 (parent-child / sexual / asexual)

| 方法 | n_total | n_retain | n_discard | n_reject | retain_rate |
|---|---:|---:|---:|---:|---:|
| parent_child | 17 | 3 | 14 | 0 | 17.65% |
| sexual | 16 | 6 | 10 | 0 | 37.50% |
| asexual | 17 | 5 | 12 | 0 | 29.41% |

## 每轮 lift (50 轮真演化)

| 轮次 | 方法 | composite | delta | lift | verdict | reject_reason |
|---:|---|---:|---:|---:|---|---|
|0|baseline|0.9718|+0.0000|+0.0000|baseline||
|1|parent_child|0.9999|+0.0281|+0.0289|retain||
|2|asexual|0.9481|-0.0237|-0.0244|discard|composite 0.9481 < baseline+0.015 (0.9718)|
|3|parent_child|0.9922|+0.0203|+0.0209|discard|composite 0.9922 < threshold 0.9999|
|4|sexual|0.9999|+0.0281|+0.0289|discard|composite 0.9999 < threshold 0.9999|
|5|asexual|0.9999|+0.0281|+0.0289|retain||
|6|parent_child|0.9779|+0.0061|+0.0063|discard|composite 0.9779 < baseline+0.015 (0.9718)|
|7|sexual|0.9999|+0.0281|+0.0289|retain||
|8|asexual|0.9999|+0.0281|+0.0289|retain||
|9|parent_child|0.9999|+0.0281|+0.0289|discard|composite 0.9999 < threshold 0.9999|
|10|sexual|0.9940|+0.0222|+0.0228|discard|composite 0.9940 < threshold 0.9999|
|11|asexual|0.9660|-0.0059|-0.0060|discard|composite 0.9660 < baseline+0.015 (0.9718)|
|12|parent_child|0.9366|-0.0353|-0.0363|discard|composite 0.9366 < baseline+0.015 (0.9718)|
|13|sexual|0.9883|+0.0165|+0.0170|discard|composite 0.9883 < threshold 0.9999|
|14|asexual|0.9703|-0.0015|-0.0016|discard|composite 0.9703 < baseline+0.015 (0.9718)|
|15|parent_child|0.9868|+0.0149|+0.0154|discard|composite 0.9868 < baseline+0.015 (0.9718)|
|16|sexual|0.9999|+0.0281|+0.0289|retain||
|17|asexual|0.9999|+0.0281|+0.0289|retain||
|18|parent_child|0.9483|-0.0235|-0.0242|discard|composite 0.9483 < baseline+0.015 (0.9718)|
|19|sexual|0.9402|-0.0316|-0.0326|discard|composite 0.9402 < baseline+0.015 (0.9718)|
|20|asexual|0.9999|+0.0281|+0.0289|retain||
|21|parent_child|0.9518|-0.0200|-0.0206|discard|composite 0.9518 < baseline+0.015 (0.9718)|
|22|sexual|0.9819|+0.0101|+0.0104|discard|composite 0.9819 < baseline+0.015 (0.9718)|
|23|asexual|0.9982|+0.0263|+0.0271|discard|composite 0.9982 < threshold 0.9999|
|24|parent_child|0.9999|+0.0281|+0.0289|retain||
|25|sexual|0.9911|+0.0193|+0.0198|discard|composite 0.9911 < threshold 0.9999|
|26|asexual|0.9999|+0.0281|+0.0289|retain||
|27|parent_child|0.9658|-0.0060|-0.0062|discard|composite 0.9658 < baseline+0.015 (0.9718)|
|28|sexual|0.9999|+0.0281|+0.0289|discard|composite 0.9999 < threshold 0.9999|
|29|asexual|0.9625|-0.0093|-0.0096|discard|composite 0.9625 < baseline+0.015 (0.9718)|
|30|parent_child|0.9999|+0.0281|+0.0289|retain||
|31|sexual|0.9999|+0.0281|+0.0289|retain||
|32|asexual|0.9999|+0.0281|+0.0289|discard|composite 0.9999 < threshold 0.9999|
|33|parent_child|0.9939|+0.0221|+0.0227|discard|composite 0.9939 < threshold 0.9999|
|34|sexual|0.9677|-0.0041|-0.0042|discard|composite 0.9677 < baseline+0.015 (0.9718)|
|35|asexual|0.9965|+0.0247|+0.0254|discard|composite 0.9965 < threshold 0.9999|
|36|parent_child|0.9949|+0.0230|+0.0237|discard|composite 0.9949 < threshold 0.9999|
|37|sexual|0.9943|+0.0224|+0.0231|discard|composite 0.9943 < threshold 0.9999|
|38|asexual|0.9680|-0.0039|-0.0040|discard|composite 0.9680 < baseline+0.015 (0.9718)|
|39|parent_child|0.9489|-0.0229|-0.0235|discard|composite 0.9489 < baseline+0.015 (0.9718)|
|40|sexual|0.9999|+0.0281|+0.0289|retain||
|41|asexual|0.9965|+0.0247|+0.0254|discard|composite 0.9965 < threshold 0.9999|
|42|parent_child|0.9524|-0.0194|-0.0200|discard|composite 0.9524 < baseline+0.015 (0.9718)|
|43|sexual|0.9612|-0.0106|-0.0109|discard|composite 0.9612 < baseline+0.015 (0.9718)|
|44|asexual|0.9479|-0.0239|-0.0246|discard|composite 0.9479 < baseline+0.015 (0.9718)|
|45|parent_child|0.9945|+0.0226|+0.0233|discard|composite 0.9945 < threshold 0.9999|
|46|sexual|0.9999|+0.0281|+0.0289|retain||
|47|asexual|0.9492|-0.0226|-0.0233|discard|composite 0.9492 < baseline+0.015 (0.9718)|
|48|parent_child|0.9999|+0.0281|+0.0289|discard|composite 0.9999 < threshold 0.9999|
|49|sexual|0.9999|+0.0281|+0.0289|retain||
|50|asexual|0.9668|-0.0050|-0.0052|discard|composite 0.9668 < baseline+0.015 (0.9718)|

## Identity 锚定审计 (P7 串联)

- identity_id: `ca_dev_17759e94ef71`
- name: Chu Ling / 楚零
- bridge_v1072: False
- core_snapshot_hash: `68a9564456d9cf93`

## V3 守门 (主 17:43 + 主 22:33)

- **n_asi_pretend_total**: 0
- **module_is_not_asi**: v0.4 是工具, ASI 是更大目标 (主 22:33 北极星)
- **measurement_is_not_truth**: lift 是 proxy, 真值仍是更大目标
- **red_queen_paradigm**: 主 20:55 红皇后 = 永远演化, 不是结束态

## DGM v0.4 增量 (vs v0.3)

- P5: 真演化闭环 archive → candidate → evaluate → retain/discard
- P6: 3 方法对照 (parent_child / sexual / asexual)
- P7: Identity 锚定 (Track B 串联 V1095 + V1072)
- P8: V1072 bridge (anchor.bridge_v1072 = True)
- P9: 50 轮 (vs R8 30 轮)
- P10: keep_state 父本引用 (拒绝无父本候选)

真演化 (主 20:55 红皇后归入 8 核心 — 永远演化, 不是结束态).
