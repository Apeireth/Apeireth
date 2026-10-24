//! `apeireth-ui-demo` 集成冒烟：试点应用（审批卡）的装配级行为。

use apeireth_ui_demo::{approval_card, tree_to_text, DemoApp};

#[test]
fn approval_card_contains_expected_parts() {
    let text = tree_to_text(&approval_card("REQ-0001", "审批卡试点"));
    assert!(text.contains("ap-approval-card"), "实测文本：{text}");
    assert!(text.contains("审批卡试点"), "实测文本：{text}");
    assert!(text.contains("编号 REQ-0001"), "实测文本：{text}");
    assert!(text.contains("批准"), "实测文本：{text}");
}

#[test]
fn demo_app_reflects_the_approval_signal() {
    let app = DemoApp::new();
    assert!(tree_to_text(&app.render()).contains("待审批"));

    app.approve();
    app.flush();

    let text = tree_to_text(&app.render());
    assert!(text.contains("已批准"), "实测文本：{text}");
    assert_eq!(app.approval_count(), 1);
    assert!(app.approved().get());
}
