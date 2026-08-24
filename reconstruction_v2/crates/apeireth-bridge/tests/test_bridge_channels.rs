use apeireth_bridge::{DiscordBridge, TelegramBridge, OneBotBridge, GameVisionLoop, GameLoopConfig, BridgeDispatcher, InboundMessage};

#[test]
fn test_social_payload_formatting() {
    let discord = DiscordBridge::new("https://discord.com/api/webhooks/test", "Apeireth");
    let d_payload = discord.format_payload("Hello from Apeireth!");
    assert_eq!(d_payload.content, "Hello from Apeireth!");

    let tg = TelegramBridge::new("test_token");
    let tg_payload = tg.format_payload("12345", "Test TG message");
    assert_eq!(tg_payload.chat_id, "12345");

    let onebot_priv = OneBotBridge::format_private_msg(10001, "Hello QQ friend");
    assert_eq!(onebot_priv.action, "send_private_msg");
}

#[test]
fn test_game_vision_loop() {
    let mut game_loop = GameVisionLoop::new(GameLoopConfig::default());
    assert_eq!(game_loop.tick_decision(0.5), None);

    game_loop.start();
    let dark_action = game_loop.tick_decision(0.1);
    assert_eq!(dark_action, Some("place_torch".into()));

    let day_action = game_loop.tick_decision(0.8);
    assert_eq!(day_action, Some("mine_forward".into()));
}

#[test]
fn test_dispatcher() {
    let mut dispatcher = BridgeDispatcher::new();
    let msg = InboundMessage {
        channel: "discord".into(),
        sender_id: "u1".into(),
        sender_name: "Yinta".into(),
        content: "What is your status?".into(),
        timestamp_ms: 1000,
    };
    let routed = dispatcher.route_inbound(msg);
    assert!(routed.contains("[discord:Yinta]"));
    assert_eq!(dispatcher.total_dispatched, 1);
}
