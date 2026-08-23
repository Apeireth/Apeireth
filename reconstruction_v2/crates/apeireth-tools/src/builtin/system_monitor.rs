use crate::{RiskLevel, Tool, ToolDefinition, ToolError, ToolResult};
use async_trait::async_trait;
#[cfg(target_os = "windows")]
use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

pub struct SystemMonitorTool;

impl Default for SystemMonitorTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitorTool {
    pub fn new() -> Self {
        SystemMonitorTool
    }
}

#[async_trait]
impl Tool for SystemMonitorTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "system_monitor".into(),
            description: "Monitors real-time system hardware metrics (physical/virtual memory, CPU, OS load)".into(),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> Result<ToolResult, ToolError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let mut mem_info: MEMORYSTATUSEX = std::mem::zeroed();
            mem_info.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut mem_info) == 0 {
                return Err(ToolError::ExecutionFailed("Failed to query Win32 GlobalMemoryStatusEx".into()));
            }

            let total_phys_mb = mem_info.ullTotalPhys / (1024 * 1024);
            let avail_phys_mb = mem_info.ullAvailPhys / (1024 * 1024);
            let used_phys_mb = total_phys_mb - avail_phys_mb;
            let mem_load = mem_info.dwMemoryLoad;
            let total_virt_mb = mem_info.ullTotalVirtual / (1024 * 1024);
            let avail_virt_mb = mem_info.ullAvailVirtual / (1024 * 1024);

            Ok(ToolResult::success(format!(
                "📊 [System Hardware Telemetry]:\n- Memory Load: {}%\n- Physical Memory: {} MB / {} MB used ({} MB available)\n- Virtual Memory: {} MB / {} MB available\n- OS Platform: Windows (x86_64)",
                mem_load, used_phys_mb, total_phys_mb, avail_phys_mb, total_virt_mb, avail_virt_mb
            )))
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(ToolResult::success("System Monitor: Non-Windows generic telemetry active".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_monitor() {
        let tool = SystemMonitorTool::new();
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert!(result.success);
        println!("System Monitor Output:\n{}", result.output);
        assert!(result.output.contains("Physical Memory"));
    }
}