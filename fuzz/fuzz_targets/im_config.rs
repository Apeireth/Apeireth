//! fuzz: IM 渠道配置解析（`ImChannelConfig::from_json_value`）。
//!
//! **面**：配置来自 env 变量或数据目录文件 —— 文本可控，且它的承诺是
//! **fail-closed**（任何缺陷都是错误，0 回退默认）。所以判据有两条：
//! 1. 不 panic；
//! 2. **不得因畸形输入回退到「空配置=看似正常」** —— 那会把 fail-closed 变成 fail-open。
//!    断言：返回 `Ok` 时，输入必须是它认得的两种形状之一（数组 / `{"channels": [...]}`），
//!    否则说明它有静默兜底。
#![no_main]

use apeireth_gateway::im_channels::ImChannelConfig;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    let Ok(value) = serde_json::from_slice::<Value>(data) else {
        return;
    };
    let is_array = value.is_array();
    let is_wrapped = value
        .as_object()
        .map(|o| o.get("channels").map(|c| c.is_array()).unwrap_or(false))
        .unwrap_or(false);

    if ImChannelConfig::from_json_value(value).is_ok() {
        assert!(
            is_array || is_wrapped,
            "from_json_value 对既非数组也非 {{channels:[...]}} 的输入返回了 Ok —— fail-closed 被破坏"
        );
    }
});
