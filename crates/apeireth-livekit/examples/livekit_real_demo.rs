//! # `apeireth-livekit` R20 阶段 6 flesh out: 真接 demo
//!
//! **本 demo 是 R20 阶段 6 flesh out 新增**, 演示 6 端点真接实现
//! (server_url / api_key / room / track / participant / event).
//!
//! 跟 `lib.rs` 现状 STUB 路径 (`TOOL_WHITELIST` 6 端点 + 1 stub_status, 编译期
//! hardcode 守门 `STUB_MODE = true`) 严格分离.
//!
//! ## 设计 (per LiveKit Server API Twirp 协议 + 主人 2026-08-06 派活)
//!
//! **8 演示入口** (跟 voice / lark / sandbox 1:1 模式):
//! 1. **server_url** — `get_server_url()` 真接 (getter, 不走 HTTP)
//! 2. **api_key** — `get_api_key()` 真接 (getter, 不暴露 secret)
//! 3. **room.create** — `create_room()` 真接 Twirp POST
//! 4. **room.list** — `list_rooms()` 真接 Twirp POST
//! 5. **room.delete** — `delete_room()` 真接 Twirp POST
//! 6. **track.mute** — `mute_track()` 真接 Twirp POST
//! 7. **participant.list / remove** — `list_participants` + `remove_participant` 真接
//! 8. **event.push / drain** — `push_event` + `drain_events` in-memory 模拟
//!
//! ## 0 真连 LiveKit server
//!
//! 跟 voice / lark / sandbox 1:1 模式 (per 任务 spec: 本机可能没启 LiveKit server):
//! - `server_url` 指向 `http://127.0.0.1:1` (无 server, 必然 connect 失败)
//! - 演示 "真请求" 失败但**架构就绪**, 跟 voice 1:1 "AuthFailed" 模式
//! - 集成时只换 server_url + api_key + api_secret 即可
//!
//! ## 6 哲学锚穿透 (跟 voice / lark / sandbox 1:1 模式)
//!
//! - **S-1 北极星**: 1:1 翻译 LiveKit Server API Twirp 6 维度, 0 假装"已连真 LiveKit server"
//! - **S-2 实事求是**: 演示输出"真尝试"结果, 不假装成功
//! - **O-2 走在前人肩上**: 8 演示入口简洁, 1 屏可读
//! - **O-3 干到底**: 6 端点 × 8 演示入口, 信息密度高
//! - **O-4 任何人都能接手**: 单一 main fn, 0 共享状态
//! - **O-5 不假装**: event 端点 in-memory 模拟显式标缺
//!
//! ## 8 项不修改承诺 守门 (跟 voice / lark / sandbox 1:1 模式)
//!
//! - **#1 不假装已实现**: 6 端点真发 HTTP, 失败如实报
//! - **#2 编译期 hardcode**: 6 端点名 / 5 K-1 / Twirp 路径前缀仍 hardcode
//! - **#3 不改 LOCKED**: 0 改 24 LOCKED crate + 0 改 `apeireth-sdk-livekit` LOCKED baseline
//! - **#4 不改 workspace version**: `version = "0.1.0"` 沿用
//! - **#5 6 哲学锚穿透**: 上 6 行
//! - **#6 不依赖 NewAPI**: 0 引外部 RPC
//! - **#7 不重复造轮子**: 沿用 workspace 已有 reqwest / tokio / serde / jsonwebtoken
//! - **#8 诚实标缺**: event 端点 in-memory 模拟显式标缺

use apeireth_livekit::real::LiveKitRealImpl;
use apeireth_livekit::{
    CreateRoomRequest, DeleteRoomRequest, ListParticipantsRequest, MuteTrackRequest, RemoveParticipantRequest,
    Room, WebhookEvent, DEFAULT_LIVEKIT_SERVER_URL, DEFAULT_TOKEN_TTL_SECONDS, PLATFORM_NAME,
};

#[tokio::main]
async fn main() {
    println!("[livekit_real_demo] apeireth-livekit 真接实现 demo (R20 阶段 6 flesh out)");
    println!(
        "[livekit_real_demo] server_url={DEFAULT_LIVEKIT_SERVER_URL} (本地 mock, 0 真连 LiveKit server, 跟 voice 1:1 模式)"
    );
    println!("[livekit_real_demo] token_ttl={DEFAULT_TOKEN_TTL_SECONDS}s platform={PLATFORM_NAME}");
    println!();

    // 构造 LiveKitRealImpl (K-1 强校验 server_url + api_key 通过)
    let real = LiveKitRealImpl::new(
        apeireth_livekit::LiveKitConfig::default(),
        "http://127.0.0.1:1", // 0 真连, 跟 voice 1:1 模式
        "APIabc123def456ghi789",
        "secret_xxx_32_chars_xxxxx",
    )
    .expect("LiveKitRealImpl::new K-1 OK");

    // 演示 1: server_url getter
    println!("[demo 1/8] server_url getter (1:1 翻译 server_url 端点)");
    match real.get_server_url().await {
        Ok(url) => println!("[livekit_real_demo] server_url -> \"{url}\""),
        Err(e) => println!("[livekit_real_demo] server_url -> \"{e}\""),
    }

    // 演示 2: api_key getter (不暴露 secret)
    println!("\n[demo 2/8] api_key getter (1:1 翻译 api_key 端点, 不暴露 secret)");
    match real.get_api_key().await {
        Ok(key) => {
            let prefix = key.chars().take(6).collect::<String>();
            println!("[livekit_real_demo] api_key -> \"{prefix}...\" (前 6 字符)")
        }
        Err(e) => println!("[livekit_real_demo] api_key -> \"{e}\""),
    }

    // 演示 3: room.create (Twirp POST /twirp/livekit.RoomService/CreateRoom)
    println!("\n[demo 3/8] room.create (Twirp POST CreateRoom, 1:1 翻译 room 端点)");
    let cr_req = match CreateRoomRequest::new("demo-room") {
        Ok(r) => r,
        Err(e) => {
            println!("[livekit_real_demo] create_room -> \"K-1 强校验失败: {e}\"");
            return; // K-1 强校验失败, 早返
        }
    };
    match real.create_room(cr_req).await {
        Ok(room) => println!("[livekit_real_demo] create_room -> \"Ok(sid={}, name={})\"", room.sid, room.name),
        Err(e) => println!("[livekit_real_demo] create_room -> \"{e}\""),
    }

    // 演示 4: room.list (Twirp POST /twirp/livekit.RoomService/ListRooms)
    println!("\n[demo 4/8] room.list (Twirp POST ListRooms)");
    match real.list_rooms().await {
        Ok(resp) => println!("[livekit_real_demo] list_rooms -> \"Ok(rooms={})\"", resp.rooms.len()),
        Err(e) => println!("[livekit_real_demo] list_rooms -> \"{e}\""),
    }

    // 演示 5: room.delete (Twirp POST /twirp/livekit.RoomService/DeleteRoom)
    println!("\n[demo 5/8] room.delete (Twirp POST DeleteRoom, K-1 强校验 room name)");
    let dr_req = match DeleteRoomRequest::new("demo-room") {
        Ok(r) => r,
        Err(e) => {
            println!("[livekit_real_demo] delete_room -> \"K-1 强校验失败: {e}\"");
            return;
        }
    };
    match real.delete_room(dr_req).await {
        Ok(_) => println!("[livekit_real_demo] delete_room -> \"Ok\""),
        Err(e) => println!("[livekit_real_demo] delete_room -> \"{e}\""),
    }

    // 演示 6: track.mute (Twirp POST /twirp/livekit.RoomService/MutePublishedTrack)
    println!("\n[demo 6/8] track.mute (Twirp POST MutePublishedTrack, K-1 强校验 room+identity+track_sid)");
    let mt_req = match MuteTrackRequest::new("demo-room", "user-1", "TR_abc123def456", true) {
        Ok(r) => r,
        Err(e) => {
            println!("[livekit_real_demo] mute_track -> \"K-1 强校验失败: {e}\"");
            return;
        }
    };
    match real.mute_track(mt_req).await {
        Ok(_) => println!("[livekit_real_demo] mute_track -> \"Ok\""),
        Err(e) => println!("[livekit_real_demo] mute_track -> \"{e}\""),
    }

    // 演示 7: participant.list + participant.remove (Twirp POST)
    println!("\n[demo 7/8] participant.list + participant.remove (Twirp POST, K-1 强校验 identity)");
    let lp_req = match ListParticipantsRequest::new("demo-room") {
        Ok(r) => r,
        Err(e) => {
            println!("[livekit_real_demo] list_participants -> \"K-1 强校验失败: {e}\"");
            return;
        }
    };
    match real.list_participants(lp_req).await {
        Ok(resp) => println!("[livekit_real_demo] list_participants -> \"Ok(participants={})\"", resp.participants.len()),
        Err(e) => println!("[livekit_real_demo] list_participants -> \"{e}\""),
    }
    let rp_req = match RemoveParticipantRequest::new("demo-room", "user-1") {
        Ok(r) => r,
        Err(e) => {
            println!("[livekit_real_demo] remove_participant -> \"K-1 强校验失败: {e}\"");
            return;
        }
    };
    match real.remove_participant(rp_req).await {
        Ok(_) => println!("[livekit_real_demo] remove_participant -> \"Ok\""),
        Err(e) => println!("[livekit_real_demo] remove_participant -> \"{e}\""),
    }

    // 演示 8: event.push + event.drain (in-memory 模拟, 跟诚实标缺 #2 1:1)
    println!("\n[demo 8/8] event.push + event.drain (in-memory 模拟, per 诚实标缺 #2 webhook server-side 模拟)");
    let room = Room {
        sid: "RM_demo".to_string(),
        name: "demo-room".to_string(),
        created_at: 1234567890,
        num_participants: 0,
        max_participants: 100,
        metadata: String::new(),
    };
    let evt = WebhookEvent::room_started(room);
    match real.push_event(evt.clone()).await {
        Ok(_) => println!("[livekit_real_demo] push_event -> \"Ok(event_id={})\"", evt.event_id),
        Err(e) => println!("[livekit_real_demo] push_event -> \"{e}\""),
    }
    match real.drain_events().await {
        Ok(events) => println!("[livekit_real_demo] drain_events -> \"Ok(drained={})\"", events.len()),
        Err(e) => println!("[livekit_real_demo] drain_events -> \"{e}\""),
    }

    // 演示 K-1 强校验 fail 演示
    println!("\n[demo bonus] K-1 强校验 fail 演示");
    let bad_req = CreateRoomRequest {
        name: "with space".to_string(),
        empty_timeout: 0,
        max_participants: 100,
        metadata: String::new(),
    };
    match real.create_room(bad_req).await {
        Ok(_) => println!("[livekit_real_demo] create_room (bad name) -> \"不应该到这\""),
        Err(e) => println!("[livekit_real_demo] create_room (bad name) -> \"{e}\""),
    }

    println!("\n[livekit_real_demo] 演示完成 (R20 阶段 6 flesh out 真接实现已 ready, Mavis 整合 #3 拍板后切 STUB_MODE=false)");
}
