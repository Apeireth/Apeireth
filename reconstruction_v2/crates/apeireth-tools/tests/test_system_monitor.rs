use apeireth_tools::builtin::system_monitor::SystemMonitorTool;
use apeireth_tools::Tool;
use serde_json::json;

#[tokio::test]
async fn test_system_monitor_live_telemetry() {
    let tool = SystemMonitorTool::new();
    let res = tool.execute(json!({})).await.expect("Failed to execute SystemMonitorTool");
    assert!(res.success);
    println!("Live System Hardware Telemetry Output:\n{}", res.output);
    assert!(res.output.contains("Physical Memory"));
    assert!(res.output.contains("Memory Load"));
}
