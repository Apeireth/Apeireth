# Apeireth 2.0 开发者扩展性指南 (Extensibility Guide)

> **核心原则**：开放封闭原则（对扩展开放，对修改关闭） · 集成而非分立 · 机制而非补丁

在 Apeireth 1.0 中，新增功能往往导致微 Crate 肆意蔓延，造成了 85+ Crate 的碎片化灾难。Apeireth 2.0 通过清晰的 **Trait 驱动与插件化体系**，使得所有维度的扩展无需新建 Cargo Crate 即可优雅完成。

---

## 一、 如何扩展自定义工具（Tool）

在 2.0 中，无需新建 `apeireth-tool-xxx` 包，只需实现 `Tool` trait 并注册即可：

```rust
use async_trait::async_trait;
use apeireth_tools::{Tool, ToolDefinition, ToolResult, RiskLevel};
use serde_json::json;

pub struct WeatherTool;

#[async_trait]
impl Tool for WeatherTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_weather".to_string(),
            description: "查询指定城市的天气状况".to_string(),
            parameters_schema: json!({
                "type": "object",
                "properties": {
                    "city": { "type": "string", "description": "城市名称" }
                },
                "required": ["city"]
            }),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, String> {
        let city = params.get("city").and_then(|v| v.as_str()).unwrap_or("Beijing");
        Ok(ToolResult::success(json!({
            "city": city,
            "temp": "22C",
            "condition": "Sunny"
        })))
    }
}

// 注册工具
let mut registry = ToolRegistry::new();
registry.register(WeatherTool)?;
```

---

## 二、 如何扩展新的大模型协议（LLM Protocol Adapter）

若未来需要支持新的本地大模型（如 Ollama、Mistral 或专有网关），在 `apeireth-protocol` 中实现 `ProtocolAdapter`：

```rust
use apeireth_protocol::{ProtocolAdapter, NormalizedRequest, NormalizedResponse};

pub struct DeepSeekAdapter;

impl ProtocolAdapter for DeepSeekAdapter {
    fn adapt_request(&self, req: &NormalizedRequest) -> Result<serde_json::Value, String> {
        // 将内部统一结构转换为目标厂商 JSON
        Ok(serde_json::json!({
            "model": req.model,
            "messages": req.messages,
            "stream": req.stream
        }))
    }

    fn adapt_response(&self, raw: &serde_json::Value) -> Result<NormalizedResponse, String> {
        // 将厂商返回 JSON 解析为内部统一结构
        // ...
        Ok(NormalizedResponse::default())
    }
}
```

---

## 三、 如何扩展新的持久化后端（Storage Backend）

`apeireth-storage` 将存储契约抽象为 Trait：
- `MemoryStore`：会话、事实、笔记读写
- `GraphStore`：因果图与实体三元组遍历
- `VectorIndex`：向量嵌入与相似度召回

实现对应的 Trait 即可无缝将底层的 SQLite 替换或平滑迁移至 PostgreSQL、Qdrant 或分布式存储，上层 `apeireth-companion` 零改动。

---

## 四、 如何添加新的 Crate（若必须新增顶级领域）

如果未来需要引入全新的大型领域（如独立的 `apeireth-vision` 视觉处理引擎）：
1. 在 `reconstruction_v2/Cargo.toml` 的 `[workspace.members]` 中添加 `"crates/apeireth-vision"`；
2. 内部依赖仅需引用 `apeireth-core`（领域实体与事件总线）；
3. 通过 `EventBus` 与主系统解耦通信，禁止产生循环依赖；
4. 运行 `cargo check --workspace` 确保 0 警告通过。
