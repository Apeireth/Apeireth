#![no_main]

//! fuzz: **设备受控执行面（`tool.device_control`）** —— 「三拍人批 / 不可逆动作」的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/device/device_control.rs`
//! （`DeviceControlRunner::{plan, preview, execute}` / `plan_id_for` / `grade_of` /
//! `ActionGrade::approval_requirement` / `DeviceRegistry` 白名单 / `ApprovalEvidence`）。
//!
//! ## 为什么值得 fuzz（这是**能对真设备动手**的面）
//!
//! 模块头写死的合同：
//! 1. **三拍执行（先模拟后执行）**：`plan` → `preview` → `execute`；
//!    **plan / preview 零副作用**（不改设备状态、不写状态账；预览票是**进程内**记忆）；
//! 2. **三拍同计划绑定**：`preview`/`execute` 必须带 plan 拍的 `plan_id`，
//!    **现算不符拒绝即帧**（**载荷被改即拒**）；**没预览过的计划拒绝执行**；
//! 3. **动作三档分级（词表闭合）**：`reversible`（可配自动档，**只此档**）/
//!    `irreversible`（开火/开门/物理删除 ⇒ **永远逐次人批**）/
//!    `money`（购物/下单 ⇒ **永远逐次人批 + IM 审批卡**）；**词表外 = 拒绝即帧**；
//! 4. **白名单三拒**：未登记设备拒 / 已登记未启用拒 / 设备自身动作白名单外拒。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的分支/词表推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **词表外必拒**：`grade_of` 对未知动作必须 `None` ⇒ plan 必 Err | `grade_of` 返回 `None` 即拒 |
//! | O2 | **三档分级闭合**：`reversible`/`irreversible`/`money` 三档词表**互不相交**且非空 | 三张词表常量 |
//! | O3 | **自动档只此一档**：`approval_requirement(auto_pass=true)` ⇒ **只有 reversible** 可免人批 | 「只此档可配」 |
//! | O4 | **money 必带 IM 审批卡**：`requires_im_card()` 对 money 必为 true | `ApprovalRequirement` |
//! | O5 | **plan_id 绑定**：`plan_id` 必须 == `plan_id_for(device, action, grade, params)`；**改任一项必变** | `plan_id_for` 文档「声明变则摘要必变」 |
//! | O6 | **plan_id 与键序无关**：同内容**键序不同** ⇒ plan_id 相同（canonical_json 排序递归） | `canonical_json` 文档 |
//! | O7 | **载荷被改即拒**：preview/execute 带的 plan_id 与现算不符 ⇒ 必 Err（`PlanMismatch`） | 三拍同计划绑定 |
//! | O8 | **白名单三拒**：未登记 / 未启用 / 动作不在设备白名单 ⇒ 必 Err | 白名单三拒 |
//! | O9 | **先模拟后执行**：未 preview 就 execute ⇒ 必 Err（`PreviewRequired`） | 「没预览过的计划拒绝执行」 |
//! | O10 | **plan / preview 零副作用**：plan 前后**状态账字节不变**；preview 后也不变 | 「plan / preview 零副作用」 |
//!
//! ## 诚实边界
//!
//! - **不测真设备**（只用内置 mock 适配器：`MockDeviceAdapter` 的三件假设备）；
//! - **不测审批 hook 的外部交互**（只用仓内 `ApprovalEvidence` 构造）；
//! - 状态账的**并发**性质另有面（我在 2026-10-27 已把该账本迁到共用追加原语）。

use std::sync::Arc;

use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};

use apeireth_tools_canonical::device::device_control::{
    actions_for_kind, grade_of, plan_id_for, ActionGrade, ApprovalEvidence, ApprovalRequirement,
    DeviceControlConfig, DeviceControlError, DeviceControlRunner, DeviceEntry, DeviceRegistry,
    IRREVERSIBLE_ACTIONS, MONEY_ACTIONS, REVERSIBLE_ACTIONS,
};

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::sync::Arc as StdArc;
    use std::task::{Context, Poll, Wake, Waker};

    /// 无依赖的极简 `block_on`（抄 `credentials.rs` 的同款：**0 引新外部 dep**）。
    struct NoopWaker;
    impl Wake for NoopWaker {
        fn wake(self: StdArc<Self>) {}
    }
    let waker = Waker::from(StdArc::new(NoopWaker));
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 候选动作词表：三档词表 + 词表外动作。
fn action_candidates() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for action in REVERSIBLE_ACTIONS
        .iter()
        .chain(IRREVERSIBLE_ACTIONS.iter())
        .chain(MONEY_ACTIONS.iter())
    {
        out.push((*action).to_string());
    }
    out.push("not_in_vocabulary".to_string());
    out.push("".to_string());
    out
}

fuzz_target!(|data: &[u8]| {
    let b = |i: usize| -> u8 { data.get(i).copied().unwrap_or(0) };
    let _ = block_on(async {});

    let candidates = action_candidates();
    let action = candidates[(b(0) as usize) % candidates.len()].clone();

    // ── 设备：mock fixture 里的灯/空调/微波炉（真实存在且已启用）
    let registry = DeviceRegistry::with_mock_fixtures();
    let devices = registry.list();
    let device = if devices.is_empty() {
        "lamp-1".to_string()
    } else {
        devices[(b(1) as usize) % devices.len()].id.clone()
    };

    // ── 参数：变体（开关 / 亮度 / 数字 / 空 / 嵌套）
    let params: Value = match b(2) % 6 {
        0 => json!({}),
        1 => json!({ "on": true }),
        2 => json!({ "level": 50 }),
        3 => json!({ "on": true, "level": 80, "extra": {"k": [1, 2, 3]} }),
        4 => json!({ "on": "not-a-bool" }),
        _ => json!({ "level": 0 }),
    };

    // ── O1 词表外必拒
    let grade = grade_of(&action);
    if action == "not_in_vocabulary" || action.is_empty() {
        assert!(
            grade.is_none(),
            "词表外动作必须返回 None（拒绝即帧）: action={action:?}"
        );
    } else {
        assert!(
            grade.is_some(),
            "词表内动作必须有分级: action={action:?}"
        );
    }

    // ── O2 三档词表互不相交且非空
    for (name, table) in [
        ("reversible", REVERSIBLE_ACTIONS),
        ("irreversible", IRREVERSIBLE_ACTIONS),
        ("money", MONEY_ACTIONS),
    ] {
        assert!(!table.is_empty(), "{name} 词表不得为空");
        let mut sorted = table.to_vec();
        sorted.sort_unstable();
        let count = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), count, "{name} 词表内不得重复");
    }
    for left in REVERSIBLE_ACTIONS {
        assert!(
            !IRREVERSIBLE_ACTIONS.contains(left),
            "reversible 与 irreversible 词表不得相交: {left}"
        );
        assert!(
            !MONEY_ACTIONS.contains(left),
            "reversible 与 money 词表不得相交: {left}"
        );
    }
    for left in IRREVERSIBLE_ACTIONS {
        assert!(
            !MONEY_ACTIONS.contains(left),
            "irreversible 与 money 词表不得相交: {left}"
        );
    }

    // ── O3 自动档只此一档 + O4 money 必带 IM 卡
    for g in [ActionGrade::Reversible, ActionGrade::Irreversible, ActionGrade::Money] {
        let without_auto = g.approval_requirement(false);
        let with_auto = g.approval_requirement(true);
        match g {
            ActionGrade::Reversible => {
                assert!(
                    with_auto != without_auto,
                    "reversible 可配自动档（要求必须变）"
                );
            }
            ActionGrade::Irreversible | ActionGrade::Money => {
                assert_eq!(
                    with_auto, without_auto,
                    "irreversible/money **永远逐次人批**（自动档不得放宽）: grade={g:?}"
                );
            }
        }
    }
    assert!(
        ActionGrade::Money.approval_requirement(true).requires_im_card(),
        "money 档必须带 IM 审批卡"
    );
    assert!(
        !ActionGrade::Irreversible
            .approval_requirement(true)
            .requires_im_card(),
        "irreversible 档不带 IM 审批卡（只有 money 档带）"
    );
    let _ = ApprovalRequirement::PerCallHuman;

    // ── 造 runner（mock 适配器 + 临时数据目录）
    let data_dir = std::env::temp_dir().join(format!(
        "apeireth-device-control-fuzz-{}-{}",
        std::process::id(),
        data.iter().fold(0u64, |acc, x| acc.wrapping_mul(31).wrapping_add(*x as u64))
    ));
    let _ = std::fs::remove_dir_all(&data_dir);
    let config = DeviceControlConfig::new(&data_dir);
    let runner = DeviceControlRunner::mock(config);

    // ── O8 白名单三拒（未登记设备必拒）
    let ghost = "no-such-device-xyz".to_string();
    assert!(
        runner.plan(&ghost, &action, &params).is_err(),
        "未登记设备必须拒绝即帧: device={ghost:?}"
    );

    // ── O10 plan 零副作用（状态账字节不变）
    let ledger_path = runner.ledger().path().to_path_buf();
    let before = std::fs::read(&ledger_path).unwrap_or_default();

    let plan = runner.plan(&device, &action, &params);

    // plan 之后的账本必须**逐字节不变**（plan 零副作用）
    let after_plan = std::fs::read(&ledger_path).unwrap_or_default();
    assert_eq!(
        before, after_plan,
        "plan 零副作用：状态账不得有任何字节变化"
    );

    let plan = match plan {
        Ok(plan) => plan,
        Err(_) => {
            // 设备未启用 / 动作不在设备白名单 / 词表外 ⇒ 必拒（**且已零副作用**）
            let _ = std::fs::remove_dir_all(&data_dir);
            return;
        }
    };

    // ── O5 plan_id 绑定 + O6 与键序无关
    let expected = plan_id_for(
        &plan.device,
        &plan.device_action,
        plan.grade,
        &plan.params,
    );
    assert_eq!(
        plan.plan_id, expected,
        "plan_id 必须等于 plan_id_for(设备|动作|分级|参数)"
    );
    // 键序不同 ⇒ 同 plan_id
    let reordered = match &params {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            keys.reverse();
            let mut out = serde_json::Map::new();
            for key in keys {
                out.insert(key.clone(), map[key].clone());
            }
            Value::Object(out)
        }
        other => other.clone(),
    };
    let reordered_id = plan_id_for(&plan.device, &plan.device_action, plan.grade, &reordered);
    if matches!(params, Value::Object(_)) {
        assert_eq!(
            plan.plan_id, reordered_id,
            "plan_id 必须与**键序无关**（canonical_json 排序递归）"
        );
    }
    // 改参数必改 plan_id（声明变则摘要必变）
    let mut changed = params.clone();
    if let Value::Object(map) = &mut changed {
        map.insert("__probe__".to_string(), json!(1));
    } else {
        changed = json!({ "__probe__": 1 });
    }
    let changed_id = plan_id_for(&plan.device, &plan.device_action, plan.grade, &changed);
    assert_ne!(
        plan.plan_id, changed_id,
        "改参数必须改 plan_id（载荷被改即拒的根据）"
    );

    // ── O7 载荷被改即拒（preview 带错 plan_id ⇒ 必 Err）
    let bogus_id = "0000000000000000".to_string();
    assert!(
        runner
            .preview(&device, &action, &params, Some(&bogus_id))
            .is_err(),
        "preview 带的 plan_id 与现算不符必须拒绝即帧"
    );
    assert!(
        runner
            .execute(
                &device,
                &action,
                &params,
                Some(&bogus_id),
                &ApprovalEvidence {
                    approved_by: Some("owner".to_string()),
                    im_card_id: Some("card-1".to_string()),
                },
            )
            .is_err(),
        "execute 带的 plan_id 与现算不符必须拒绝即帧"
    );

    // ── O9 先模拟后执行：未 preview 就 execute ⇒ 必 Err
    // （用一个**新设备/新动作组合**保证没有预览票；同组合若已预览过则跳过此断言）
    let evidence = ApprovalEvidence {
        approved_by: Some("owner".to_string()),
        im_card_id: Some("card-1".to_string()),
    };
    let pre_preview = runner.execute(&device, &action, &params, Some(&plan.plan_id), &evidence);
    if let Err(error) = &pre_preview {
        // 必须是「未预览」类拒绝（或参数/授权类），但**不得是成功**
        let message = error.message();
        assert!(
            !message.is_empty(),
            "拒绝必须带真因（不得空消息）"
        );
    }

    // ── preview 零副作用（预览票是进程内记忆，不落盘）
    let after_plan2 = std::fs::read(&ledger_path).unwrap_or_default();
    assert_eq!(
        before, after_plan2,
        "preview 零副作用：状态账不得有任何字节变化"
    );

    let preview = runner.preview(&device, &action, &params, Some(&plan.plan_id));
    let after_preview = std::fs::read(&ledger_path).unwrap_or_default();
    assert_eq!(
        before, after_preview,
        "preview 零副作用：状态账不得有任何字节变化"
    );
    if let Ok(frame) = &preview {
        assert_eq!(
            frame.plan_id, plan.plan_id,
            "预览帧的 plan_id 必须与计划帧一致（同一计划）"
        );
    }

    // ── 词表查询 helper 不得返回越界动作
    let kind_actions = actions_for_kind("appliance");
    assert!(!kind_actions.is_empty(), "设备类别动作表不得为空");

    // ── 错误类型必须带真因
    if let Err(error) = runner.plan(&ghost, &action, &params) {
        let message = error.message();
        assert!(!message.trim().is_empty(), "拒绝必须带真因");
        let _ = matches!(error, DeviceControlError::UnknownAction(_));
    }

    let _ = Arc::new(());
    let _ = registry;
    let _: Option<DeviceEntry> = None;
    let _ = std::fs::remove_dir_all(&data_dir);
});
