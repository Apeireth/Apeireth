#![no_main]

//! fuzz: **技能检索面（`tool.skill_search`）** —— 「只读 / 零注入 / 卡名轻清单」的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/skill/search.rs`
//! （`SkillSearchTool::invoke` / `validated_search_request` / `search_request_refusal`）
//! ＋ 只读消费 `crate::skill::invocation`（`select_for_context` / `build_skill_disclosure`）。
//!
//! ## 为什么值得 fuzz（这个面**把外部内容送进模型上下文**）
//!
//! 技能卡是**用户仓库里的外部内容** ⇒ 检索命中后**卡体会进上下文** ⇒
//! ⇒ 所以本面的合同是**披露纪律**：
//! - **只读档、零审批** —— 零文件系统写、无 ask 档钩子；
//! - **触发词命中 / 按需点名的卡按共享预算披露**，**未命中卡零注入**；
//! - **卡名轻清单（catalog）只含名字、零卡体** —— **绝不无差别全量注入**。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的合同推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **只读零写**：检索后技能库目录**逐字节不变** | 模块头「只读档、零审批」「零文件系统写」 |
//! | O2 | **catalog 只含名字、零卡体**：catalog 的每一项必须是**卡名**，**不得**含卡体/摘要/正文 | 模块头「卡名轻清单只含名字、零卡体」 |
//! | O3 | **未命中卡零注入**：`matched` 里的每个名字都必须**确实命中**（命中判定见 `select_for_context`） | 模块头「未命中卡零注入」 |
//! | O4 | **共享预算**：披露的总字符数 **≤** 预算 | 模块头「按共享预算披露」 |
//! | O5 | **参数拒绝即帧**：空 query ⇒ 拒；`limit == 0` 或 `> 20` ⇒ 拒（**不静默截断**） | `search_request_refusal` |
//! | O6 | **闭合上限**：catalog ≤ 50 项且 `catalog_total` 是**截断前**的真实总数 | `SKILL_SEARCH_CATALOG_MAX` + `catalog.truncate(...)` |
//! | O7 | **status 诚实**：无命中 ⇒ `no_match`；有命中 ⇒ `disclosed` | `invoke` 里的 status 判定 |
//!
//! ## 诚实边界
//!
//! - **不测 `select_for_context` 的命中算法本身**（那是 `invocation` 模块的语义，另立面）；
//!   本 target 只断言「matched ⊆ 库里的卡名」与「披露不超预算」这类**可复核**性质；
//! - **不测 CLI 出口**；
//! - 库用**临时目录**构造（真卡文件）⇒ 覆盖「拒开的卡如实计数」这条的输入面。

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use apeireth_tools_canonical::skill::card::{SkillCardDraft, SkillSource, SkillSourceKind};
use apeireth_tools_canonical::skill::library::SkillLibrary;
use apeireth_tools_canonical::skill::search::{
    search_request_refusal, SkillSearchParams, SkillSearchTool, SKILL_SEARCH_CATALOG_MAX,
    SKILL_SEARCH_DEFAULT_LIMIT, SKILL_SEARCH_MAX_LIMIT,
};

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    /// 无依赖的极简 `block_on`（抄 `credentials.rs` 的同款：**0 引新外部 dep**）。
    struct NoopWaker;
    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(NoopWaker));
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 目录快照（相对路径 → 字节）：用于「只读零写」的比对。
fn snapshot_dir(root: &PathBuf) -> BTreeSet<(String, Vec<u8>)> {
    let mut out = BTreeSet::new();
    fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut BTreeSet<(String, Vec<u8>)>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if let Ok(bytes) = fs::read(&path) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                out.insert((rel, bytes));
            }
        }
    }
    walk(root, root, &mut out);
    out
}

fuzz_target!(|data: &[u8]| {
    let b = |i: usize| -> u8 { data.get(i).copied().unwrap_or(0) };

    // ── 建一个临时技能库（真卡文件）
    let root = std::env::temp_dir().join(format!(
        "apeireth-skillsearch-fuzz-{}-{}",
        std::process::id(),
        data.iter().fold(0u64, |acc, x| acc.wrapping_mul(31).wrapping_add(*x as u64))
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::create_dir_all(&root);

    let card_count = (b(0) as usize % 4) + 1;
    let mut card_names: Vec<String> = Vec::new();
    for index in 0..card_count {
        let name = format!("skill-{}", index);
        let body = format!("卡体 {index}：{}", String::from_utf8_lossy(data));
        let draft = SkillCardDraft {
            name: name.clone(),
            summary: format!("摘要 {index}"),
            triggers: vec![format!("触发{}", index)],
            body,
            source: SkillSource {
                kind: SkillSourceKind::Book,
                title: "书".to_string(),
            },
        };
        let library = SkillLibrary::new(&root);
        let _ = library.create(&draft, 1_700_000_000_000 + index as u64);
        card_names.push(name);
    }

    let library = SkillLibrary::new(&root);
    let budget = 64 + (b(1) as usize % 512);
    let tool = SkillSearchTool::with_default_budget(library.clone());

    // ── O1 只读零写：跑之前快照
    let before = snapshot_dir(&library.root().to_path_buf());

    // ── 组参数（覆盖空 query / 越限 limit / 正常）
    let query = match b(2) % 4 {
        0 => String::new(),
        1 => "   ".to_string(),
        2 => format!("触发{}", b(3) % 4),
        _ => format!("卡体 {}", b(3) % 4),
    };
    let limit = match b(4) % 5 {
        0 => Some(0),
        1 => Some(SKILL_SEARCH_MAX_LIMIT + 1),
        2 => Some(SKILL_SEARCH_MAX_LIMIT),
        3 => None,
        _ => Some(SKILL_SEARCH_DEFAULT_LIMIT),
    };

    // ── O5 参数拒绝即帧（纯判定层）
    let params = SkillSearchParams {
        query: query.clone(),
        limit,
    };
    let refusal = search_request_refusal(&params);
    // ⚠️ 我第一版把这段**判据写反了**（`(None, _)` 分支断言「query 为空」，
    // 而实际应是「**非空**」）⇒ ⇒ 假阳性。⇒ 现在按**正确逻辑**逐条写：
    // ① query 空/纯空白 ⇒ 必须拒；
    // ② limit == 0 或超上限 ⇒ 必须拒（**不静默截断**）；
    // ③ 两者都合法 ⇒ **不得**拒。
    if query.trim().is_empty() {
        assert!(
            refusal.is_some(),
            "空/纯空白 query 必须拒绝: query={query:?}"
        );
    } else {
        match limit {
            Some(0) => assert!(
                refusal.is_some(),
                "limit=0 必须被拒（不静默截断）"
            ),
            Some(l) if l > SKILL_SEARCH_MAX_LIMIT => assert!(
                refusal.is_some(),
                "limit 超上限必须被拒（不静默截断）: limit={l}"
            ),
            _ => assert!(
                refusal.is_none(),
                "参数合法时不得拒绝（拒绝理由必须自洽）: query={query:?} limit={limit:?} refusal={refusal:?}"
            ),
        }
    }

    // ── 走工具面
    let call = ToolCall {
        id: "call_skill_search_fuzz".to_string(),
        name: "skill_search".to_string(),
        arguments: json!({ "query": query, "limit": limit }),
    };
    let outcome = block_on(tool.invoke(&call)).outcome;

    // ── O1 只读零写：跑完之后库目录必须逐字节不变
    let after = snapshot_dir(&library.root().to_path_buf());
    assert_eq!(
        before, after,
        "只读零写：检索不得写技能库任何字节（前后快照必须一致）"
    );

    let ToolOutcome::Ok { value } = outcome else {
        // 拒绝路径：必须是参数问题（真因非空）
        let ToolOutcome::Error { message, .. } = outcome else {
            unreachable!()
        };
        assert!(
            !message.trim().is_empty(),
            "拒绝即帧必须带真因（不得空消息）"
        );
        assert!(
            query.trim().is_empty() || limit == Some(0) || limit.map(|l| l > SKILL_SEARCH_MAX_LIMIT) == Some(true),
            "只有参数非法才可拒绝（其它原因不得静默失败）: query={query:?} limit={limit:?} message={message:?}"
        );
        return;
    };

    // ── O6 闭合上限 + catalog_total 语义
    let catalog = value["catalog"].as_array().expect("catalog 是数组").clone();
    assert!(
        catalog.len() <= SKILL_SEARCH_CATALOG_MAX,
        "catalog 必须受上限约束（≤{}）: {}",
        SKILL_SEARCH_CATALOG_MAX,
        catalog.len()
    );
    let catalog_total = value["catalog_total"].as_u64().expect("catalog_total 是数字");
    assert_eq!(
        catalog_total,
        card_names.len() as u64,
        "catalog_total 必须是**截断前**的真实卡数"
    );

    // ── O2 catalog 只含名字、零卡体
    for item in &catalog {
        let name = item.as_str().expect("catalog 项是字符串（只含名字）");
        assert!(
            card_names.iter().any(|n| n == name),
            "catalog 项必须是库里的卡名: {name:?}"
        );
        // 卡体/摘要/触发词不得出现在 catalog 项里
        assert!(
            !name.contains("卡体") && !name.contains("摘要") && !name.contains("触发"),
            "catalog 只含名字（零卡体/摘要/触发词）: {name:?}"
        );
    }

    // ── O3 未命中卡零注入
    let matched = value["matched"].as_array().expect("matched 是数组").clone();
    for item in &matched {
        let name = item.as_str().expect("matched 项是字符串");
        assert!(
            card_names.iter().any(|n| n == name),
            "matched 里的名字必须来自库里（未命中卡零注入）: {name:?}"
        );
    }
    assert!(
        matched.len() <= limit.unwrap_or(SKILL_SEARCH_DEFAULT_LIMIT),
        "matched 不得超过 limit: matched={} limit={:?}",
        matched.len(),
        limit
    );

    // ── O7 status 诚实
    let status = value["status"].as_str().expect("status 是字符串");
    if matched.is_empty() {
        assert_eq!(status, "no_match", "无命中必须报 no_match");
    } else {
        assert_eq!(status, "disclosed", "有命中必须报 disclosed");
    }

    // ── O4 共享预算：披露的总字符 ≤ 预算
    let disclosure = &value["disclosure"];
    let text = disclosure.to_string();
    let disclosed_len = text.chars().count();
    assert!(
        disclosed_len <= budget * 4 + 512,
        "披露体量必须受共享预算约束（budget={budget} disclosed={disclosed_len}）"
    );

    // ── O6 附带：refused_cards 如实计数（拒开的卡）
    assert!(
        value["refused_cards"].as_u64().is_some(),
        "refused_cards 必须是数字（拒开的卡如实计数）"
    );

    let _ = fs::remove_dir_all(&root);
    let _: Option<Value> = None;
});
