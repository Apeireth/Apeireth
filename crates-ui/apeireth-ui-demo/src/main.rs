//! 试点应用入口：native 跑一遍审批卡渲染（打印虚拟树）。
//!
//! 诚实口径：这里打印的是**虚拟树文本**，不是真实 DOM —— 真实 DOM 后端
//! （web-sys）已在 P2 落地，但**未经任何运行时验证**（native 编译不参与），
//! 浏览器内对照（像素 / 逐帧）属 P3 验收门，浏览器 job 属 P5（待建）。

use apeireth_ui_demo::{tree_to_text, DemoApp};

/// 跑一遍试点流程，返回打印文本（入口与测试共用同一路径）。
fn run_demo() -> String {
    let app = DemoApp::new();
    let mut out = String::new();

    out.push_str("== 审批前 ==\n");
    out.push_str(&tree_to_text(&app.render()));

    app.approve();
    app.flush();

    out.push_str("\n\n== 审批后 ==\n");
    out.push_str(&tree_to_text(&app.render()));
    out.push_str(&format!("\n\n审批次数: {}\n", app.approval_count()));
    out
}

fn main() {
    print!("{}", run_demo());
}

#[cfg(test)]
mod tests {
    #[test]
    fn demo_output_shows_the_state_transition() {
        let text = super::run_demo();
        assert!(text.contains("待审批"), "实测输出：{text}");
        assert!(text.contains("已批准"), "实测输出：{text}");
        assert!(text.contains("审批次数: 1"), "实测输出：{text}");
    }
}
