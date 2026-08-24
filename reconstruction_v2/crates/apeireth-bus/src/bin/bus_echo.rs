//! `bus_echo` — L2 pipe echo server (CI fix 2026-08 重建).
//!
//! 历史: round15-03 删除了本 bin (Windows 构建 0 绿), 导致 `l2_pipe_json_roundtrip`
//! 等集成测试失去 echo 子进程 (spawn 测试二进制不解析 `--bus-echo-json` → 超时)。
//! 重建为 `[[bin]] required-features = ["full-bus"]` (0 默认装, 0 影响 Windows 构建).
//!
//! v2 适配: l2 module 是 `#[cfg(unix)]`, Windows 上整个 main 走 stub.

#[cfg(unix)]
fn real_main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !apeireth_bus::l2::try_run_echo_server(&args) {
        eprintln!("bus_echo: 无法识别的参数: {args:?} (期望 --bus-echo-json / --bus-echo-msgpack)");
        std::process::exit(1);
    }
}

#[cfg(not(unix))]
fn real_main() {
    eprintln!("bus_echo: l2 pipe server is unix-only (Windows builds skip)");
    std::process::exit(0);
}

fn main() {
    real_main();
}
