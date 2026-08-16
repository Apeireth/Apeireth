//! # `apeireth-lark` R20 阶段 6 flesh out: 真接飞书 Open API 5 端点 demo
//!
//! 演示 `LarkRealImpl` 真接飞书 Open API 5 端点 (auth/im/calendar/docx/bitable).
//! 跟 `lark_stub_demo` (STUB 模式) 严格分离, 是显式 opt-in 真接路径.
//!
//! **注意**: 本 demo 默认 `base_url` 指向 `https://open.feishu.cn/open-apis` (真飞书).
//! 真跑前需:
//! 1. 在飞书开发者后台创建应用, 拿 `app_id` + `app_secret`
//! 2. 通过环境变量 `LARK_APP_ID` / `LARK_APP_SECRET` 注入
//! 3. 给应用授权对应 API 权限 (im:message, calendar:calendar, docs:document, bitable:app)
//!
//! 不传环境变量时, demo 用 `LarkConfig::default()` 跑 STUB 默认值 (会 401).
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-lark --example lark_real_demo
//! # 或带环境变量:
//! LARK_APP_ID=cli_xxx LARK_APP_SECRET=xxx cargo run -p apeireth-lark --example lark_real_demo
//! ```
//!
//! ## 输出 (STUB 默认值, 无 env 时)
//!
//! ```text
//! [lark_real_demo] LarkRealImpl 创建: app_id=apeireth-stub-app-id base_url=https://open.feishu.cn/open-apis
//! [lark_real_demo] auth_refresh -> Err(AuthFailed("auth_refresh HTTP 401: ..."))
//! [lark_real_demo] send_message -> Err(AuthFailed("..."))
//! [lark_real_demo] list_calendars -> Err(AuthFailed("..."))
//! [lark_real_demo] 演示完成 (R20 阶段 6 flesh out 真接实现已 ready, Mavis 整合 #3 拍板后切 STUB_MODE=false)
//! ```
//!
//! ## 6 哲学锚穿透 (per 蓝图 §1)
//!
//! - **S-1**: 1:1 翻译飞书 Open API URL + 方法, 跟 `real.rs` 同款
//! - **S-2**: demo 不假装"调通", 无环境变量时如实返 401 错误
//! - **O-3**: 1 文件覆盖 5 端点调用入口
//! - **O-5**: 缺 app_id/app_secret 时如实 demo 失败, 不假装成功

use apeireth_lark::{
    LarkClient, LarkConfig, LarkError, LarkRealImpl, MessageType, LARK_API_BASE_URL,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) 配 LarkConfig (env > default)
    let app_id = std::env::var("LARK_APP_ID").unwrap_or_else(|_| "apeireth-stub-app-id".into());
    let app_secret =
        std::env::var("LARK_APP_SECRET").unwrap_or_else(|_| "apeireth-stub-app-secret".into());
    let cfg = LarkConfig {
        app_id,
        app_secret,
        base_url: LARK_API_BASE_URL.to_string(),
        token_cache_ttl_seconds: 7200,
    };

    println!(
        "[lark_real_demo] LarkRealImpl 创建: app_id={} base_url={}",
        cfg.app_id, cfg.base_url
    );

    // 2) 创建 LarkRealImpl
    let real = LarkRealImpl::new(cfg)?;

    // 3) 演示 5 端点 (无 env 时会 401, 实事求是)
    let r1 = real.auth_refresh().await;
    println!("[lark_real_demo] auth_refresh -> {:?}", short_err(&r1));

    // 4) 演示 send_message (鉴权失败时返 AuthFailed, 仍走完流程)
    let r2 = real
        .send_message("u_test", MessageType::Text, "hello from lark_real_demo")
        .await;
    println!("[lark_real_demo] send_message -> {:?}", short_err(&r2));

    // 5) 演示 list_calendars
    let r3 = real.list_calendars().await;
    println!("[lark_real_demo] list_calendars -> {:?}", short_err(&r3));

    // 6) 演示 create_event (start/end = 0 时返 ConfigInvalid, 不发 HTTP)
    let r4 = real.create_event("cal_x", "demo event", 0, 0).await;
    println!(
        "[lark_real_demo] create_event (start=0) -> {:?}",
        short_err(&r4)
    );

    // 7) 演示 get_document
    let r5 = real.get_document("doc_demo").await;
    println!("[lark_real_demo] get_document -> {:?}", short_err(&r5));

    // 8) 演示 search_documents (URL encode 测试)
    let r6 = real.search_documents("apeireth 计划", 10).await;
    println!("[lark_real_demo] search_documents -> {:?}", short_err(&r6));

    // 9) 演示 list_bitable_records
    let r7 = real.list_bitable_records("app_demo", "tbl_demo", 10).await;
    println!(
        "[lark_real_demo] list_bitable_records -> {:?}",
        short_err(&r7)
    );

    // 10) 演示 create_bitable_record
    let r8 = real
        .create_bitable_record(
            "app_demo",
            "tbl_demo",
            serde_json::json!({ "title": "demo task", "status": "todo" }),
        )
        .await;
    println!(
        "[lark_real_demo] create_bitable_record -> {:?}",
        short_err(&r8)
    );

    println!(
        "[lark_real_demo] 演示完成 (R20 阶段 6 flesh out 真接实现已 ready, Mavis 整合 #3 拍板后切 STUB_MODE=false)"
    );
    Ok(())
}

/// 短格式化错误 (不打印全 body, 防 noise).
fn short_err<T: std::fmt::Debug>(r: &Result<T, LarkError>) -> String {
    match r {
        Ok(_) => "Ok".to_string(),
        Err(e) => {
            // 只取前 80 字符, 防飞书返回大 body 撑爆 stdout
            let s = format!("{e:?}");
            if s.len() > 80 {
                format!("{}...(truncated)", &s[..80])
            } else {
                s
            }
        }
    }
}
