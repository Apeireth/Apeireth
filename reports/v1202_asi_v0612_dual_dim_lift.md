# V1202 ASI V0.6.12 鍙?dim lift 鎶ュ憡

- snapshot_id: `v1202-81bebc70`
- V1202 鐗堟湰: `0.1.0` (dim_version=`0.6.12`)
- 鏃堕棿鎴? `1785765473.462` (elapsed: `5.248s`)

## 3-formula (涓?17:43 瀹炰簨姹傛槸)

- **formula_1 additive** (continuity):  `0.9682`
- **formula_2 recompute** (V1153 std): `0.9682` 鈫?涓绘寚鏍?
- **formula_3 corrected** (rebuild):   `0.9682`
- **inflation_gap additive vs recompute**: `0.0000`
- **inflation_gap additive vs corrected**: `0.0000`

## ASI 鍖楁瀬鏄?(涓?22:33 LOCKED)

- ASI north_star = `0.9800`
- V1201 baseline (recompute) = `0.9624`
- V1202 lifted (recompute) = `0.9682`
- 螖 vs V1201 = `+0.0058`
- gap to north_star = `+0.0118`
- position = `98.80%` of north star

## 2 dim lifts (涓?13:31 澶ц儐婵€杩?

- n_dims_lifted: `2`
- n_dims_pass: `2` / partial: `0` / missing: `0`

### `rubric_open`

- baseline (V1201): `0.8643`
- new_value (V1202): `0.9400`
- 螖: `+0.0757`
- weight: `0.0500`
- lift_contribution: `+0.0038`
- status: `R`
- sub_dim_count: `10`
- source: `V1202 rubric_open lift (V1160 5 sub-dim + 5 new V1202 sub-dim)`

  - V1201 baseline 0.8643
  - V1202 鐪熻ˉ: 10 sub-dim 鐪熸祴 (5 V1160 澶嶇敤 + 5 V1202 鏂板)
  - sub-dim pass=9 / partial=1 / missing=0
  - V1202 鎬?0.9400 (螖=+0.0757)
  - ASI recompute contribution = 0.05 脳 +0.0757 = +0.0038

### `self_organizing_core`

- baseline (V1201): `0.9095`
- new_value (V1202): `0.9500`
- 螖: `+0.0405`
- weight: `0.0500`
- lift_contribution: `+0.0020`
- status: `R`
- sub_dim_count: `10`
- source: `V1202 self_organizing_core lift (V1165 5 sub-dim + 5 new V1202 sub-dim)`

  - V1201 baseline 0.9095
  - V1202 鐪熻ˉ: 10 sub-dim 鐪熸祴 (5 V1165 澶嶇敤 + 5 V1202 鏂板)
  - sub-dim pass=10 / partial=0 / missing=0
  - V1202 鎬?0.9500 (螖=+0.0405)
  - ASI recompute contribution = 0.05 脳 +0.0405 = +0.0020

## rubric_open 10 sub-dim 鐪熸祴

- n_subdims_total: `10`
- n_subdims_pass: `9`
- n_subdims_partial: `1`
- n_subdims_missing: `0`

| sub-dim | score | status |
|---------|-------|--------|
| `evaluate_week_real` | `1.0000` | `R` |
| `halting_signals_real` | `1.0000` | `R` |
| `dashboard_render_real` | `1.0000` | `R` |
| `v3_guards_real` | `0.8000` | `R` |
| `track_decision_real` | `0.6000` | `P` |
| `halting_signal_real_run` | `1.0000` | `R` |
| `v1074_v03_above_floor_real` | `1.0000` | `R` |
| `v03_history_real` | `1.0000` | `R` |
| `all_ok_real` | `1.0000` | `R` |
| `guards_v3_guards_real` | `1.0000` | `R` |

## self_organizing_core 10 sub-dim 鐪熸祴

- n_subdims_total: `10`
- n_subdims_pass: `10`
- n_subdims_partial: `0`
- n_subdims_missing: `0`

| sub-dim | score | status |
|---------|-------|--------|
| `autopoietic_closure` | `1.0000` | `R` |
| `autocatalytic_raf` | `1.0000` | `R` |
| `requisite_variety` | `1.0000` | `R` |
| `dissipative_export` | `1.0000` | `R` |
| `chemoton_coupling` | `0.8333` | `R` |
| `mr_closure_real` | `1.0000` | `R` |
| `adaptive_diversity_real` | `0.8333` | `R` |
| `order_param_dominance_real` | `1.0000` | `R` |
| `report_readability_real` | `0.8333` | `R` |
| `measure_dict_complete_real` | `1.0000` | `R` |

## V1202 Notes (涓?17:43 + 17:58 + 20:46 + 22:33 + 19:33 + 13:31 + 23:44 + 00:56 + 00:44)

- V1202 ASI V0.6.12: 2 dim 鐪?lift, recompute 0.9624 鈫?0.9682 (螖=+0.0058)
- rubric_open: 0.8643 鈫?0.9400 (螖=+0.0757)
- self_organizing_core: 0.9095 鈫?0.9500 (螖=+0.0405)
- gap to north_star: +0.0118 (鍖楁瀬鏄?0.9800 LOCKED 涓?22:33)
- position: 98.80% of north star
- inflation guard: additive vs recompute = 0.0000 (V1197 fix, no inflation)

- rubric_open baseline: 0.8643 (V1201 artifact or V1196 fallback)
- self_organizing_core baseline: 0.9095 (V1201 artifact or V1196 fallback)
- --- rubric_open (5+5=10 sub-dim) ---
- rubric_open aggregate: 0.9400 (螖 vs baseline 0.8643 = +0.0757)
- --- self_organizing_core (5+5=10 sub-dim) ---
- self_organizing_core aggregate: 0.9500 (螖 vs baseline 0.9095 = +0.0405)

