#![no_main]

//! fuzz: **受控工作树写面（`tool.worktree_apply`）的改单/补丁交叉核对** ——
//! 「**改单之外无处混入**」的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/worktree_apply.rs`
//! （`cross_check_change_order` / `REFUSAL_DRIFT_PREFIX` / `REFUSAL_VERIFY_PREFIX` /
//! `ValidateVerifyCommands` 的等价判据）。
//!
//! ## 为什么值得 fuzz（**这是能改真实工作区多文件的面**）
//!
//! 模块头写死的合同：**四拍受控写**（计划→改→验证→落），每拍可审、**任一失败整批不落地**；
//! 计划拍把「补丁声明的动作」与「改单」**逐项核对**（文件集 + 动作标签），
//! **不一致拒绝即帧** ⇒ ⇒ **改单之外的写无处混入**。
//! ⇒ ⇒ 若交叉核对漏一条 ⇒ ⇒ **可以绕过审批卡上写着的那张改单去改别的文件**
//! （审批卡载荷 = 改单摘要：文件数 / 动作分布）⇒ ⇒ **审批就形同虚设**。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的分支推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **补丁声明改单之外的目标** ⇒ 必拒 | 「补丁声明了改单之外的目标」分支 |
//! | O2 | **改单有补丁未动的目标** ⇒ 必拒 | 「改单声明了补丁未动的目标」分支 |
//! | O3 | **同一目标动作不一致**（改单 update vs 补丁 delete 等）⇒ 必拒 | 「动作不一致」分支 |
//! | O4 | **补丁内同一目标重复声明** ⇒ 必拒（**即使两次标签相同也拒**） | 「补丁重复声明同一目标」分支 |
//! | O5 | **路径归一后比较**：`./a.txt` 与 `a.txt` 视为**同一目标** | `normalized_path_key` |
//! | O6 | **完全一致必通过**（文件集与标签逐项相等） | `Ok(())` 分支 |
//! | O7 | **拒绝必须带 `REFUSAL_DRIFT_PREFIX`**（可审计的固定前缀） | `REFUSAL_DRIFT_PREFIX` 常量 |
//!
//! ## 诚实边界
//!
//! - **不测**真实落盘（四拍的暂存面/观测门禁/原子写 ⇒ 需要 root + ObservedGate 会话态，另立面）；
//! - **不测** `prepare_apply` 的参数解析（JSON 形态面，另立）；
//! - **不测**审批 hook（治理面）；
//! - ⚠️ `normalized_path_key` 是**私有**函数 ⇒ 本 target 只断言「归一后的等价性」
//!   这一**可观察行为**，不复刻其内部实现。

use std::path::PathBuf;

use libfuzzer_sys::fuzz_target;

use apeireth_tools_canonical::apply_patch::{FilePatchAction, PatchHunk};
use apeireth_tools_canonical::worktree_apply::{cross_check_change_order, REFUSAL_DRIFT_PREFIX};
use apeireth_tools_canonical::worktree_plan::{
    WorktreeChangeAction, WorktreeChangeOrder, WorktreeFileChange,
};

fn change(path: &str, action: WorktreeChangeAction) -> WorktreeFileChange {
    WorktreeFileChange {
        path: path.to_string(),
        action,
        expect: format!("预期 {path} 变为 {:?}", action),
    }
}

fn order(files: Vec<WorktreeFileChange>) -> WorktreeChangeOrder {
    WorktreeChangeOrder {
        goal: "fuzz 目标".to_string(),
        files,
    }
}

fn patch_add(path: &str) -> FilePatchAction {
    FilePatchAction::Add {
        path: PathBuf::from(path),
        content: "x".to_string(),
    }
}

fn patch_delete(path: &str) -> FilePatchAction {
    FilePatchAction::Delete {
        path: PathBuf::from(path),
    }
}

fn patch_update(path: &str) -> FilePatchAction {
    FilePatchAction::Update {
        path: PathBuf::from(path),
        hunks: vec![PatchHunk {
            search_context: "a".to_string(),
            replacement_content: "b".to_string(),
        }],
    }
}

fn patch_for(path: &str, action: WorktreeChangeAction) -> FilePatchAction {
    match action {
        WorktreeChangeAction::Add => patch_add(path),
        WorktreeChangeAction::Update => patch_update(path),
        WorktreeChangeAction::Delete => patch_delete(path),
    }
}

const ACTIONS: [WorktreeChangeAction; 3] = [
    WorktreeChangeAction::Add,
    WorktreeChangeAction::Update,
    WorktreeChangeAction::Delete,
];

fuzz_target!(|data: &[u8]| {
    let b = |i: usize| -> u8 { data.get(i).copied().unwrap_or(0) };

    // ── 场景 A：完全一致 ⇒ 必通过（O6）
    {
        let action = ACTIONS[(b(0) as usize) % 3];
        let files = vec![
            change("a.txt", action),
            change("sub/b.txt", ACTIONS[(b(1) as usize) % 3]),
        ];
        let patches: Vec<FilePatchAction> = files
            .iter()
            .map(|f| patch_for(&f.path, f.action))
            .collect();
        let result = cross_check_change_order(&order(files), &patches);
        assert!(
            result.is_ok(),
            "改单与补丁**文件集与标签逐项相等**时必须通过: err={:?}",
            result.err()
        );
    }

    // ── 场景 B：补丁多动一个改单之外的目标 ⇒ 必拒（O1）
    {
        let files = vec![change("a.txt", WorktreeChangeAction::Update)];
        let patches = vec![
            patch_update("a.txt"),
            patch_add(&format!("extra-{}.txt", b(2))),
        ];
        let result = cross_check_change_order(&order(files), &patches);
        assert!(result.is_err(), "补丁动了改单之外的目标必须拒绝即帧");
        let message = result.unwrap_err();
        assert!(
            message.contains(REFUSAL_DRIFT_PREFIX),
            "拒绝必须带固定前缀（可审计）: {message:?}"
        );
    }

    // ── 场景 C：改单有补丁未动的目标 ⇒ 必拒（O2）
    {
        let files = vec![
            change("a.txt", WorktreeChangeAction::Update),
            change(&format!("b-{}.txt", b(3)), WorktreeChangeAction::Add),
        ];
        let patches = vec![patch_update("a.txt")];
        let result = cross_check_change_order(&order(files), &patches);
        assert!(result.is_err(), "改单声明的目标补丁未动必须拒绝即帧");
        let message = result.unwrap_err();
        assert!(message.contains(REFUSAL_DRIFT_PREFIX), "拒绝必须带固定前缀");
    }

    // ── 场景 D：同一目标动作不一致 ⇒ 必拒（O3）
    {
        let order_action = ACTIONS[(b(4) as usize) % 3];
        let patch_action = ACTIONS[(b(5) as usize) % 3];
        if order_action != patch_action {
            let files = vec![change("a.txt", order_action)];
            let patches = vec![patch_for("a.txt", patch_action)];
            let result = cross_check_change_order(&order(files), &patches);
            assert!(
                result.is_err(),
                "同一目标动作不一致必须拒绝即帧: 改单 {order_action:?} vs 补丁 {patch_action:?}"
            );
            let message = result.unwrap_err();
            assert!(
                message.contains(REFUSAL_DRIFT_PREFIX),
                "动作不一致的拒绝必须带固定前缀（可审计）: {message:?}"
            );
        }
    }

    // ── 场景 E：补丁内同一目标重复声明 ⇒ 必拒（O4）——**即使两次标签相同**
    {
        let action = ACTIONS[(b(6) as usize) % 3];
        let files = vec![change("a.txt", action)];
        let patches = vec![patch_for("a.txt", action), patch_for("a.txt", action)];
        let result = cross_check_change_order(&order(files), &patches);
        assert!(
            result.is_err(),
            "补丁重复声明同一目标必须拒绝即帧（即使两次动作标签相同）"
        );
        let message = result.unwrap_err();
        assert!(message.contains(REFUSAL_DRIFT_PREFIX), "重复声明的拒绝必须带固定前缀");
    }

    // ── 场景 F：路径归一后视为同一目标（O5）
    {
        // 改单写 `a.txt`，补丁写 `./a.txt` ⇒ 归一是同一目标 ⇒ 动作一致时必通过
        let files = vec![change("a.txt", WorktreeChangeAction::Update)];
        let patches = vec![patch_update("./a.txt")];
        let result = cross_check_change_order(&order(files), &patches);
        assert!(
            result.is_ok(),
            "`./a.txt` 与 `a.txt` 归一后是同一目标，动作一致时必须通过: err={:?}",
            result.err()
        );
        // 反过来：改单 `./a.txt`、补丁 `a.txt` ⇒ 同样应通过
        let files2 = vec![change("./a.txt", WorktreeChangeAction::Update)];
        let patches2 = vec![patch_update("a.txt")];
        assert!(
            cross_check_change_order(&order(files2), &patches2).is_ok(),
            "反向归一也必须视为同一目标"
        );
        // 但归一后同目标而动作不同 ⇒ 必拒
        let files3 = vec![change("a.txt", WorktreeChangeAction::Update)];
        let patches3 = vec![patch_delete("./a.txt")];
        assert!(
            cross_check_change_order(&order(files3), &patches3).is_err(),
            "归一后同目标但动作不同必须拒绝即帧"
        );
    }

    // ── 场景 G：前缀常量本身是稳定契约（O7）
    assert!(
        !REFUSAL_DRIFT_PREFIX.trim().is_empty(),
        "漂移拒绝前缀不得为空（它是审计的固定锚）"
    );

    // ── 场景 H：多条一致改单（含数据驱动的路径）必通过
    {
        let n = 1 + (b(7) as usize % 3);
        let files: Vec<WorktreeFileChange> = (0..n)
            .map(|i| {
                let suffix = String::from_utf8_lossy(&data.get(8 + i..9 + i).unwrap_or(&[]))
                    .to_string();
                change(
                    &format!("dir{i}/f{i}-{suffix}.txt"),
                    ACTIONS[(b(0) as usize + i) % 3],
                )
            })
            .collect();
        let patches: Vec<FilePatchAction> = files
            .iter()
            .map(|f| patch_for(&f.path, f.action))
            .collect();
        assert!(
            cross_check_change_order(&order(files), &patches).is_ok(),
            "多条一致改单（任意路径）必须通过"
        );
    }
});
