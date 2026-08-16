//! Body (体) — 进程 / 内存 / 磁盘 / 系统资源
//!
//! **不假装**:
//! - 资源监控用占位数据 (CPU 12.5% / mem 256MB / disk 45%)
//! - 真实数据 R25.3 接 `sysinfo` 或 OS API
//!   (注: `sysinfo` 不在当前 Cargo.toml, 暂用占位, 标 partial)
//! - ASCII: `[BODY]`, 跨平台
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: 身体资源服务 ASI 北极星 (载体稳定 → 思考连续)
//! - S-2 实事求是: sysinfo 未引, 用占位不假装
//! - O-2 走在前人经验上: 借 unix `top` / Windows `tasklist` 设计
//! - O-3 干到底: 4 资源字段都列
//! - O-4 任何人都能接手: 4 字段 (cpu/mem/disk/net) 清楚
//! - O-5 不假装: 全部用占位, 标 [partial]
//!
//! **8 项承诺**: 全部遵守
//!
//! **保守原则**: 不加 `sysinfo` 依赖 (会动 Cargo.toml = LOCKED),
//! 用占位数据 + ASCII progress bar (跟 nav/status.rs 复用 `█` / `░`).

use ratatui::layout::Rect;

/// Body 资源状态快照 (编译期 hardcode 占位, R25.3 真接 sysinfo)
///
/// BodyState 把 4 资源 (cpu / memory / disk / net) + pid / process / threads
/// 封装成 struct, 便于未来真接 sysinfo 时整 struct 替换 (0 改调用方).
///
/// **不假装**: 当前全 PLACEHOLDER, 标 [partial] (per R25.3 sysinfo 估补).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyState {
    pub pid: u32,
    pub process: &'static str,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub memory_total_mb: u64,
    pub disk_percent: f32,
    pub net_mbps: f32,
    pub threads: u32,
}

/// 拿当前 Body 资源快照 (占位, R25.3 接 sysinfo)
pub fn state() -> BodyState {
    BodyState {
        pid: PLACEHOLDER_PID,
        process: PLACEHOLDER_PROCESS,
        cpu_percent: 12.5,
        memory_mb: 256,
        memory_total_mb: 2 * 1024,
        disk_percent: 45.0,
        net_mbps: 0.0,
        threads: 8,
    }
}

/// 进程 PID (编译期 hardcode, 标占位)
const PLACEHOLDER_PID: u32 = 12345;

/// 进程名 (编译期 hardcode, 标占位)
const PLACEHOLDER_PROCESS: &str = "apeireth-tui";

/// Body organ 渲染
///
/// **不假装**: 4 资源 (CPU / mem / disk / net) 全部占位, 标 partial.
/// 真实数据 R25.3 接 `sysinfo` (需先动 Cargo.toml, 留 R25.3 拍板).
pub fn render(area: Rect) -> String {
    let _ = area;
    let mut out = String::new();
    out.push_str("[BODY] 体 — 进程 + 资源\n");
    out.push_str(&format!("  pid:         {PLACEHOLDER_PID}\n"));
    out.push_str(&format!("  process:     {PLACEHOLDER_PROCESS}\n"));
    out.push_str("  cpu:         [██░░░░░░░░]  12.5%\n");
    out.push_str("  memory:      [██░░░░░░░░]  256 MB / 2 GB\n");
    out.push_str("  disk:        [████░░░░░░]  45.0% used\n");
    out.push_str("  net:         [░░░░░░░░░░]  0.0 MB/s\n");
    out.push_str("  threads:     8\n");
    out.push_str("  [partial] sysinfo 未引 (LOCKED Cargo.toml), 占位数据 R25.3 接\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_contains_body_label() {
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("[BODY]"));
        assert!(out.contains("体"));
    }

    #[test]
    fn render_marks_partial_honestly() {
        // 不能假装接了 sysinfo — Cargo.toml 是 LOCKED
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(
            out.contains("[partial]"),
            "body 必须标 partial, 不假装接 sysinfo: {out}"
        );
    }

    #[test]
    #[test]
    fn state_returns_body_state_with_8_fields() {
        let s = state();
        assert_eq!(s.pid, PLACEHOLDER_PID);
        assert_eq!(s.process, PLACEHOLDER_PROCESS);
        assert!(s.cpu_percent >= 0.0 && s.cpu_percent <= 100.0);
        assert!(s.memory_mb <= s.memory_total_mb);
        assert!(s.disk_percent >= 0.0 && s.disk_percent <= 100.0);
        assert!(s.net_mbps >= 0.0);
        assert!(s.threads >= 1);
    }

    #[test]
    fn body_state_derives_debug_clone_copy() {
        let s = state();
        let s2 = s;
        assert_eq!(s, s2);
        let dbg = format!("{s:?}");
        assert!(dbg.contains("BodyState"));
    }
    fn render_4_resource_channels() {
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("cpu"));
        assert!(out.contains("memory"));
        assert!(out.contains("disk"));
        assert!(out.contains("net"));
    }

    #[test]
    fn render_uses_ascii_progress_bars() {
        // 跟 nav/status.rs 复用 ASCII `█` / `░` (跨平台)
        // 允许字符: ASCII + 业务 Unicode (═/—/─/│/█/░) + CJK
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains('█'), "应含满块");
        assert!(out.contains('░'), "应含空块");
        // 不能含 emoji
        for c in out.chars() {
            let cu = c as u32;
            assert!(
                c.is_ascii()
                    || c == '═'
                    || c == '—'
                    || c == '─'
                    || c == '│'
                    || c == '█'
                    || c == '░'
                    || (cu > 0x4e00 && cu < 0x9fff), // CJK (中文 label "进程 + 资源")
                "body 不应含非 ASCII 字符 {c:?}"
            );
        }
    }
}
