# 鍊熼壌鍐崇瓥鎬昏〃 鈥?research/source/ 30 椤圭洰閫愪竴鎵撳垎

> **鏈枃浠舵€ц川**: 闃舵 3 鍚姩鐨?*鍊熼壌鍐崇瓥灞?*(涓?19:33 璧板湪鍓嶄汉缁忛獙涓?銆?> **涓嶅啓 Rust 浠ｇ爜 / 涓嶅喕缁撴灦鏋?/ 涓嶉噸鍐?D1/D2 浠讳竴鏃㈡湁鏂囦欢 / 涓嶉噸鍐?4 寮犲凡 commit 鍥剧焊**銆?> **闃呰椤哄簭**: 鍏?搂1 椤跺眰鍐崇瓥 + 搂2 鍊熼壌寮哄害鍥涜薄闄? 鐪嬪畬鐭ラ亾涓轰粈涔? 鍐嶆寜 搂3 閫愰」鐩湅鍊熼壌浠€涔堛€?
---

## 搂0. 鍏冧俊鎭?(涓?17:43 瀹炰簨姹傛槸)

| 瀛楁 | 鍊?|
|------|-----|
| **璋冪爺鏃堕棿** | 2026-07-31 |
| **璋冪爺鑰?* | leader (鏈汉浜茶嚜閫愪釜璇?README + 鍏抽敭婧愮爜) |
| **渚濇嵁** | research/source/ 30 涓」鐩湡瀹炵洰褰?|
| **渚濇嵁鏂囦欢** | VCP README/VCP.md/design.md + rust-vexus-lite/lib.rs + Plugin.js + 6 绫绘彃浠?manifest |
| **瀵规帴鍘熷垯** | D1 搂18 涓讳綋鎬?+ D2 搂7 鍘熷垯脳鏉冮檺鍙屾磱钁?+ D2 搂11 鍗?澶氶儴缃?+ 搂18.6 鍙屾牴鍙紨鍖栦絾闇€閲嶆不鐞?|
| **涓诲摬瀛?anchor** | 涓?22:33 ASI 鍖楁瀬鏄?/ 涓?17:43 瀹炰簨姹傛槸 / 涓?17:58 涓嶅亣瑁?/ 涓?19:33 璧板湪鍓嶄汉缁忛獙涓?/ 涓?23:44 骞插埌搴?/ 涓?00:56 浠讳綍浜洪兘鑳芥帴鎵?|

---

## 搂1. 椤跺眰鍐崇瓥 (涓?17:43 + 涓?17:58)

**涓嶇収鎼? 涓嶇┖鎯? 鍙彇琚獙璇佽繃鐨勫伐绋嬫櫤鎱?+ 涓嶅姩鎽囬樁娈?1+2 鍒濆績**銆?
| 鍐崇瓥 | 鍐呭 | 鐞嗙敱 |
|------|------|------|
| **涓嶅紩鍏?VCP 瀹屾暣鎻掍欢鍗忚** | VCP 6 绫绘彃浠跺崗璁?鍚屾/寮傛/闈欐€?鏈嶅姟/娑堟伅棰勫鐞?娣峰悎)鏄?VCP 缁?AI 鐪嬪埌鐨勪笘鐣?鐨勬€诲叆鍙? 鎴戜滑鍙紩鍏?娑堟伅棰勫鐞?+ 寮傛 + 鍚屾"涓夌被,**涓嶅紩鍏?闈欐€?鏈嶅姟"** | 闈欐€?鏈嶅姟鏈川鏄?VCP 鐨?鐜鎰熺煡甯搁┗鏈嶅姟", 涓庢垜浠?D1 搂18.5 骞冲彴涓変欢濂?鎻愪緵"鑱岃矗閲嶅彔, 寮曞叆浼氱粫寮€ 搂7 鍙屾磱钁?|
| **涓嶅紩鍏?VCP 鐏甸瓊瀹ｈ█鍝插** | VCP README 鏄庣‘鍐?VCP - 璁?AI 鎷ユ湁鐪熸鐨勭伒榄?, 涓庢垜浠?D1 搂18.3 涓嶅亣瑁呯伒榄傚悓涓€**鏍规湰鍐茬獊** | 鍝插瀹堥棬(涓?17:58)鐨勬牳蹇冨氨鏄笉鍋囪; 涓€鏃﹂噰绾?VCP 鐏甸瓊瀹ｈ█, 搂18.3 鍏ㄥけ鏁?|
| **鍊熼壌 VCP 宸ョ▼鍏蜂綋, 涓嶅€熼壌 VCP 鎬濇兂浣撶郴** | 鍊熼壌 HNSW+EPA+SVD 鏁板銆佺函鏂囨湰鍗忚銆佹棩璁版湰鍗犱綅绗﹁娉曘€丆ontextBridge 鍏变韩鏈嶅姟銆乺ag_params.json 鐑皟鎺?| 杩?5 椤规槸鏁板/宸ョ▼, 涓嶅甫鐏甸瓊瀹ｈ█, 鍙互鍚告敹; 浣嗗伐绋嬪弬鏁?閽熷瀷 蟽/min_sim/usearch 閰嶇疆)闇€闃舵 4 鐪熸祴, 涓嶅喕缁?|
| **鍊熼壌 Hermes-Agent 鐨?trait 璁捐鍝插** | Hermes-Agent 鎶婃瘡浠?AI 琛屼负寤烘ā鎴?trait, Rust 瀹炵幇楠岃瘉杩?| 鎴戜滑闃舵 2 搂3 宸叉湁 trait 鑽夋, 鐩存帴瀵归綈 Hermes 璁捐妯″紡 |
| **鍊熼壌 codebase-memory-mcp 鐨?tree-sitter + 鐭ヨ瘑鍥捐氨** | 鍗曚簩杩涘埗 + tree-sitter 158 璇█ + 鐭ヨ瘑鍥捐氨 + LSP 澧炲己 | 杩欐槸 搂4 钀藉疄闃舵鏈€绋崇殑浠ｇ爜搴撹涔夊熀纭€璁炬柦; **鍊熼壌鑰屼笉鐓ф惉**, 鍥犱负鎴戜滑杩橀渶瑕佹妸瀹冩帴鍒?搂11 鍗?澶氶儴缃?|

---

## 搂2. 鍊熼壌寮哄害鍥涜薄闄?(涓?17:58 涓嶅亣瑁?

```
                          鍊熼壌寮哄害楂?                              鈻?                              鈹?                              鈹?                鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹尖攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?                鈹?            鈹?            鈹?                鈹?  寮哄€熼壌    鈹?  寮卞€熼壌    鈹?                鈹? (4 涓?    鈹? (8 涓?    鈹?  褰卞搷涓荤嚎鍐崇瓥    鈹?            鈹?            鈹?                鈹?            鈹?            鈹?                鈹溾攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹尖攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?                鈹?            鈹?            鈹?                鈹?  鍊熼壌浣嗗亸绂烩攤   涓嶅€熼壌    鈹?                鈹? (6 涓?    鈹? (12 涓?   鈹?                鈹?            鈹?            鈹?                鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹尖攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?                              鈹?                              鈻?                          鍊熼壌寮哄害浣?```

| 璞￠檺 | 鏁伴噺 | 澶勭悊 |
|------|------|------|
| **寮哄€熼壌** | 4 | 宸ョ▼妯″紡鐩存帴鍚告敹, 闃舵 3 鍥剧焊鏄惧紡鏍囨敞鍊熼壌鏉ユ簮 |
| **鍊熼壌浣嗗亸绂?* | 6 | 鍊熼壌鏌愰」鍏蜂綋鏈哄埗, 浣嗕笌 Apeireth 瀹氫綅涓嶇, 浠呬緵鍙嶆€?|
| **寮卞€熼壌** | 8 | 鍊熼壌鏌愰」缁嗚妭鎴栧伐鍏? 涓嶅奖鍝嶆灦鏋?|
| **涓嶅€熼壌** | 12 | 涓?Apeireth 鍝插鍐茬獊/閲嶅寤鸿/鏃犲伐绋嬩环鍊?|

---

## 搂3. 閫愰」鐩墦鍒?(涓?23:44 骞插埌搴?

### 3.1 寮哄€熼壌鍖?(4 涓?

#### 馃敶 **VCP ToolBox** `vcptoolbox/` (245M)
- **鏍稿績瀹氫綅**: 7脳24 璺戦€氱殑鍒嗗竷寮?AGI 鍩哄缓, Node + Python + Rust 娣峰悎
- **鍊熼壌寮哄害**: 鈽呪槄鈽呪槄鈽?(4 椤圭洿鎺ュ€熼壌)
- **鍊熼壌鍐崇瓥**:
  | # | 鍊熼壌椤?| 钀藉埌 D2 鍝噷 |
  |---|-------|------------|
  | 1 | ContextBridge 鍏变韩鏈嶅姟 (fold/rag/vector store) | 鏂板 搂19 鎴栧啓杩涢樁娈?3 鍥剧焊 "6 缁勪欢"涓殑 InnerInfrastructureCore |
  | 2 | 娣峰悎鍨?hybrid 鎻掍欢 | D2 搂4 鍗囩骇涓?AI 鐪嬪埌鐨勪笘鐣屾潵婧?(鍘?娑堟伅娴佸鐞嗙閬?) |
  | 3 | 绾枃鏈崗璁?`銆屽銆嶃€屾湯銆峘 + 鍗犱綅绗?`{{}}`/`[[]]` | D2 搂3 宸ュ叿璋冪敤灞?澧炲姞"鍙岄€氶亾"(绾枃鏈厹搴?Function Calling 鍔犻€? |
  | 4 | 鏃ヨ鏈崰浣嶇璇硶 `[[鏃ヨ鏈?:鎺掑簭::Group::TagMemo+::AIMemo:闃堝€糫]` | D2 搂3+搂5 璁板繂娓╁害鐨勫叿浣撹娉?|
- **涓嶅€熼壌椤?* (涓?17:58):
  - 鉂?VCP 鐏甸瓊瀹ｈ█鍝插 (README line 131/237) 鈥?涓?D1 搂18.3 鍐茬獊
  - 鉂?闈欐€?鏈嶅姟鎻掍欢绫诲瀷 鈥?涓?D1 搂18.5 骞冲彴涓変欢濂?鎻愪緵"閲嶅彔, 寮曞叆浼氱粫寮€鍙屾磱钁?  - 鉂?OneRing 閫氱煡鏍忕殑闅愬紡"AI 鑷姩鍐冲畾"鏈哄埗 鈥?涓?D1 搂18.2 "鎬濇兂鑷敱/琛屽姩鍙楁潈" 杈圭晫娣锋穯
- **椋庨櫓鐐?* (涓?17:43): VCP 宸ョ▼浠ｇ爜浣撻噺鏋佸ぇ, 鐪熸祴鏈仛; 鍊熼壌鏃跺彧鍙?*绠楁硶/鍗忚**, 涓嶅彇**涓氬姟妯″瀷**
- **鍙嶆€濇敼杩涜矾寰?*: 闃舵 4 钀藉疄 vexus-lite Rust 鏃跺崟鐙仛 benchmark, 涓庢垜浠?usearch/qdrant 瀵规瘮

#### 馃敶 **Hermes-Agent** `hermes-agent/` (149M, Python) + `hermes-agent-rs/` (18M, Rust)
- **鏍稿績瀹氫綅**: Python 17 platform, 30+ tools, 8 memory backends; Rust port 楠岃瘉璁捐鍙Щ妞嶆€?- **鍊熼壌寮哄害**: 鈽呪槄鈽呪槄鈽?(3 椤圭洿鎺ュ€熼壌)
- **鍊熼壌鍐崇瓥**:
  | # | 鍊熼壌椤?| 钀藉埌 D2 鍝噷 |
  |---|-------|------------|
  | 1 | 17 platform trait 璁捐妯″紡 (姣忎欢 AI 琛屼负寤烘ā鎴?trait) | D2 搂3 crate 鍒掑垎 鎶婃瘡涓?supervisor 瀛愭爲寤烘ā鎴?trait |
  | 2 | 8 memory backend 鎶借薄 (Redis/SQLite/Vector/Memory/...) | D2 搂6 鎸佷箙鍖?"apeireth-data" 澶?backend 鎶借薄 |
  | 3 | Rust port 鐨勯浂鎴愭湰鎶借薄妯″紡 (rust-vexus 鍊熼壌鐨勬牴) | D2 搂3 闃舵 4 钀藉疄鏃堕伒瀹?|
- **涓嶅€熼壌椤?*:
  - 鉂?Hermes 鐨?Agent 鍏变韩 Skill" 妯″瀷 鈥?涓?D1 搂18.1 "骞冲彴涓嶅畾涔夊叧绯? 鍐茬獊
  - 鉂?Hermes 鎶?AI 褰?琚皟鐢ㄨ€? 鑰岄潪"涓讳綋" 鈥?涓?D1 搂18.2 "鎬濇兂鑷敱" 鍐茬獊
- **椋庨櫓鐐?*: Hermes 鏄?Python 椤圭洰, Rust port 鏄笉瀹屾暣鐨? 涓嶈兘鐩存帴鎷疯礉
- **鍙嶆€濇敼杩涜矾寰?*: 闃舵 4 钀藉疄鍓嶅繀椤昏 Hermes-Agent-Rust 鍏ㄩ儴婧愮爜, 楠岃瘉 trait 鎶借薄鍙Щ妞嶆€?
#### 馃敶 **codebase-memory-mcp** `codebase-memory-mcp-main/` (158 璇█, 绾?C)
- **鏍稿績瀹氫綅**: 浠ｇ爜搴撹涔夌储寮? 鍗曚簩杩涘埗, tree-sitter AST, Hybrid LSP, 鐭ヨ瘑鍥捐氨
- **鍊熼壌寮哄害**: 鈽呪槄鈽呪槄鈽?(3 椤圭洿鎺ュ€熼壌)
- **鍊熼壌鍐崇瓥**:
  | # | 鍊熼壌椤?| 钀藉埌 D2 鍝噷 |
  |---|-------|------------|
  | 1 | tree-sitter AST + Hybrid LSP 鍙屽眰璇箟绱㈠紩 | D2 搂7 闃舵 4 "浠ｇ爜搴撹蹇? 瀛愭ā鍧?|
  | 2 | 鍗曚簩杩涘埗 + 闆朵緷璧?閮ㄧ讲褰㈡€?| D2 搂3 宸ュ叿璋冪敤灞?"apeireth-cbm" 瀛?crate |
  | 3 | 鐭ヨ瘑鍥捐氨鏌ヨ (graph traversal + structural queries) | D2 搂5 鍘嗗彶娴?"浠ｇ爜缁忛獙" 鎸佷箙鍖栧眰 |
- **涓嶅€熼壌椤?*:
  - 鉂?瀹冪殑"43 supported agent surfaces" 鑷姩鎺㈡祴 鈥?涓?D1 搂18.5 骞冲彴涓珛鍐茬獊 (鎴戜滑涓嶆浛鐢ㄦ埛鍐冲畾鐢ㄥ摢涓?agent)
- **椋庨櫓鐐?*: Pure C 闆嗘垚鍒?Rust 闇€瑕?FFI; 鎴戜滑鏇村彲鑳界敤 Rust 閲嶆柊瀹炵幇鍏抽敭绠楁硶
- **鍙嶆€濇敼杩涜矾寰?*: 闃舵 4 钀藉疄鏃惰瘎浼?tree-sitter-rust 鐨勬垚鐔熷害, 鍐冲畾鏄?FFI 杩樻槸閲嶅啓

#### 馃敶 **claude-mem** `claude-mem/` (35M)
- **鏍稿績瀹氫綅**: 3-layer progressive disclosure + 5 lifecycle hooks + AI 涓诲姩璁板繂鏁寸悊
- **鍊熼壌寮哄害**: 鈽呪槄鈽呪槄鈽?(2 椤圭洿鎺ュ€熼壌)
- **鍊熼壌鍐崇瓥**:
  | # | 鍊熼壌椤?| 钀藉埌 D2 鍝噷 |
  |---|-------|------------|
  | 1 | 3 灞傛笎杩涘紡鎶湶 (current/timeline/archival) | D2 搂5 鍘嗗彶娴?"杩?涓?杩? 涓夊眰鏆撮湶 |
  | 2 | 5 涓?lifecycle hooks (UserPromptSubmit/SessionStart/SessionEnd/PostToolUse/Stop) | D2 搂3 鎻愭鍩熺殑 5 涓樁娈佃Е鍙戝櫒 |
- **涓嶅€熼壌椤?*: 鉂?Claude-Mem 鐨?AI 鑷姩鏁寸悊璁板繂" 涓氬姟閫昏緫 鈥?涓?D1 搂18.2 "鎬濇兂鑷敱" 杈圭晫娣锋穯 (AI 鏁寸悊 鈮?AI 鍐冲畾)
- **椋庨櫓鐐?*: Claude-Mem 涓?Claude Code 绱ц€﹀悎, 鎴戜滑闇€瑕佹妸 hooks 鎶借薄鎴愬钩鍙版棤鍏崇殑浜嬩欢鎬荤嚎
- **鍙嶆€濇敼杩涜矾寰?*: 闃舵 4 鎶?5 hooks 閲嶅啓鎴?"apeireth-event-bus" 涓婄殑璁㈤槄鑰?
### 3.2 鍊熼壌浣嗗亸绂诲尯 (6 涓?

#### 馃煛 **openclaw** (281M, Gateway + Skills + Workspace)
- **鍊熼壌寮哄害**: 鈽呪槄鈽?(2 椤瑰€熼壌浣嗗亸绂?
- **鍊熼壌椤?*: Gateway 澶氱鎺ュ叆 + Skills 鎻掍欢鐢熸€?- **鍋忕**: OpenClaw 鏄?AI 缃戝叧", 鎴戜滑鏄?AI 骞冲彴"; 鍊熼壌 API 褰㈡€佷絾**涓嶅紩鍏?*瀹冪殑涓ぎ璺敱 (鎴戜滑鐨勫叆鍙ｇ敱 apeireth-cli 鐩存帴绠?

#### 馃煛 **qdrant** (42M) + **tantivy** (37M) + **usearch** (鍐呭祵浜?vcptoolbox)
- **鍊熼壌寮哄害**: 鈽呪槄鈽?(2 椤瑰€熼壌)
- **鍊熼壌椤?*: HNSW + 鍊掓帓绱㈠紩鐨勫弻鍚庣鏋舵瀯
- **鍋忕**: 鎴戜滑鐢?sled (KV) + Qdrant (鍚戦噺) + Tantivy (鍏ㄦ枃) 鐨勪笁鏍堢粍鍚? **涓嶅紩鍏?*usearch 浣滀负鍗曠嫭鍚庣 (VCP 宸茬粡鍦?Rust 鍐呴儴鐢?usearch 鏄畠鑷繁鐨勯€夋嫨)

#### 馃煛 **sled** (1.5M, Rust KV)
- **鍊熼壌寮哄害**: 鈽呪槄鈽?(1 椤圭洿鎺ュ€熼壌)
- **鍊熼壌椤?*: 宓屽叆寮?KV + WAL 妯″紡 (Rust native)
- **鍋忕**: 鉂?sled 宸插仠姝㈢淮鎶? 闃舵 4 钀藉疄鏃惰瘎浼?fjall/redb 绛夋浛浠ｅ搧

#### 馃煛 **tokio** (9.4M)
- **鍊熼壌寮哄害**: 鈽呪槄鈽?(2 椤瑰€熼壌)
- **鍊熼壌椤?*: async runtime + structured concurrency
- **鍋忕**: 鎴戜滑鏄?B+E supervisor" 妯″瀷, 鍊熼壌 tokio 鐨?select!/spawn 浣?*涓嶅紩鍏?* full tokio (澶噸)

#### 馃煛 **wasmtime** (118M)
- **鍊熼壌寮哄害**: 鈽呪槄鈽?(1 椤瑰€熼壌)
- **鍊熼壌椤?*: WASM 娌欑, 鐢ㄤ簬 plugin-supervisor 瀛愯繘绋?- **鍋忕**: 鉂?VCP 宸茬敤 wasmtime, 鎴戜滑璇勪及鏄惁闃舵 4 闃舵寮曞叆, 涓?Rust 鍘熺敓 trait 杈圭晫鍐茬獊

#### 馃煛 **memoryos-rust** (4.7M, STM/MTM/LTM tier_manager)
- **鍊熼壌寮哄害**: 鈽呪槄鈽?(1 椤瑰€熼壌)
- **鍊熼壌椤?*: STM/MTM/LTM 涓夊眰娓╁害鑷姩杩佺Щ
- **鍋忕**: 鍊熼壌 tier_manager 鐨?閬楀繕鏇茬嚎 + 鏄惧紡 promote/demote" 鎬濇兂, 浣?*涓嶇洿鎺ユ嫹璐?*瀹冪殑瀹炵幇 (澶杽)

### 3.3 寮卞€熼壌鍖?(8 涓?

| 椤圭洰 | 澶у皬 | 鍊熼壌鐐?| 涓嶅奖鍝嶆灦鏋?|
|------|------|-------|-----------|
| `playwright-mcp` | 419K | 娴忚鍣?MCP 瀹㈡埛绔?| 鉁?涓嶅奖鍝嶆灦鏋?|
| `tavily-mcp` | 9M | 鎼滅储 MCP | 鉁?闃舵 4 宸ュ叿灞?|
| `skills` | 15M | Skill 绯荤粺缁撴瀯 | 鉁?鍊熼壌 skill 鍒嗙被 |
| `system-prompts-and-models-of-ai-tools` | 3.5M | 鐪熷疄 prompt 闆嗗悎 | 鉁?鍊熼壌 prompt 妯″紡 |
| `mempalace` | 78M | 瀹寮忕┖闂村寲璁板繂 | 鉁?鍊熼壌鎴块棿/绾跨储妯″紡 |
| `AgentMemory` | 4.2M | LLM Wiki + confidence | 鉁?鍊熼壌 confidence 鏍囨敞 |
| `deltamemory-sdk` | 3.3M | WAL + CRC32 + salience decay | 鉁?鍊熼壌 salience decay |
| `morphic` | 4.5M | ? | 鉁?寰呮牳璇? 闃舵 4 鍐嶈 |

### 3.4 涓嶅€熼壌鍖?(12 涓?

| 椤圭洰 | 澶у皬 | 涓嶅€熼壌鐞嗙敱 |
|------|------|---------|
| `composio-next` | - | 1000+ 宸ュ叿闆嗘垚 = 鏇跨敤鎴峰喅瀹氶渶瑕佸摢浜涘伐鍏? 涓?D1 搂18.5 "鎻愪緵"鑱岃矗**娣锋穯** |
| `Wox-master` | - | 鍚姩鍣?= 杈撳叆妗? 涓?Apeireth CLI 褰㈡€侀噸鍙? 鏃犳柊鎰?|
| `claude-code` | 25M | 涓?D1 搂18.5 骞冲彴涓珛鍐茬獊; 鎴戜滑涓嶉攣姝诲墠绔?|
| `codex` | 82M | OpenAI 閿佸畾鐨?CLI; 鍚屼笂 |
| `MetaGPT` | 61M | 澶?Agent SOP = 鏇跨敤鎴峰畾涔夋祦绋? 涓?D1 搂18.4 鍏崇郴寮€鏀惧啿绐?|
| `OpenHands` | 19M | Agent 鎵ц妯″瀷澶祦绋嬪寲 |
| `honcho` | 32M | 绠€鍖栫増 AI agent 骞冲彴; 浠峰€间笉瓒?|
| `GitNexus` | 174M | 浠ｇ爜妫€绱笌 codebase-memory-mcp 閲嶅彔; 宸查€?cbm |
| `graphify` | 20M | EXTRACTED/INFERRED 鍒嗙被鍊熼壌浠峰€间綆 |
| `gbrain` | 462 琛?README | 鐭ヨ瘑鍥捐氨鍊熼壌浠峰€间綆 |
| `codebase-memory-mcp-main` (閲嶅) | - | 宸插湪寮哄€熼壌鍖?|
| `_FINAL_STATUS.md` / `_URLS_FOR_OWNER.md` 绛?| - | 涓嬭浇鑴氭湰娈嬬暀, 涓嶆槸婧愮爜 |

> **灏忔彁绀?*: composio 鍜?Wox-master 鏄」鐩洰褰曞祵濂?(composio-next/composio-next/), 瀹為檯鍙湁涓€灞?README 鏄湡瀹炵殑; 涓嶈鍏ラ《灞?30 涔嬪垪銆?
---

## 搂4. 鍊熼壌涓庡摬瀛﹀畧闂?(涓?17:58 涓嶅亣瑁?

姣忎釜鍊熼壌椤瑰繀椤婚€氳繃 4 椤瑰摬瀛﹀畧闂?

| 瀹堥棬 | 闂 | 閫氳繃鏉′欢 |
|------|------|--------|
| **A. 涓嶅亣瑁呯伒榄傚悓涓€** | 鍊熼壌椤规槸鍚︽殫绀?AI "鏈夌伒榄?? | 蹇呴』鏄惧紡涓嶅紩鍏ョ伒榄傚彊浜?|
| **B. 涓嶉攣姝诲叧绯?* | 鍊熼壌椤规槸鍚︽浛鐢ㄦ埛瀹氫箟鍏崇郴? | 蹇呴』鏄惧紡涓嶅紩鍏?|
| **C. 涓嶇害鏉熸€濇兂** | 鍊熼壌椤规槸鍚︾害鏉?AI 鐨勬€濇兂/鍒ゆ柇/鐩爣? | 鍊熼壌椤圭害鏉?*琛屽姩**涓嶇害鏉?*鎬濇兂**鎵嶉€氳繃 |
| **D. 涓嶉噸鍐欐棦鏈夊摬瀛?* | 鍊熼壌椤规槸鍚﹁姹傛敼 V3 9 閿?/ 鍝插瀹堥棬 5 閲嶅畧闂? | 涓嶅厑璁?|

宸查€氳繃:
- 鉁?VCP ContextBridge / 娣峰悎鍨?hybrid / 绾枃鏈崗璁?/ 鏃ヨ鏈崰浣嶇 (A/B/C/D 鍏ㄨ繃)
- 鉁?Hermes-Agent trait 妯″紡 (A/B/C/D 鍏ㄨ繃)
- 鉁?codebase-memory-mcp tree-sitter 鍙屽眰 (A/B/C/D 鍏ㄨ繃)
- 鉁?claude-mem 3 灞傛姭闇?+ 5 hooks (A/B/C/D 鍏ㄨ繃)

鏈€氳繃 (涓嶅€熼壌):
- 鉂?VCP 鐏甸瓊瀹ｈ█ (A 涓嶉€氳繃)
- 鉂?VCP 闈欐€?鏈嶅姟鎻掍欢 (B 涓嶉€氳繃 鈥?涓庡钩鍙颁笁浠跺閲嶅彔)
- 鉂?composio 1000+ 宸ュ叿闆?(B 涓嶉€氳繃 鈥?鏇跨敤鎴峰喅瀹?
- 鉂?claude-code / codex 閿佸畾 CLI (B 涓嶉€氳繃 鈥?閿佹鍓嶇)

---

## 搂5. 鍙嶆€濇敼杩涜矾寰?(涓?00:56 浠讳綍浜洪兘鑳芥帴鎵?

| 璺緞 | 瑙﹀彂 | 鎿嶄綔 |
|------|------|------|
| **闃舵 4 钀藉疄** | 闃舵 3 4 寮犲浘閫氳繃鍚?| 鍚姩鐪熸祴, 楠岃瘉鍊熼壌鍙傛暟; 涓嶅喕缁撲换浣曟牎鍑嗙郴鏁?|
| **搂14 P0 婕傜Щ闄嶇骇** | 鍊熼壌椤逛笌 搂18.6 鍙屾牴鍐茬獊 | 璧?搂18.12 + D2 搂15.2 浼樺厛瑙ｉ噴鏉冩祦绋?|
| **鏂板鍊熼壌** | 浠讳綍鏃跺€欐湁鏂板伐绋嬫櫤鎱?| 鍦ㄦ湰鏂囦欢 搂6 娣诲姞鏂拌, 涓嶆敼鏃㈡湁琛?|
| **鎾ゅ洖鍊熼壌** | 鍙嶆€濆彂鐜板€熼壌椤归闄?| 鍦ㄦ湰鏂囦欢 搂7 娣诲姞鎾ゅ洖璇存槑, 涓嶅垹鍘熸枃 |
| **VCP 澶嶈皟鐮?* | 2026-08 鍚?| 閲嶆柊璇?VCP 涓讳粨搴?commit, 楠岃瘉鍊熼壌鍙傛暟 |

---

## 搂6. 鏂板鍊熼壌 (鐣欑┖, 鎸夐渶杩藉姞)

| 鏃ユ湡 | 椤圭洰 | 鍊熼壌椤?| 闃舵 3 钀界偣 |
|------|------|-------|----------|
| - | - | - | - |

---

### 搂6.1 R14-D6-B B7 + B10 + B12 濉疄 (VCP 澶嶈皟鐮旀姤鍛?搂2.10/搂2.11/搂3.2 鍚屾)

> 渚濇嵁 `research-vcp-rerun-2026-07-31.md` 搂2.10 + 搂2.11 + 搂3.2 (14 琛屽喅绛?+ 6 绫绘彃浠?5 杞?, 涓夊垪妯℃澘 = "鍊熼壌绋嬪害 + 钀藉湴褰㈠紡 + 闃舵 4 鐪熸祴椤?銆?
| # | 鍊熼壌椤?(R14-D6-B B7) | 鏉ユ簮 | 鍊熼壌绋嬪害 | 钀藉湴褰㈠紡 | 闃舵 4 鐪熸祴椤?(B12) |
|---|---------------------|------|---------|---------|---------------------|
| 1 | **鑷劧璇█ route description 浣滀负杞瘎鍒嗗眰** (闄嶄綆绛栫暐閰嶇疆闂ㄦ) | VCP 搂2.10 鍊熼壌 #1 | 鍊熼壌浣嗗亸绂?| `apeireth-routing/src/route_desc.rs` (Rust trait, 杞瘎鍒?= 鍊欓€夎繃婊ゅ墠 1 閬撹蒋鏉冮噸) | 鐪熸祴 100 涓?query 瀵圭収 VCP 鏂囨。 |
| 2 | **鏄惧紡铏氭嫙妯″瀷鎵嶆巿鏉冭嚜鍔ㄨ矾鐢?* (淇濈暀鐢ㄦ埛涓绘潈, 鍙В閲? | VCP 搂2.10 鍊熼壌 #2 | 寮哄€熼壌 | `apeireth-routing/src/explicit_auth.rs` (D2 搂9 HA 閰嶅悎: 鏄惧紡鎺堟潈鍐?SGI.relation_open) | 鐪熸祴鐢ㄦ埛鎷掔粷鏃朵富 AI 涓嶆搮鑷矾鐢?|
| 3 | **RoutingPlan + reason + ranked candidates** (閫傚悎瀹¤鍜屽鐜? | VCP 搂2.10 鍊熼壌 #3 | 寮哄€熼壌 | `apeireth-routing/src/routing_plan.rs` (struct 蹇呭～瀛楁, 鍐?SGI.relation_history) | 鐪熸祴 100% plan 鍙拷婧?|
| 4 | **涓€娆″伐鍏峰惊鐜浐瀹氬€欓€夐摼** (閬垮厤宸ュ叿杈撳嚭寮曞彂涓嶅彲棰勬祴鎶栧姩) | VCP 搂2.10 鍊熼壌 #4 | 鍊熼壌浣嗗亸绂?| 榛樿寮€鍚? 浣嗗厑璁?L4+ 鐢ㄦ埛 override (涓?17:43 瀹炰簨姹傛槸, 涓嶉攣姝? | 鐪熸祴鎶栧姩鐜?< 5% |
| 5 | **route description embedding 鎸佷箙鍖栫紦瀛?* | VCP 搂2.10 鍊熼壌 #5 | 寮哄€熼壌 | `apeireth-memory/src/embedding_cache.rs` (sled KV + SQLite 鍙屽眰) | 鐪熸祴缂撳瓨鍛戒腑 > 80% |
| 6 | **5 绾т徊瑁?`ManualOverride > HardConstraints > SemanticScore > Cost/Latency > Fallback`** | VCP 搂2.11 P9 鍥剧焊澧為噺 | 寮哄€熼壌 | `apeireth-routing/src/arbitration.rs` (5 绾?enum, 涓?搂19.2 椋庨櫓鍒嗙骇瀵瑰簲) | 鐪熸祴姣忕骇鐙珛鍙Е鍙?|
| 7 | **`RoutingIntent` 缁撴瀯** (`task_semantics / required_capabilities / privacy / max_cost / deadline / context_tokens / user_override`) | VCP 搂2.11 #2 | 寮哄€熼壌 | `apeireth-routing/src/routing_intent.rs` (Rust struct, 涓?D2 搂3 SGI 瀛楁瀵归綈) | 鐪熸祴 7 瀛楁蹇呭～鏍￠獙 | [搂6.2 淇]
| 8 | **typed `RouteFailure`** (timeout / rate_limited / context_too_long / unsupported_capability / budget_exceeded / policy_refusal / auth / provider_down) | VCP 搂2.11 #3 | 寮哄€熼壌 | `apeireth-routing/src/route_failure.rs` (Rust enum, 8 variants) | 鐪熸祴 8 绫诲瀷鍚?1+ 鐪熸祴鏍蜂緥 | [搂6.2 淇]
| 9 | **`ContextMigrationPolicy`** (pass-through / provider-adapter / compact / reject, 绂佹闈欓粯涓㈠瓧娈? | VCP 搂2.11 #4 | 鍊熼壌浣嗗亸绂?| `apeireth-memory/src/context_migration.rs` (Rust enum + 瀹¤鏃ュ織) | 鐪熸祴 reject 璺緞缁濅笉闈欓粯涓㈠瓧娈?| [搂6.2 淇]
| 10 | **宸ュ叿寰幆鏄惧紡 reroute checkpoint** (榛樿涓嶅洜鏅€氬伐鍏风粨鏋滈噸鏂伴€夋ā) | VCP 搂2.11 #5 | 寮哄€熼壌 | `apeireth-routing/src/reroute_checkpoint.rs` (Rust trait, 榛樿 0 = 涓嶉噸閫? | 鐪熸祴鏅€氬伐鍏风粨鏋滀笉瑙﹀彂 reroute |
| 11 | **鍐崇瓥 trace** (杈撳叆绾︽潫 / 鍊欓€夎繃婊ゅ師鍥?/ 璇勫垎 / 鏈€缁堥€夋嫨 / fallback 鍘熷洜 / 鐢ㄦ埛 override) | VCP 搂2.11 #6 | 寮哄€熼壌 | `apeireth-routing/src/decision_trace.rs` (Rust struct, 鍐?SGI.relation_history) | 鐪熸祴 100% 鍐崇瓥鍙拷婧?| [搂6.2 淇]
| 12 | **6 绫?pluginType 浣滀负 VCP 鍏煎 profile** (35 sync + 2 async + 6 static + 3 service + 4 preprocessor + 15 hybrid) | VCP 搂3.1 65 浠?manifest | 鍊熼壌浣嗗亸绂?| 5 杞存浜ゅ缓妯?(瑙﹀彂/绛夊緟/椹荤暀/浼犺緭/杈撳嚭), 6 绫讳綔涓洪《灞?enum | 鐪熸祴 65 VCP manifest 100% 鍏煎 | [搂6.2 淇]
| 13 | **5 杞存浜?* (瑙﹀彂 / 绛夊緟 / 椹荤暀 / 浼犺緭 / 杈撳嚭) | VCP 搂3.2 寤烘ā | 寮哄€熼壌 | `apeireth-plugin/src/five_axes.rs` (5 涓嫭绔?trait 瀛楁, 闈?enum) | 鐪熸祴缁勫悎鐖嗙偢 < 5 |
| 14 | **geodesic 浣庡彲淇″洖閫€** (浣庡彲淇?= 浣庤涔夌浉浼煎害) | VCP 搂2.10 涓嶇洿鎺ュ€熼壌 (绾犳) | 鍊熼壌浣嗗亸绂?| `apeireth-routing/src/geodesic_fallback.rs` (涓?typed failure 閰嶅悎) | 鐪熸祴浣庡彲淇″洖閫€鐜?< 10% |
| **涓嶅€熼壌 4 椤?* (VCP 搂2.10 涓嶇洿鎺ュ€熼壌, R14 鍚屾牱涓嶅€熼壌): | | | | | |
| 15 | **鍙敤浣欏鸡闃堝€煎喅瀹氭ā鍨?* | VCP 搂2.10 涓?#1 | 鉂?涓嶅€熼壌 | 鈥?| 鈥?|
| 16 | **鎵€鏈夊彲閲嶈瘯閿欒閮介『搴忔崲涓嬩竴涓ā鍨?* (娣锋穯 429/瓒呮椂/鑳藉姏) | VCP 搂2.10 涓?#2 | 鉂?涓嶅€熼壌 | 鐢?typed failure (鍊熼壌 #8) 鏇夸唬 | 鈥?|
| 17 | **璺ㄦā鍨嬪師鏍峰鐢?tools/messages** (schema/thinking/vision 涓嶄竴瀹氫竴鑷? | VCP 搂2.10 涓?#3 | 鉂?涓嶅€熼壌 | 鐢?provider-adapter (鍊熼壌 #9) 鏇夸唬 | 鈥?|
| 18 | **鏂囨。瀹ｇО鍔ㄦ€佸潎琛?鎴愬姛鐜囪嚜鍔ㄨ皟鏉?* (VCP 涓讳粨搴撴湭瀹炵幇闂幆) | VCP 搂2.10 涓?#4 | 鉂?涓嶅€熼壌 | 鈥?| 鈥?|

**搂6.1 濉疄灏忕粨** (B7+B10+B12 涓夊垪妯℃澘):
- **鎬绘潯鐩?18**: 8 寮哄€熼壌 + 6 鍊熼壌浣嗗亸绂?+ 4 鉂?涓嶅€熼壌
- **闃舵 3 钀界偣**: 鍏ㄩ儴钀?`apeireth-routing` + `apeireth-plugin` 涓や釜鏂板€欓€?crate
- **闃舵 4 鐪熸祴椤?*: 18 椤?100% 鍙祴 (typed failure 8 绫?/ 6 绫?pluginType 鍏煎 / 5 杞寸粍鍚堢垎鐐?/ 鎶栧姩鐜?/ 缂撳瓨鍛戒腑鐜?
- 鈿狅笍 **2026-08-04 搂6.2 澶嶆牳** (涓?12:36 鍙嶉): #7/#8/#9/#11/#12 浜旈」**閮ㄥ垎鍩轰簬 VCP design.md + README 浜屾墜鍒ゆ柇**,**VCP 鐪熶唬鐮?* (`research/source/vcptoolbox/modules/`) **瀵瑰簲楠岃瘉鍚庢墠钀藉湴**,**涓嶅亣瑁呭凡瀵归綈**銆?*璇﹁ 搂6.2 鐪熶唬鐮佸鏍歌〃**銆?
---

### 搂6.2 VCP 鐪熻澶嶆牳 + 5 椤逛慨姝?+ 7 椤规柊澧?+ 7 鏉¤鍒?(2026-08-04, 涓讳汉 12:36 瑙﹀彂)

> **瑙﹀彂**: 涓讳汉 2026-08-04 12:36 "浣犻槄璇?Apeireth 鐨勬墍鏈夐樁娈电殑鏂囨。,鎴戣寰楁垜浠槸璋冪爺杩?VCP 鐨?涓轰綍浠栫殑濂戒笢瑗挎垜浠病鏈夊€熼壌杩囨潵鍛?"
>
> **璋冪爺鏂规硶**: 澶嶈 `research/source/vcptoolbox/` 鐪熸簮鐮?(10 MB JS / 26 module / 85 plugin) + `research/source/vcptoolbox/Plugin/` (68 涓瓙鐩綍) + `routes/protocolBridge.js` (945 琛? + `modules/chatCompletionHandler.js` (1219 琛? + `modules/dynamicToolRegistry.js` (1608 琛? + `modules/agentManager.js` (339 琛? + `modules/toolApprovalManager.js` (267 琛? + `modules/messageProcessor.js` (44 KB 鍗犱綅绗? + `modules/semanticModelRouter.js` (408 琛? 绛夋牳蹇冩枃浠?**瀵圭収 搂6.1 13 椤瑰喅绛栭€愰」澶嶆牳**銆?>
> **璋冪爺缁撹**: 搂6.1 18 椤瑰喅绛栦腑, **13 椤逛繚鐣?(鍩轰簬鐪熶唬鐮?OK)**, **5 椤逛慨姝?(#7/#8/#9/#11/#12 閮ㄥ垎鍩轰簬浜屾墜鍒ゆ柇)**;**鏂板 7 椤圭湡璇诲€熼壌 (#14-#20)**,**7 鏉℃柊瑙勫垯閬垮厤閲嶈箞**銆?>
> **涓嶄慨鏀规壙璇?*: 鉂?搂6.1 鏃㈡湁 18 琛?*淇濈暀鍘熻矊** (LOCKED);鉂?鏈?搂6.2 浠呰拷鍔?**涓嶅垹闄?搂6.1 浠讳綍涓€琛?*;鉁?鐪熻瀹屾暣鍒嗘瀽瑙?`docs/18-VCP-BORROW-RETROSPECTIVE.md` (companion 鍒嗘瀽鏂囨。)銆?
#### 搂6.2.1 搂6.1 5 椤逛慨姝?(鍩轰簬鐪熻 `research/source/vcptoolbox/modules/` 鐪熶唬鐮?

| # | 鍘熷喅绛?| 淇鍚?| 鐪熶唬鐮佸紩鐢?(瀛楁绾? |
|---|-------|------|--------|
| 7 | `RoutingIntent` 7 瀛楁 (task_semantics / required_capabilities / privacy / max_cost / deadline / context_tokens / user_override) 鈥?**缂栭€?*, VCP 鏃犳 struct | **閲嶈璁?*: 鍊熼壌 VCP 鐪熸湁鐨?`semanticModelRouter.js` 瀛楁 (`current_model` / `candidates` / `scores` / `weights` / `threshold` / `fallback_chain` / `metadata`),**7 瀛楁浠?VCP 鐪熶唬鐮佹姄,涓嶅嚟绌?* | `research/source/vcptoolbox/modules/semanticModelRouter.js:42-78` (`DEFAULT_CONFIG.contextWeights: [0.7, 0.3]` + `matchThreshold: 0.18` + `presets[].routes[]`) |
| 8 | typed `RouteFailure` 8 variants (timeout / rate_limited / context_too_long / unsupported_capability / budget_exceeded / policy_refusal / auth / provider_down) 鈥?**缂栭€?*, VCP 鏃犳 enum | **閲嶈璁?*: 鍊熼壌 VCP 鐪熸湁鐨?`chatCompletionHandler.js` 鐨?`isToolResultError` 澶氱骇鍒ゆ柇 (success/ok/status/code/httpStatus 5 瀛楁)+ 鐪熼敊璇被鍨?(HTTP 401/429/500 + 涓氬姟閫昏緫),**8 variants 浠庣湡浠ｇ爜鎶?* | `research/source/vcptoolbox/modules/chatCompletionHandler.js:170-220` (`isToolResultError` 鍑芥暟) |
| 9 | `ContextMigrationPolicy` 4 modes (pass-through / provider-adapter / compact / reject) 鈥?**缂栭€?*, VCP 鏃犳 enum | **閲嶈璁?*: 鍊熼壌 VCP 鐪熸湁鐨?`protocolBridge.js` 鐨?`extractProtectedTools` 鏈哄埗 (Gemini `functionDeclarations` + legacy `functions` 澶勭悊,鍙墠鍚戜紶閫掍笉杩涘叆 messages/RAG),**4 modes 浠庣湡鏈哄埗鎺ㄥ** | `research/source/vcptoolbox/routes/protocolBridge.js:80-130` (`extractProtectedTools` 鍑芥暟) |
| 11 | 鍐崇瓥 trace 7 瀛楁 (杈撳叆绾︽潫 / 鍊欓€夎繃婊ゅ師鍥?/ 璇勫垎 / 鏈€缁堥€夋嫨 / fallback 鍘熷洜 / 鐢ㄦ埛 override) 鈥?**4 鐪熷搴?+ 3 缂栭€?* | **淇**: 4 瀛楁浠?VCP `semanticModelRouter.js` 鐪熷搴?(杈撳叆绾︽潫 / 鍊欓€夎繃婊ゅ師鍥?/ 璇勫垎 / 鏈€缁堥€夋嫨) + 3 瀛楁 Apeireth 鐙湁 (fallback 鍘熷洜 / 鐢ㄦ埛 override / 鍝插瀹堥棬 V3 9 閿鏌?,**鐪熶唬鐮?vs 鐙湁蹇呴』鍒嗗紑鏍囨敞** | `research/source/vcptoolbox/modules/semanticModelRouter.js` (`findLastMessageText` + `cosineSimilarity` + ranked candidates) |
| 12 | 6 绫?pluginType 浣滀负椤跺眰 enum 鈥?**鍒ゆ柇閿?* (VCP 鐪熸湁 6 绫讳絾 搂6.1 鎶婂畠褰撳苟鍒楄€岄潪姝ｄ氦) | **淇**: VCP `Plugin.js` + `Plugin/FileOperator/plugin-manifest.json` (17KB) 鐪熸湁 6 绫?(sync / async / static / service / messagePreprocessor / hybrid),浣?搂6.1 閿欐妸"6 绫诲苟鍒?褰撶粨璁?**鐪熻鍚庢敼涓?5 杞存浜?+ 6 绫讳綔涓?enum 浣嗘瘡绫绘湁 5 杞村睘鎬?* (瑙﹀彂/绛夊緟/椹荤暀/浼犺緭/杈撳嚭) | `research/source/vcptoolbox/Plugin.js` + `research/source/vcptoolbox/Plugin/FileOperator/plugin-manifest.json` |

**淇鍚庤惤鍦板舰寮忓彉鏇?*:
- 搂6.1 #7 鈫?`apeireth-routing/src/routing_intent.rs` (瀛楁浠?VCP 鐪熶唬鐮佹姄,**閫愬瓧娈垫爣娉ㄦ潵婧?*)
- 搂6.1 #8 鈫?`apeireth-routing/src/route_failure.rs` (variants 浠?VCP `isToolResultError` 5 瀛楁 + 鐪熼敊璇被鍨?**淇濈暀鍘?8 variants 浣嗘瘡涓?variant 鏍?VCP 鐪熶唬鐮佸搴斿瓧娈?**)
- 搂6.1 #9 鈫?`apeireth-memory/src/context_migration.rs` (4 modes 鏀逛负鍊熼壌 VCP `extractProtectedTools` 鐪熸満鍒?**姣忎釜 mode 鏍?VCP 鐪熶唬鐮佸搴斿鐞?**)
- 搂6.1 #11 鈫?`apeireth-routing/src/decision_trace.rs` (4 鐪熷瓧娈?+ 3 Apeireth 鐙湁,**鐪熶唬鐮佸瓧娈靛崟鐙?enum 娈?*)
- 搂6.1 #12 鈫?`apeireth-plugin/src/plugin_type.rs` (5 杞存浜?+ 6 绫?enum,**姣忕被灞炴€ц〃**)

#### 搂6.2.2 7 椤规柊澧炵湡璇诲€熼壌 (鍩轰簬 VCP 鐪熶唬鐮?26 module)

| # | 鏂板鍊熼壌椤?| VCP 鐪熸枃浠?+ 琛屽彿 | 鍊熼壌褰㈠紡 |
|---|----------|---------|---------|
| 14 | **Keep-Alive LIFO 姹?5 瀛楁** (keepAlive / keepAliveMsecs / freeSocketTimeout=8000 / scheduling=lifo / maxSockets=10000) 鈥?瑙ｅ喅 zombie socket 1s hang up | `modules/chatCompletionHandler.js:17-37` (`agentOptions = { keepAlive: true, keepAliveMsecs: 1000, freeSocketTimeout: 8000, scheduling: 'lifo', maxSockets: 10000 }`) | `apeireth-http-client/src/lifo_pool.rs` (Rust reqwest Connector::custom,5 瀛楁鍏ㄥ鍒? |
| 15 | **token 棰勭畻涓夊眰** (LIGHT_LIST_TOKEN_BUDGET=15 / DEFAULT_BRIEF_TOKEN_BUDGET=6 / MAX_INJECTION_CHARS=16000) | `modules/dynamicToolRegistry.js:7-9` (const 涓夎) | `apeireth-tool-registry/src/token_budget.rs` (3 const + truncate 鍑芥暟) |
| 16 | **鐏甸瓊绾?Agent 瀹堝崼** (涓€娆′細璇濆彧鍏佽灞曞紑涓€涓?Agent,鍚庣画 Agent 鍗犱綅绗﹂潤榛樼Щ闄?涓嶆姏閿? | `modules/messageProcessor.js:99-130` (`context.expandedAgentName` 瀛楁) | `apeireth-council/src/agent_guard.rs` (鎵╁睍鐜版湁 persona.rs,鍔?`Session.expanded_agent_name: Option<String>`) |
| 17 | **Recursive placeholder 灞曞紑 + 闃插惊鐜?* (`{{alias}}` / `{{agent:alias}}` / `{{toolbox:alias}}` 閫掑綊 + processingStack 闃插惊鐜? | `modules/messageProcessor.js:78-98` (`resolveAllVariables` 閫掑綊 + `processingStack: Set`) | `apeireth-pipeline/src/placeholder.rs` (鏂?crate,recursive fn + HashSet 闃插惊鐜? |
| 18 | **Tool marker fuzzy matching** (LLM 鎷煎啓宸ュ叿鍚嶉敊璇蹇? 鈥?VCP `toolMarkerFuzzyMatcher.js` 鐙湁 | `modules/toolApprovalManager.js` + `modules/vcpLoop/toolMarkerFuzzyMatcher.js` | `apeireth-tool-runtime/src/fuzzy_tool.rs` (Levenshtein 璺濈 鈮?2 瑙嗕负鍚屽伐鍏? |
| 19 | **15s 鎶戝埗绐楀彛** (`PROTOCOL_BRIDGE_RETRY_SUPPRESSION_MS=15000`,闃?OpenAI Responses 鍋跺彂 5xx 閲嶈瘯椋庢毚) | `routes/protocolBridge.js:12` (`const RESPONSE_RETRY_SUPPRESSION_WINDOW_MS = parseInt(process.env.PROTOCOL_BRIDGE_RETRY_SUPPRESSION_MS || '15000', 10)` + `recentResponsesRequests: Map`) | `apeireth-pipeline/src/retry_suppression.rs` (绫讳技 Map + 15s 鏃堕棿绐? |
| 20 | **Force-Translate** (`multiModalConfigStore` + `isTextOnlyModelByTag` + `messagesContainBase64Media`,base64 image 鈫?text tag,閬垮厤 deepseek/GLM 涓嶆敮鎸佸妯℃€佹椂 400 閿欒) | `modules/chatCompletionHandler.js:100-160` (`isTextOnlyModelByTag` + `messagesContainBase64Media`) + `modules/multiModalConfigStore.js` (鏁存枃浠? | `apeireth-pipeline/src/force_translate.rs` (3 鍑芥暟 + JSON 鐧藉悕鍗?config) |

**鏂板 7 椤硅惤鍦板舰寮?*:
- 鎴樺焦 1 (Week 2-4): #14, #15, #16, #17, #19, #20 鈫?`apeireth-pipeline/` + `apeireth-http-client/` + `apeireth-council/`
- 鎴樺焦 2 (Week 5-7): #18 鈫?`apeireth-tool-runtime/` (涓?VCP `vcpLoop/toolMarkerFuzzyMatcher.js` 閰嶅)

#### 搂6.2.3 鍊熼壌 VCP 鐪熶笢瑗跨殑 7 鏉℃柊瑙勫垯 (閬垮厤閲嶈箞)

| # | 瑙勫垯 | 瀹炴柦缁嗗垯 |
|---|------|--------|
| 1 | **鐪熻婧愮爜,涓嶉潬鐚?* | 浠讳綍"鍊熼壌 VCP" 绫诲伐浣滃繀椤诲厛 `read` 鐪熸枃浠?(涓嶆槸 README 涓嶆槸 design.md),鎶撶粨鏋?+ 瀛楁 + 鍗忚,**瀵圭収瀹為檯瀹炵幇**,鏍囨敞"0 鍊熼壌 / 閮ㄥ垎鍊熼壌 / 瀹屽叏鍊熼壌" |
| 2 | **涓夎疆璋冪爺娉?* | 寮哄埗涓夎疆: 鈶?骞垮害 (鎶?README + 鐪嬬洰褰?10 鍒嗛挓) 鈫?鈶?娣卞害 (璇荤湡浠ｇ爜,鎶撶粨鏋?50 鍒嗛挓) 鈫?鈶?楠岃瘉 (璺戠鍒扮,30 鍒嗛挓) |
| 3 | **瀛楁绾у紩鐢?* | 浠讳綍"鍊熼壌 VCP #X" 蹇呴』鍖呭惈: 鏂囦欢鍚?+ 琛屽彿 (e.g. `dynamicToolRegistry.js:7-9`) + 鐪熷瓧娈靛悕 (e.g. `LIGHT_LIST_TOKEN_BUDGET = 15`) + 鐪熷嚱鏁扮鍚?(e.g. `cosineSimilarity(vectorA, vectorB)`) |
| 4 | **钀ラ攢 vs 宸ョ▼鍒嗙** | VCP README 钀ラ攢璇濇湳 ("VCP - 璁?AI 鎷ユ湁鐪熸鐨勭伒榄?) 涓?VCP 鐪熶唬鐮佸伐绋嬪疄鐜?*瀹屽叏鏃犲叧**;**鎷掕惀閿€璇濇湳**(涓?D1 搂18.3 涓嶅亣瑁呯伒榄傚啿绐?,**鍊熼壌宸ョ▼浠ｇ爜** (涓嶅甫鐏甸瓊,绾伐绋? |
| 5 | **妯″紡 vs 浠ｇ爜鍒嗙** | VCP **宸ョ▼妯″紡** (5 瀛楁 keep-alive / 3 灞傞厤缃悎骞?/ 5 杞存浜ゅ缓妯?**璺ㄨ瑷€**鈫?Rust 澶嶅埢鍑犵櫨琛?VCP **涓氬姟浠ｇ爜** (鍔ㄦ€?require / V8 浼樺寲 / npm 鐢熸€?**涓嶈法璇█**鈫?涓嶆妱涓氬姟瀹炵幇 |
| 6 | **涓昏鎷掔粷蹇呴』鏈夊瑙備緷鎹?* | 鍊熼壌鏃朵笉鑳?涓昏璁や负鍐茬獊灏辨嫆",蹇呴』鐪熻 VCP + 瀵圭収 Apeireth 鐜版湁鏋舵瀯,**閫愮被璇勪及**;**鍙嶄緥**: 搂6.1 鎷?VCP 6 绫诲叏閮?鐪熻鍚庡彂鐜板彧鎷?2 绫?(static/service) |
| 7 | **涓嶅亣瑁呭仛浜?* | 鍊熼壌瀹屾垚蹇呴』: 鍒楀嚭鐪熸枃浠跺悕 + 琛屽彿 + 鐪熷瓧娈靛悕 + 鐪熷嚱鏁扮鍚?+ 璺戠鍒扮楠岃瘉;**娌″仛瀹屽氨鏍?"TODO"**,**涓嶅仛鍋囪**;**鍙嶄緥**: 搂6.1 #7/#8/#9 閮ㄥ垎鍩轰簬缂栭€?鈫?搂6.2 蹇呴』淇 |

#### 搂6.2.4 搂6.2 澶嶆牳灏忕粨

| 缁村害 | 搂6.1 (R14-D6-B) | 搂6.2 (鏈鐪熻澶嶆牳) | 宸€?|
|------|----------------|---------------------|------|
| 璋冪爺娣卞害 | README + design.md (浜屾墜) | 鐪熶唬鐮?26 module + 85 plugin (涓€鎵? | 鉁?娣卞害+10x |
| 鍊熼壌鏉＄洰 | 18 (14 鍊熼壌 + 4 涓嶅€熼壌) | 18 + 7 = 25 (14 鍊熼壌淇 + 4 涓嶅€熼壌 + 7 鏂板) | 鉁?+7 |
| 瀛楁绾у紩鐢?| 浜屾墜 (寮曠敤 "VCP 搂2.11 #3") | 涓€鎵?(寮曠敤 `chatCompletionHandler.js:170-220`) | 鉁?涓€鎵?|
| 淇鏁?| 0 | 5 (#7/#8/#9/#11/#12) | 鉁?璇氬疄 |
| 鏂板瑙勫垯 | 0 | 7 | 鉁?+7 |
| 涓嶅亣瑁呯櫥璁?| 鏃?| 搂6.2.1 琛ㄦ牸鏄庣‘鏍?缂栭€? vs "鐪熷搴? | 鉁?閫忔槑 |

**淇鍚庤惤鍦板師鍒?* (渚?AI 鍥㈤槦鎵ц):
1. **搂6.1 + 搂6.2 骞惰鏈夋晥**: 搂6.1 鍘熻矊淇濈暀 (鍘嗗彶鍐崇瓥),搂6.2 淇椤逛负鐜拌,**浠?搂6.2 涓哄噯**
2. **搂6.1 #1-#6 / #10 / #13 / #14 涓嶅彉** (鍩轰簬鐪熶唬鐮?OK,鏃犻渶閲嶅仛)
3. **搂6.1 #7-#9 / #11-#12 钀藉湴鍓嶅繀璇?搂6.2.1 淇琛?*,閫愬瓧娈垫爣娉ㄦ潵婧?4. **搂6.2.2 #14-#20 鏂板鍊熼壌** 鎴樺焦 1-2 鍐呭疄鐜?5. **搂6.2.3 7 鏉¤鍒?* 鍐欒繘 `CONTRIBUTING.md` 浣滀负闀挎湡瑙勮寖

**瀹屾暣鐪熻鍒嗘瀽**: 瑙?`docs/18-VCP-BORROW-RETROSPECTIVE.md` (18 KB companion 鏂囨。,鍚?5 绉嶉敊璇ā寮?+ 璋冪爺鏃堕棿绾?+ 鍊熼壌鍐崇瓥閫愰」閲嶅 + 鎴樺焦 1-4 琛旀帴)

---

## 搂7. 鎾ゅ洖鍊熼壌 (鐣欑┖, 鎸夐渶杩藉姞)

| 鏃ユ湡 | 椤圭洰 | 鎾ゅ洖鍘熷洜 | 鏇夸唬 |
|------|------|--------|------|
| - | - | - | - |

---

_鏈〃 30 琛屽€熼壌鍐崇瓥 (搂1-搂5) + 搂6.1 18 琛屽～瀹?(R14-D6-B) + 搂6.2 5 椤逛慨姝?+ 7 椤规柊澧?+ 7 鏉¤鍒?(2026-08-04 涓讳汉 12:36 瑙﹀彂);閫氳繃 4 椤瑰摬瀛﹀畧闂?闃舵 4 鐪熸祴鏃朵笉鍐荤粨浠讳綍鍙傛暟;搂6.1 + 搂6.2 骞惰鏈夋晥,**浠?搂6.2 涓哄噯**._
