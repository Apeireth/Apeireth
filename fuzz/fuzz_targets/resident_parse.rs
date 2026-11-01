//! fuzz: 常驻实例槽文件解析（`ResidentRecord::parse`）。
//!
//! **面**：读的是**本地文件内容**，但文件可能被外部进程写坏/伪造。
//! 它的承诺是「读不懂就大声失败，绝不当作健康声明」——所以判据是
//! **不 panic 且不把畸形内容静默解析成合法值**（后者难以在 fuzz 里断言，
//! 这里先钉住 panics；语义断言在单测里做）。
#![no_main]

use apeireth_gateway::resident::ResidentRecord;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let _ = ResidentRecord::parse(s);
});
