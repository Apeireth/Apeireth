use std::sync::Arc;
use apeireth_tools::{ToolRegistry, Tool};
use apeireth_tools::vision::{ScreenCapture, OmniParser, UiElement, UiElementType, DesktopActionTool};

use apeireth_tools::worktree::WorktreeSandbox;
use apeireth_tools::synthesis::ToolSynthesizer;
use apeireth_tools::sandbox::PlatformSandbox;

#[tokio::test]
async fn test_vision_som_and_screen_diffing() {
    // 1. Test Screen Perceptual Hashing & Difference Detection
    let mut capture = ScreenCapture::new(0.10);
    let mut frame_pattern_a = vec![0u8; 1920 * 1080];
    for i in 0..(1920 * 540) {
        frame_pattern_a[i] = 255;
    }
    let (f1, changed1) = capture.process_frame(&frame_pattern_a, 1920, 1080, 1000);
    assert!(changed1);

    // Identical frame -> No change
    let (_, changed2) = capture.process_frame(&frame_pattern_a, 1920, 1080, 1033);
    assert!(!changed2, "Identical frame should trigger 0 diff");

    // Inverted pattern frame -> High change
    let mut frame_pattern_b = vec![255u8; 1920 * 1080];
    for i in 0..(1920 * 540) {
        frame_pattern_b[i] = 0;
    }
    let (f3, changed3) = capture.process_frame(&frame_pattern_b, 1920, 1080, 1066);
    assert!(changed3);
    assert_ne!(f1.perceptual_hash, f3.perceptual_hash);


    // 2. Test OmniParser Set-of-Marks (SoM) Tree Generation
    let elements = vec![
        UiElement {
            id: 1,
            element_type: UiElementType::Button,
            label: "Submit Order".into(),
            bbox: [0.1, 0.2, 0.15, 0.05],
            is_interactive: true,
        },
        UiElement {
            id: 2,
            element_type: UiElementType::InputBox,
            label: "Search Codebase".into(),
            bbox: [0.3, 0.1, 0.4, 0.04],
            is_interactive: true,
        },
    ];

    let parsed = OmniParser::parse_screen(elements, 1920, 1080);
    assert!(parsed.som_markup_text.contains("[#1] Button: \"Submit Order\""));
    assert!(parsed.som_markup_text.contains("[#2] InputBox: \"Search Codebase\""));

    let (cx, cy) = OmniParser::resolve_mark_center(&parsed.elements[0], 1920, 1080);
    assert_eq!(cx, 336); // (0.1 + 0.075) * 1920 = 336
    assert_eq!(cy, 243); // (0.2 + 0.025) * 1080 = 243

    // 3. Test Desktop Action Tool
    let action_tool = DesktopActionTool::new(1920, 1080);
    let click_res = action_tool.execute(serde_json::json!({
        "action": "click",
        "x": cx,
        "y": cy,
        "button": "left"
    })).await.unwrap();
    assert!(click_res.success);
    assert!(click_res.output.contains("Executed mouse click 'left' at (336, 243)"));
}

#[tokio::test]
async fn test_worktree_and_tool_synthesis() {
    // 1. Test Worktree Immutable PatchSet Hashing
    let diff = "diff --git a/main.rs b/main.rs\n+println!(\"Hello Sandboxed Worktree\");\n";
    let patch = WorktreeSandbox::create_patch_set(
        "task/hotfix-worktree",
        vec!["src/main.rs".into()],
        diff.into(),
        None,
    );
    assert!(patch.patch_id.starts_with("patch_"));
    assert_eq!(patch.content_sha256.len(), 64);

    // 2. Test Autonomous Tool Synthesizer
    let sandbox = Arc::new(PlatformSandbox::new().unwrap());
    let synthesizer = ToolSynthesizer::new(sandbox);
    let mut reg = ToolRegistry::new();

    let tool_name = synthesizer.synthesize_and_register(
        "json_prettifier",
        "Synthesized tool to prettify JSON",
        "powershell",
        "Write-Output 'json_prettifier output ok'",
        &mut reg,
    ).unwrap();

    assert_eq!(tool_name, "json_prettifier");
    let res = reg.execute("json_prettifier", serde_json::json!({"payload": "{\"key\":123}"})).await.unwrap();
    assert!(res.success);
    assert!(res.output.contains("json_prettifier"));
}

