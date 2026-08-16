//! Example: Build a RealtimeSessionConfig and serialize it as a
//! session.update client event. No HTTP / WebSocket calls; this is the
//! pure schema layer.

use apeireth_voice::realtime::{
    ClientEvent, RealtimeModel, RealtimeSessionConfig, RealtimeTool, RealtimeVoice,
};
use serde_json::json;

fn main() {
    let cfg = RealtimeSessionConfig::new()
        .model(RealtimeModel::GptRealtimeMini)
        .voice(RealtimeVoice::Sage)
        .instructions("You are a concise voice assistant. Reply in 1-2 sentences.")
        .temperature(0.7)
        .add_tool(RealtimeTool::function(
            "get_weather",
            "Look up current weather for a city",
            json!({
                "type": "object",
                "properties": {
                    "city": { "type": "string", "description": "City name" }
                },
                "required": ["city"]
            }),
        ))
        .metadata("session_origin", "apeireth-r153-demo");

    println!("Config: {:?}", cfg);

    let event = ClientEvent::SessionUpdate {
        config: cfg.clone(),
    };
    let json = serde_json::to_string_pretty(&event).unwrap();
    println!("session.update payload:");
    println!("{}", json);

    cfg.validate().expect("config should validate");
    println!("Validation: OK");
    println!("Tools registered: {}", cfg.tools.len());
}
