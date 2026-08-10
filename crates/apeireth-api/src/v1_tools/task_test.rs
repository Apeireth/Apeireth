//! `v1_tools/task_test` — task 6 测试函数 unit test library
//!
//! **目的**: 给 R20 阶段 4 (D-01 真接细节) task 6 actions + 5 K-1 强校验
//! 加 6 测试, 由 `tests/test_v1_tools_unit_in_process.rs` 通过 `#[path]` 注入.
//!
//! **6 API 测试函数** (per 任务规范):
//! 1. `task_6_actions_e2e` — list / create / update / delete / get / complete 端到端
//! 2. `task_list_filter` — list filter (status / priority / assignee)
//! 3. `task_k1_title_status_priority` — K-1-1 title 非空 + K-1-2 status 枚举 + K-1-3 priority 0-3
//! 4. `task_k1_due_date_iso8601` — K-1-4 due_date YYYY-MM-DD
//! 5. `task_k1_assignee_email` — K-1-5 assignee 邮箱
//! 6. `task_complete_idempotent_and_errors` — complete 幂等 + 错误路径 + 编译期 hardcode
//!
//! **5 K-1 强校验** (per `task.rs` 头部 + 任务规范):
//! 1. **title 非空** — 拒空字符串 + 上限 512
//! 2. **status 枚举** — 4 值: pending / in_progress / done / cancelled
//! 3. **priority 0-3** — 0/1/2/3 (low/med/high/urgent)
//! 4. **due_date ISO 8601** — YYYY-MM-DD (date, 简化版, 不接 datetime)
//! 5. **assignee 邮箱** — 复用 `task::validate_email` (内联简化版)
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 6 actions 全部真实现 (非 stub 501)
//! - ✅ 5 K-1 强校验真跑
//! - ✅ storage trait 真用, InMemoryStorage 真跑
//! - ✅ task ID 用 UUID v4
//! - ✅ complete 真设 done_at ts (per ts 在 update 路径也跑)
//!
//! **6 哲学锚穿透**:
//! - 锚 #1 不漂移: 6 actions 真接, 5 K-1 真跑
//! - 锚 #2 编译期 hardcode: `TASK_ACTIONS_COUNT = 6` const assert
//! - 锚 #3 不引入 unsafe: `#![deny(unsafe_code)]` 继承
//! - 锚 #4 真值守门: storage trait NotFound 真返
//! - 锚 #5 复用 calendar 5 工具校验函数 (内联 validate_email 不依赖 LOCKED super::calendar)
//! - 锚 #6 工程铁律: 5 K-1 强校验
//!
//! **8 项不修改承诺 (严守)**:
//! - ❌ 不改 LOCKED `task.rs` (本文件 0 触碰)
//! - ❌ 不改 LOCKED `storage.rs` (本文件 0 触碰)
//! - ❌ 不改 LOCKED `mod.rs` (本文件用 `#[path]` 注入, 不动 mod.rs)
//! - ❌ 不改 workspace version (1.0.0)
//! - ❌ 不改 workspace Cargo.toml
//! - ❌ 不引第三方 DB / chrono 之外日期库
//! - ❌ 不假装支持完整 ISO 8601 (datetime / timezone, 留 R21)
//! - ❌ 不假装 task 已支持子任务 / 依赖 (留 R21)

#![deny(unsafe_code)]

// ============================================================
// 通过 #[path] 注入 storage + task 源文件
// ============================================================

/// **storage 源** — 注入 `src/v1_tools/storage.rs`
/// (命名必须用 `storage` 因为 task.rs 源文件 `use super::storage::{...}` / `super::storage::validate_id`
///  依赖模块名 `storage` 在 super 作用域内可见, `_storage_src` 会失败)
#[path = "storage.rs"]
mod storage;

/// **task 源** — 注入 `src/v1_tools/task.rs`
/// (本 crate mod.rs LOCKED 未 declare task, 用 #[path] 绕开)
#[path = "task.rs"]
mod _task_src;

// ============================================================
// `pub use super::invoke_by_name as invoke;` 桩
// ============================================================
//
// task.rs 源文件 line 505: `pub use super::invoke_by_name as invoke;`

#[allow(dead_code)]
pub async fn invoke_by_name() -> Result<(), String> {
    Ok(())
}

// ============================================================
// 6 测试函数 (per 任务规范)
// ============================================================

/// **6 测试函数总入口** — 由 `tests/test_v1_tools_unit_in_process.rs` 注入后
/// 通过 `#[tokio::test]` 调每个入口.
pub mod entries {
    #[allow(unused_imports)]
    use super::storage; // 让 _task_src 内 `super::storage` 引用可见
    use super::_task_src::{
        validate_due_date, validate_email, validate_priority, validate_status, Task, TaskStatus,
        TaskTool, TASK_ACTIONS, TASK_K1_CHECKS,
    };
    // Tool trait 必须 in scope, 否则 TaskTool::call 方法找不到
    use apeireth_tool_registry::Tool;
    use serde_json::json;

    /// 1. 6 actions e2e — list / create / update / delete / get / complete 端到端
    pub async fn task_6_actions_e2e() {
        let t = TaskTool::new();

        // 1.1 list empty
        let r = t.call(json!({"action": "list"})).await.expect("list");
        assert_eq!(r["count"], 0);
        assert!(r["tasks"].as_array().unwrap().is_empty());

        // 1.2 create
        let r = t
            .call(json!({
                "action": "create",
                "title": "Write spec",
                "description": "R20 阶段 4 spec",
                "status": "pending",
                "priority": 2,
                "due_date": "2026-12-31",
                "assignee": "alice@example.com"
            }))
            .await
            .expect("create");
        let tid = r["task_id"].as_str().expect("task_id").to_string();
        assert!(!tid.is_empty(), "task_id 应非空 (UUID v4)");

        // 1.3 get
        let r = t
            .call(json!({"action": "get", "id": tid}))
            .await
            .expect("get");
        assert_eq!(r["task"]["title"], "Write spec");
        assert_eq!(r["task"]["status"], "pending");
        assert_eq!(r["task"]["priority"], 2);
        assert_eq!(r["task"]["due_date"], "2026-12-31");
        assert_eq!(r["task"]["assignee"], "alice@example.com");
        assert!(r["task"]["created_at"].as_i64().unwrap() > 0);
        assert!(r["task"]["done_at"].is_null());

        // 1.4 update (改 priority 2 → 3 + status → in_progress)
        let r = t
            .call(json!({
                "action": "update", "id": tid,
                "priority": 3,
                "status": "in_progress",
                "description": "Updated desc"
            }))
            .await
            .expect("update");
        assert_eq!(r["ok"], true);
        assert_eq!(r["updated"], tid);

        let r = t
            .call(json!({"action": "get", "id": tid}))
            .await
            .expect("get after update");
        assert_eq!(r["task"]["priority"], 3, "priority 应被 update");
        assert_eq!(r["task"]["status"], "in_progress", "status 应被 update");
        assert_eq!(r["task"]["description"], "Updated desc");

        // 1.5 complete
        let r = t
            .call(json!({"action": "complete", "id": tid}))
            .await
            .expect("complete");
        assert_eq!(r["ok"], true);
        assert_eq!(r["id"], tid);
        assert_eq!(r["status"], "done");
        assert!(r["done_at"].as_i64().unwrap() > 0);

        let r = t
            .call(json!({"action": "get", "id": tid}))
            .await
            .expect("get after complete");
        assert_eq!(r["task"]["status"], "done");
        assert!(r["task"]["done_at"].as_i64().is_some());

        // 1.6 delete
        let r = t
            .call(json!({"action": "delete", "id": tid}))
            .await
            .expect("delete");
        assert_eq!(r["ok"], true);
        assert_eq!(r["deleted"], tid);

        // 1.7 delete 后 get 应 NotFound
        let r = t.call(json!({"action": "get", "id": tid})).await;
        assert!(r.is_err(), "delete 后 get 应 Err (NotFound)");

        // 1.8 delete 不存在 id 应 Err
        let r = t.call(json!({"action": "delete", "id": "nonexistent"})).await;
        assert!(r.is_err(), "delete 不存在应 Err");
        assert!(r.unwrap_err().contains("not found"));

        // 1.9 complete 不存在 id 应 Err
        let r = t.call(json!({"action": "complete", "id": "nonexistent"})).await;
        assert!(r.is_err(), "complete 不存在应 Err");
    }

    /// 2. list filter (status / priority / assignee)
    pub async fn task_list_filter() {
        let t = TaskTool::new();
        // 编 5 个 task
        let fixture: [(&str, &str, i64, &str); 5] = [
            ("a", "pending", 0, "alice@example.com"),
            ("b", "pending", 1, "bob@example.com"),
            ("c", "in_progress", 2, "alice@example.com"),
            ("d", "done", 3, "carol@example.com"),
            ("e", "cancelled", 1, "alice@example.com"),
        ];
        for (title, status, prio, assignee) in fixture.iter() {
            t.call(json!({
                "action": "create",
                "title": title,
                "status": status,
                "priority": prio,
                "assignee": assignee
            }))
            .await
            .expect("create");
        }

        // filter status=pending → 2 (a + b)
        let r = t
            .call(json!({"action": "list", "filter": {"status": "pending"}}))
            .await
            .expect("list pending");
        assert_eq!(r["count"], 2, "filter status=pending 应 2 个");

        // filter priority=1 → 2 (b + e)
        let r = t
            .call(json!({"action": "list", "filter": {"priority": 1}}))
            .await
            .expect("list prio 1");
        assert_eq!(r["count"], 2, "filter priority=1 应 2 个");

        // filter assignee=alice → 3 (a + c + e)
        let r = t
            .call(json!({"action": "list", "filter": {"assignee": "alice@example.com"}}))
            .await
            .expect("list alice");
        assert_eq!(r["count"], 3, "filter assignee=alice 应 3 个");

        // 多 filter: status=pending + priority=1 → 1 (b)
        let r = t
            .call(json!({"action": "list", "filter": {"status": "pending", "priority": 1}}))
            .await
            .expect("list pending+prio1");
        assert_eq!(r["count"], 1, "filter pending+prio1 应 1 个 (b)");

        // 无 filter → 5
        let r = t.call(json!({"action": "list"})).await.expect("list all");
        assert_eq!(r["count"], 5, "无 filter 应 5 个");
    }

    /// 3. K-1-1 title + K-1-2 status + K-1-3 priority
    pub async fn task_k1_title_status_priority() {
        let t = TaskTool::new();

        // K-1-1: title 空 → Err
        let r = t.call(json!({"action": "create", "title": ""})).await;
        assert!(r.is_err(), "空 title 应 Err (K-1-1)");
        assert!(r.unwrap_err().contains("title"), "err 应含 'title'");

        // K-1-1: title 缺 → Err
        let r = t.call(json!({"action": "create"})).await;
        assert!(r.is_err(), "缺 title 应 Err");
        assert!(r.unwrap_err().contains("title"));

        // K-1-1: title 超长 → Err (> 512)
        let long_title = "x".repeat(513);
        let r = t
            .call(json!({"action": "create", "title": long_title}))
            .await;
        assert!(r.is_err(), "超长 title 应 Err");

        // K-1-2: status 不在枚举 → Err
        let r = t
            .call(json!({
                "action": "create", "title": "x", "status": "weird"
            }))
            .await;
        assert!(r.is_err(), "status 'weird' 应 Err (K-1-2)");
        let err = r.unwrap_err();
        assert!(err.contains("status"), "err 应含 'status': {err}");

        // K-1-2: 4 枚举值都通过
        for status in ["pending", "in_progress", "done", "cancelled"] {
            let r = t
                .call(json!({"action": "create", "title": format!("t-{status}"), "status": status}))
                .await
                .unwrap_or_else(|e| panic!("status {status} 应 Ok, got: {e}"));
            assert!(r["task_id"].is_string());
        }

        // K-1-3: priority 越界 → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "priority": 99}))
            .await;
        assert!(r.is_err(), "priority 99 应 Err (K-1-3)");
        assert!(r.unwrap_err().contains("priority"));

        // K-1-3: priority 负 → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "priority": -1}))
            .await;
        assert!(r.is_err(), "priority -1 应 Err (K-1-3)");

        // K-1-3: priority 4 越界 → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "priority": 4}))
            .await;
        assert!(r.is_err(), "priority 4 应 Err (K-1-3, 上界 3)");

        // K-1-3: priority 0/1/2/3 都通过
        for prio in 0i64..=3 {
            let r = t
                .call(json!({"action": "create", "title": format!("p-{prio}"), "priority": prio}))
                .await
                .unwrap_or_else(|e| panic!("priority {prio} 应 Ok, got: {e}"));
            assert!(r["task_id"].is_string());
        }

        // validate_status / validate_priority 函数单测
        assert!(validate_status("pending").is_ok());
        assert!(validate_status("in_progress").is_ok());
        assert!(validate_status("done").is_ok());
        assert!(validate_status("cancelled").is_ok());
        assert!(validate_status("weird").is_err());
        assert_eq!(TaskStatus::Pending.as_str(), "pending");
        assert_eq!(TaskStatus::InProgress.as_str(), "in_progress");
        assert_eq!(TaskStatus::Done.as_str(), "done");
        assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
        assert_eq!(TaskStatus::ALL.len(), 4);

        assert!(validate_priority(0).is_ok());
        assert!(validate_priority(3).is_ok());
        assert!(validate_priority(4).is_err());
        assert!(validate_priority(-1).is_err());
    }

    /// 4. K-1-4 due_date YYYY-MM-DD
    pub async fn task_k1_due_date_iso8601() {
        let t = TaskTool::new();

        // 缺 - → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "20261231"}))
            .await;
        assert!(r.is_err(), "due_date '20261231' 缺 '-' 应 Err (K-1-4)");
        assert!(r.unwrap_err().contains("due_date"));

        // 长度不对 → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-1-1"}))
            .await;
        assert!(r.is_err(), "due_date '2026-1-1' 长度 8 应 Err");

        // 月超界 → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-13-01"}))
            .await;
        assert!(r.is_err(), "due_date month 13 应 Err");
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-00-15"}))
            .await;
        assert!(r.is_err(), "due_date month 00 应 Err");

        // 日超界 → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-12-32"}))
            .await;
        assert!(r.is_err(), "due_date day 32 应 Err");
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-12-00"}))
            .await;
        assert!(r.is_err(), "due_date day 00 应 Err");

        // 含 datetime (留 R21) → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-12-31T10:00:00"}))
            .await;
        assert!(r.is_err(), "due_date 含 'T' datetime 应 Err (留 R21)");

        // 含非数字 → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": "2026-1a-31"}))
            .await;
        assert!(r.is_err(), "due_date 含字母应 Err");

        // 空 → Ok (可选)
        let r = t
            .call(json!({"action": "create", "title": "x", "due_date": ""}))
            .await
            .expect("create no due_date");
        assert!(r["task_id"].is_string());

        // 不传 → Ok (默认空)
        let r = t
            .call(json!({"action": "create", "title": "y"}))
            .await
            .expect("create default");
        assert!(r["task_id"].is_string());

        // 合法: YYYY-MM-DD 通过
        let r = t
            .call(json!({"action": "create", "title": "z", "due_date": "2026-12-31"}))
            .await
            .expect("create iso date");
        assert!(r["task_id"].is_string());

        // validate_due_date 函数单测
        assert!(validate_due_date("").is_ok());
        assert!(validate_due_date("2026-12-31").is_ok());
        assert!(validate_due_date("2026-01-01").is_ok());
        assert!(validate_due_date("2026-13-01").is_err());
        assert!(validate_due_date("20261231").is_err());
        assert!(validate_due_date("2026-12-31T10:00:00").is_err());
        assert!(validate_due_date("26-12-31").is_err()); // 年份 2 位
    }

    /// 5. K-1-5 assignee 邮箱
    pub async fn task_k1_assignee_email() {
        let t = TaskTool::new();

        // 不传 → Ok (默认空)
        let r = t
            .call(json!({"action": "create", "title": "x"}))
            .await
            .expect("create no assignee");
        assert!(r["task_id"].is_string());

        // 空字符串 → Ok (K-1 跳过)
        let r = t
            .call(json!({"action": "create", "title": "x", "assignee": ""}))
            .await
            .expect("create empty assignee");
        assert!(r["task_id"].is_string());

        // 非空 + 错格式 → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "assignee": "bad"}))
            .await;
        assert!(r.is_err(), "assignee 'bad' 应 Err (K-1-5)");
        let err = r.unwrap_err();
        assert!(err.contains("email") || err.contains("@"), "err: {err}");

        // 非空 + 缺 @ → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "assignee": "noatsign"}))
            .await;
        assert!(r.is_err(), "assignee 'noatsign' 应 Err");

        // 非空 + 缺 . → Err
        let r = t
            .call(json!({"action": "create", "title": "x", "assignee": "a@b"}))
            .await;
        assert!(r.is_err(), "assignee 'a@b' 缺 '.' 应 Err");

        // 合法 → Ok
        let r = t
            .call(json!({"action": "create", "title": "x", "assignee": "a@example.com"}))
            .await
            .expect("create good assignee");
        assert!(r["task_id"].is_string());

        // update 改 assignee → 校验也跑
        let tid = r["task_id"].as_str().unwrap().to_string();
        let r = t
            .call(json!({"action": "update", "id": tid, "assignee": "bad"}))
            .await;
        assert!(r.is_err(), "update assignee 'bad' 应 Err");

        // validate_email 函数单测
        assert!(validate_email("a@b.c").is_ok());
        assert!(validate_email("").is_err());
        assert!(validate_email("no_at").is_err());
        assert!(validate_email("@b.c").is_err());
    }

    /// 6. complete 幂等 + 错误路径 + 编译期 hardcode
    pub async fn task_complete_idempotent_and_errors() {
        let t = TaskTool::new();

        // 编一个 task
        let r = t
            .call(json!({"action": "create", "title": "idempotent-test"}))
            .await
            .expect("create");
        let tid = r["task_id"].as_str().unwrap().to_string();

        // complete 第 1 次
        let r1 = t
            .call(json!({"action": "complete", "id": tid}))
            .await
            .expect("complete 1");
        assert_eq!(r1["status"], "done");
        let done_at_1 = r1["done_at"].as_i64().unwrap();

        // complete 第 2 次 (幂等: status=done, done_at 更新到新 ts)
        // (per 锚 #1 不假装, 真更新 done_at, 保持幂等终态)
        let r2 = t
            .call(json!({"action": "complete", "id": tid}))
            .await
            .expect("complete 2");
        assert_eq!(r2["status"], "done", "complete 幂等: 仍 done");
        let done_at_2 = r2["done_at"].as_i64().unwrap();
        // 第 2 次 done_at ≥ 第 1 次 (ts 单调)
        assert!(done_at_2 >= done_at_1, "complete 幂等: done_at 升序");

        // update 改 status=in_progress → done_at 应清空
        let r = t
            .call(json!({"action": "update", "id": tid, "status": "in_progress"}))
            .await
            .expect("update undo done");
        assert_eq!(r["ok"], true);
        let r = t
            .call(json!({"action": "get", "id": tid}))
            .await
            .expect("get after undo");
        assert_eq!(r["task"]["status"], "in_progress");
        assert!(
            r["task"]["done_at"].is_null(),
            "in_progress 应清空 done_at"
        );

        // 错误路径
        assert!(t.call(json!({})).await.is_err(), "缺 action 应 Err");
        assert!(t.call(json!({"action": "unknown"})).await.is_err(), "错 action 应 Err");
        assert!(t.call(json!({"action": "get"})).await.is_err(), "get 缺 id 应 Err");
        assert!(t.call(json!({"action": "update", "title": "x"})).await.is_err(), "update 缺 id 应 Err");
        assert!(t.call(json!({"action": "delete", "id": "x"})).await.is_err(), "delete 不存在应 Err");
        assert!(t.call(json!({"action": "complete", "id": "x"})).await.is_err(), "complete 不存在应 Err");

        // 编译期 hardcode
        assert_eq!(TASK_ACTIONS.len(), 6, "6 actions");
        assert_eq!(TASK_K1_CHECKS.len(), 5, "5 K-1 强校验");
        assert_eq!(TASK_ACTIONS[0], "list");
        assert_eq!(TASK_ACTIONS[5], "complete");
        assert_eq!(TASK_K1_CHECKS[0], "title_not_empty");
        assert_eq!(TASK_K1_CHECKS[4], "assignee_email");
    }
}
