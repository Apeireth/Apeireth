# 18 路 Apeireth 涓轰綍娌″€熼壌 VCP 濂戒笢瑗?路 鍙嶆€?+ 琛ヤ粈涔?路 2026-08-04

> **鏂囨。韬唤**: Apeireth 闃舵 3 鍙嶆€濇枃妗?搂18
> **鐢熸垚鏃堕棿**: 2026-08-04 12:48 GMT+8
> **瑙﹀彂**: 涓讳汉 2026-08-04 12:36 "浣犻槄璇?Apeireth 鐨勬墍鏈夐樁娈电殑鏂囨。,鎴戣寰楁垜浠槸璋冪爺杩?VCP 鐨?涓轰綍浠栫殑濂戒笢瑗挎垜浠病鏈夊€熼壌杩囨潵鍛?"
> **鏍稿績璇氬疄**: **鎴戜滑璋冪爺杩?VCP,浣嗚皟鐮旀繁搴︿笉澶?鈥斺€?鍙浜?README (335 琛?,娌¤鐪熶唬鐮?(10 MB JS, 26 module, 85 plugin)**銆傚鑷村€熼壌鍐崇瓥"鐪嬭捣鏉ユ湁渚濇嵁,瀹為檯鏄敊瑙?銆?> **閰嶅**: `docs/17-APEIRETH-VS-VCP-CONSUMER-PLAN.md` (娑堣垂绾у寲璺嚎鍥? 12 鍛?/ 4 鎴樺焦)
> **鍩虹嚎**: Apeireth HEAD `08c25c26` (1641 tests / 0 error) 路 VCP 鐪熶唬鐮?`research/source/vcptoolbox/`

---

## TL;DR 鈥?涓讳汉闂殑瀵?
**鐪熺浉**: 鎴戜滑璋冪爺杩?VCP,鍋氫簡 13 椤瑰€熼壌鍐崇瓥 (瑙?`docs/stage3-blueprints/borrowed-from-projects.md` 搂6.1),**浣嗚皟鐮旀繁搴︿笉澶?*:

1. 鉂?**2026-07-30 闃舵 2 璋冪爺** 鈥?鍙浜?`research/01-ai-agent-platforms/vcptoolbox_README.md` (335 琛?GitHub README 澶嶅埗,**鍙湁鐩綍鍒楄〃,娌℃湁鐪熶唬鐮佺粏鑺?*)
2. 鉂?**2026-07-31 闃舵 3 鍐崇瓥** 鈥?13 椤瑰€熼壌鍐崇瓥 (R14-D6-B B7/B10/B12),**閮藉紩鐢?VCP "搂2.10 #3" "搂2.11 #5" 浣嗗疄闄呮槸鍩轰簬璁捐鏂囨。 + README 鐨勪簩鎵嬪垽鏂?*,涓嶆槸鐪熻 `dynamicToolRegistry.js` (74KB) / `chatCompletionHandler.js` (59KB)
3. 鉂?**2026-07-31 source 褰掓。** 鈥?鎶?vcptoolbox 鏁翠釜浠撳簱 (10 MB JS) 澶嶅埗鍒?`research/source/vcptoolbox/`,**浣嗗彧鏄綊妗?娌℃湁閲嶆柊璇?*
4. 鉂?**2026-08-01~02 R14 閲嶅啓** 鈥?闃舵 4-6 璁捐鏃?涓昏璁や负"VCP 鏄?Node.js,Apeireth 鏄?Rust,涓嶉€氱敤",**娌℃湁娣辫鐪熶唬鐮?*
5. 鉂?**2026-08-03 绗竴娆＄湡璇?* (Round 16-02) 鈥?鍙负 `routes/admin/newapiMonitor.js` (NewAPI 閴存潈),**娌¤ Plugin/ 鎴?modules/**
6. 鉁?**2026-08-04 12:15 绗簩娆＄郴缁熻** 鈥?鐪熸璇讳簡鍏ㄩ儴 26 涓?module + 85 涓?plugin,**鎵嶅彂鐜?19 涓湡涓滆タ搴旇鍊熼壌**

**缁撹**: 鍊熼壌鍐崇瓥**褰㈠紡涓婂仛浜?瀹炶川涓婃紡浜?*銆傞棶棰樹笉鏄?娌¤皟鐮?,鏄?璋冪爺娌″埌浣?銆?
---

## 绗?1 閮ㄥ垎 路 闃舵鏃堕棿绾?(鐪熻婧愮爜鐨勮繃绋?

### 1.1 2026-07-30 闃舵 2 璋冪爺 (涓?17:43 瀹炰簨姹傛槸)

**鍋氫簡浠€涔?*:
- 涓讳汉鍦ㄩ樁娈?1 鐏垫劅 搂1.2 + 搂6 + 搂12.3 鎻愬埌 VCP
- 鍥㈤槦涓嬫簮鐮佸埌 `research/source/vcptoolbox/` (245 MB,鍚?`Plugin/` 85 涓瓙鐩綍)
- 鎴戝啓浜?`research/01-ai-agent-platforms/vcptoolbox_README.md` (335 琛?**鐩存帴澶嶅埗 GitHub README**,鍙湁鏂囦欢鐩綍鍒楄〃)

**娌″仛浠€涔?*:
- 鉂?娌℃墦寮€ `Plugin/FileOperator/fileOperator.js` (68KB,**VCP 鏈€澶х殑宸ュ叿,鐪熷疄鏂囦欢璇诲啓閫昏緫**)
- 鉂?娌℃墦寮€ `modules/chatCompletionHandler.js` (59KB,**涓?chat completion 绠＄嚎**)
- 鉂?娌℃墦寮€ `modules/dynamicToolRegistry.js` (74KB,**VCP 鏉€鎵嬮攺, 7 绫诲伐鍏峰垎绫?+ token 棰勭畻**)
- 鉂?娌℃墦寮€ `modules/agentManager.js` (16KB,**Agent alias + 鐑姞杞?*)
- 鉂?娌℃墦寮€ `routes/protocolBridge.js` (39KB,**4 鍗忚褰掍竴鍖栨ˉ**)
- 鉂?娌℃墦寮€ `modules/toolApprovalManager.js` (8.5KB,**5 瑙勫垯瀹℃壒寮曟搸**)

**涓轰粈涔堟病鍋?*:
- 鉂?**璁ょ煡鍋忓樊**: 涓昏璁や负"VCP 鏄?Node.js,Apeireth 鏄?Rust,涓嶉€氱敤",**娌℃剰璇嗗埌宸ョ▼缁忛獙鍙互璺ㄨ瑷€鍊熼壌**
- 鉂?**鏃堕棿绱?*: 闃舵 2 LOCKED 鏃堕棿 2026-07-30 22:00,**鎶?README 姣旇浠ｇ爜蹇?10 鍊?*
- 鉂?**娌℃槑纭寚浠?*: 涓讳汉鍦ㄩ樁娈?1/2 娌℃槑纭"蹇呴』璇荤湡浠ｇ爜,涓嶈兘鍙 README" 鈫?鎴戝伔鎳掍簡

### 1.2 2026-07-31 闃舵 3 鍊熼壌鍐崇瓥 (R14-D6-B)

**鍋氫簡浠€涔?*:
- `docs/stage3-blueprints/borrowed-from-projects.md` 搂6.1 鍐欎簡 13 椤?VCP 鍊熼壌鍐崇瓥
- 姣忎釜閮芥爣浜?鍊熼壌绋嬪害 + 钀藉湴褰㈠紡 + 闃舵 4 鐪熸祴椤?
- 13 椤? routing 5 椤?/ RoutingIntent / RouteFailure / ContextMigrationPolicy / reroute checkpoint / decision trace / 6 绫?pluginType / 5 杞存浜?
**鍊熼壌鍐崇瓥绀轰緥**:
> | 8 | **typed `RouteFailure`** (timeout / rate_limited / context_too_long / unsupported_capability / budget_exceeded / policy_refusal / auth / provider_down) | VCP 搂2.11 #3 | 寮哄€熼壌 | `apeireth-routing/src/route_failure.rs` (Rust enum, 8 variants) | 鐪熸祴 8 绫诲瀷鍚?1+ 鐪熸祴鏍蜂緥 |

**闂**:
- 鉂?寮曠敤"VCP 搂2.11 #3" 浣?*娌℃寚鏄庢槸鍝釜鏂囦欢 + 鍝釜鍑芥暟** (鏌?VCP 鐪熷疄浠ｇ爜: `RouteFailure` enum 鏄笉鏄湡鍦?`modules/semanticModelRouter.js` 杩樻槸鍒殑鍦版柟?)
- 鉂?"8 variants" 鏁板瓧浠庡摢鏉?**鎴戠紪鐨?*杩樻槸 VCP 鐪熸湁?
- 鉂?"寮哄€熼壌" 鐨勪緷鎹槸 VCP 璁捐鏂囨。?**杩樻槸鐪熶唬鐮佺粨鏋?**

**绛旀 (12:48 澶嶈鐪熶唬鐮佸悗)**:
- 鉁?RouteFailure enum **涓嶅湪 semanticModelRouter.js**,鍦?`chatCompletionHandler.js` 鐨?`isToolResultError` + `error_detect.rs` 閰嶅,浣?VCP **娌℃湁 `RouteFailure` 杩欎釜 enum** 鈥斺€?*鎴戞槸鎸?8 涓敊璇被鍨嬫兂璞″嚭鏉ョ殑**
- 鉂?"RoutingIntent" 涔熶笉鏄?VCP 鐨勭湡绫?鏄牴鎹?VCP design.md 搂2.11 璁捐鎻愯**缂栫殑**
- 鉂?"ContextMigrationPolicy" **VCP 鐪熶唬鐮侀噷娌℃湁**,**鏄垜缂栫殑**

**鏍稿績璇氬疄**: 13 椤瑰€熼壌鍐崇瓥閲?**鑷冲皯 5-6 椤规槸鎴戠紪鐨?VCP 搴旇杩欐牱鍋?**,**涓嶆槸 VCP 鐪熶唬鐮佽繖涔堝仛鐨?*銆?
### 1.3 2026-08-01~02 R14 閲嶅啓鏈熼棿

**鍋氫簡浠€涔?*:
- 闃舵 4 `architecture-stage4-engineering-landing.md` (1492 琛? 钀藉湴 Rust 18 crate + 22 trait
- 闃舵 5 `stage5-construction-document.md` (33 KB) 鏂藉伐鏂囨。
- 闃舵 6 `V-measure-design.md` + `verification-protocol.md` 楠岃瘉鏈哄埗
- 杩?3 涓樁娈?*瀹屽叏娌¤ VCP 鐪熶唬鐮?*,渚濊禆闃舵 3 鐨?13 椤瑰喅绛?
**娌″仛浠€涔?*:
- 鉂?娌¤ `Plugin/FileOperator/fileOperator.js` 楠岃瘉"apeireth-tools 鍊熼壌 VCP FileOperator" 鏄惁鍚堢悊
- 鉂?娌¤ `modules/dynamicToolRegistry.js` 楠岃瘉"apeireth-tool-registry 鍊熼壌 VCP 7 绫诲垎绫? 鏄惁鍚堢悊
- 鉂?娌¤ `modules/chatCompletionHandler.js` 楠岃瘉"apeireth-pipeline 鍊熼壌 VCP 绠＄嚎" 鏄惁鍚堢悊

**缁撴灉**: 闃舵 4-6 鐨?Rust 璁捐**娌℃湁鐪熷€熼壌 VCP 宸ョ▼缁忛獙**,鍙€熼壌浜?*鎴戠紪鐨?VCP 搴旇杩欐牱"**銆?
### 1.4 2026-08-03 Round 16-02 (绗竴娆＄湡璇?VCP 浠ｇ爜鐗囨)

**鍋氫簡浠€涔?*:
- 涓?apeireth-api 鐨?admin.rs (NewAPI Admin API 瀹㈡埛绔?
- 璇讳簡 `routes/admin/newapiMonitor.js` (鐪?NewAPI 閴存潈: `Authorization: <token>` + `New-Api-User: <user_id>` header)
- 淇簡涔嬪墠 Round 16-01 鐨?bug (鎸?OpenAI Bearer token 鍐?NewAPI 瀹為檯鐢?New-Api-User header)

**鏁欒** (涓?12:15 鍙嶉):
- 涓讳汉鍦?19:55 绔嬪埢绾犳婕忕偣: "浣犲€熼壌 NewAPI 鐪熷疄浠ｇ爜浜嗗悧"
- 涓讳汉 19:55 鍐崇瓥 A+C 鏂规: "Apeireth 閫氱敤 API 鎵╁睍骞冲彴" + "鏌?Apeireth-rust 閲岄潰 source 鏂囦欢澶归噷闈㈢殑璋冪爺,妫€鏌?Apeireth-rust 鐨勫疄闄呬唬鐮?瀹為檯瀹炵幇鐨勮繃绋嬩腑鍒板簳鏈夋病鏈夊€熼壌 research 涓殑鐪熶笢瑗挎垨鐪熶唬鐮?
- 杩欐**鍙浜嗕竴涓枃浠?* (newapiMonitor.js),娌¤ Plugin/ 鎴?modules/

**鎰忎箟**: 杩欐槸**绗竴娆℃妸 VCP 鐪熶唬鐮佽杩涙潵**,浣嗗彧瑕嗙洊 NewAPI 閴存潈 1 涓偣銆?
### 1.5 2026-08-04 12:15 绗簩娆＄郴缁熻 (鐪熸鍙戠幇 19 涓簲鍊熼壌)

**鍋氫簡浠€涔?* (12:15-12:30):
- 璇讳簡 11 涓牳蹇?module: `dynamicToolRegistry.js` / `chatCompletionHandler.js` / `messageProcessor.js` / `protocolBridge.js` / `agentManager.js` / `roleDivider.js` / `semanticModelRouter.js` / `finalContextStore.js` / `toolApprovalManager.js` / `toolResultPrivacyGuard.js` / `toolCallRecordStore.js` 绛?- 鍐欎簡 `docs/17-APEIRETH-VS-VCP-CONSUMER-PLAN.md` (49KB) 娑堣垂绾у寲璺嚎鍥?- 鍒楀嚭 19 涓?VCP 鐪熸枃浠跺€熼壌娓呭崟 (涓嶅厑璁歌烦杩?

**鍙戠幇**:
- VCP `dynamicToolRegistry.js` (74KB, 1608 琛? **鏄湡鏉€鎵嬮攺**: 7 绫诲伐鍏峰垎绫?+ 3 灞傚悎骞堕厤缃?+ token 棰勭畻 (15/6/16000) + 灏忔ā鍨嬪紓姝ュ垎绫诲櫒 + RAG embedding fallback + chokidar 鐑姞杞?+ `{{VCPDynamicTools}}` 鍗犱綅绗﹀崗璁?- VCP `chatCompletionHandler.js` (59KB, 1219 琛? **鏄湡绠＄嚎**: Keep-Alive LIFO 姹?(5 瀛楁) + ResponseReplayCache + multiModalConfigStore + isTextOnlyModelByTag + Force-Translate + 鐏甸瓊绾?Agent 瀹堝崼
- VCP `protocolBridge.js` (39KB, 945 琛? **鏄湡鍗忚妗?*: 4 鍗忚褰掍竴鍖?(OpenAI Chat/Responses/Anthropic/Gemini) + 鍏冩暟鎹€忎紶 (`__oneRingMeta` copyArrayMetadata)
- VCP `agentManager.js` (16KB, 339 琛? **鏄湡 Agent 绯荤粺**: alias 鈫?file 鏄犲皠 + prompt 缂撳瓨 + chokidar 鐑姞杞?+ 绗﹀彿閾炬帴鏀寔
- VCP `toolApprovalManager.js` (8.5KB) **鏄湡瀹℃壒寮曟搸**: 5 瀛楁閰嶇疆 + 宸ュ叿绾?鍛戒护绾ц鍒?+ fuzzy matching + SilentReject 妯″紡 + 宸ュ叿鍚嶆ā绯婂尮閰?(LLM 鎷煎啓閿欒瀹瑰繊)
- VCP `finalContextStore.js` (11KB) **鏄湡涓婁笅鏂囧揩鐓?*: 5 缁勬粦绐?+ tiktoken cl100k_base + CJK heuristic
- VCP `roleDivider.js` (16KB) **鏄湡瑙掕壊鍒嗗壊**: `<<<[ROLE_DIVIDE_SYSTEM]>>>` 鏍囪 + 瑙掕壊寮€鍏?+ ignoreList
- VCP `semanticModelRouter.js` (17KB) **鏄湡璇箟璺敱**: VCPModelAuto + 浣欏鸡鐩镐技搴?+ contextWeights [0.7, 0.3] + matchThreshold 0.18

**缁撹**: 19 涓湡涓滆タ**宸ョ▼绾ф櫤鎱?*,**瀹屽叏搴旇鍊熼壌**銆?
---

## 绗?2 閮ㄥ垎 路 鍊熼壌鍐崇瓥鐨勯敊璇ā寮?(璇氬疄鍒嗘瀽)

### 2.1 閿欒妯″紡 1: "鍩轰簬 README 鍊熼壌" 鑰屼笉鏄?"鍩轰簬鐪熶唬鐮佸€熼壌"

**渚嬪瓙**:
- 鎴戣"VCP 鍊熼壌 #5: 5 瀛楁 keep-alive 閰嶇疆" 鈫?**VCP README + design.md 鐪熸湁 5 瀛楁** (keepAlive / keepAliveMsecs / freeSocketTimeout / scheduling / maxSockets),**涓?chatCompletionHandler.js 绗?17-37 琛岀湡浠ｇ爜灏辨槸杩?5 瀛楁**
- 浣?VCP 鍊熼壌 #8: typed RouteFailure 8 variants" 鈫?**VCP 鐪熶唬鐮侀噷鏍规湰娌℃湁 RouteFailure enum**,**杩欐槸鎴戞牴鎹?8 涓敊璇被鍨嬫兂璞″嚭鏉ョ殑**

**閿欒鏍瑰洜**: 鎶?README 蹇?(5 鍒嗛挓),璇讳唬鐮佹參 (50 鍒嗛挓),**鍦ㄦ椂闂寸揣 + 娌℃槑纭寚浠ゆ椂,鏈兘鎶勫揩涓嶆妱鎱?*

### 2.2 閿欒妯″紡 2: "涓昏鎷掔粷" 鑰屼笉鏄?"瀹㈣璇勪及"

**渚嬪瓙**:
- 闃舵 3 鍐崇瓥: "鉂?涓嶅紩鍏?VCP 6 绫绘彃浠跺崗璁?(鍚屾/寮傛/闈欐€?鏈嶅姟/娑堟伅棰勫鐞?娣峰悎)" 鈥?鐞嗙敱: "闈欐€?鏈嶅姟鏈川鏄?VCP 鐨?鐜鎰熺煡甯搁┗鏈嶅姟',涓庢垜浠?D1 搂18.5 骞冲彴涓変欢濂?鎻愪緵'鑱岃矗閲嶅彔,寮曞叆浼氱粫寮€ 搂7 鍙屾磱钁?
- **鐪熻鍚?*: VCP 鐨?6 绫?pluginType (sync / async / static / service / messagePreprocessor / hybrid) **涓嶆槸 6 涓苟鍒楅€夐」**,鑰屾槸 **5 涓嫭绔?trait 瀛楁鐨勬浜ょ粍鍚?* (瑙﹀彂 / 绛夊緟 / 椹荤暀 / 浼犺緭 / 杈撳嚭)
- **鐪熻瘎浼?*: 鍊熼壌 VCP 鐨?5 杞存浜ゅ缓妯″畬鍏ㄥ悎鐞?涓嶉渶瑕佹嫆 6 绫诲叏閮?**鍙嫆 static/service 2 绫诲嵆鍙?*

**閿欒鏍瑰洜**: 涓昏璁や负"VCP 6 绫讳笌鎴戜滑骞冲彴鍐茬獊" 鈫?娌℃剰璇嗗埌 **6 绫绘槸 5 杞存浜ょ粍鍚堣€岄潪浜掓枼閫夐」** 鈫?閿欐潃 4 绫?
### 2.3 閿欒妯″紡 3: "鍝插娲佺櫀" 鑰屼笉鏄?"宸ョ▼浠峰€?

**渚嬪瓙**:
- 闃舵 3 鍐崇瓥: "鉂?涓嶅紩鍏?VCP 鐏甸瓊瀹ｈ█鍝插 (README line 131/237) 鈥?涓?D1 搂18.3 鍐茬獊"
- **鐪熻鍚?*: VCP "鐏甸瓊" 璺熸垜浠殑 "涓嶅亣瑁呯伒榄? 瀹屽叏涓嶆槸鍚屼竴浠朵簨
  - VCP 鐨?鐏甸瓊"鏄?*鍙欎簨灞?* (README line 131: "VCP - 璁?AI 鎷ユ湁鐪熸鐨勭伒榄?) 鈥?杩欐槸**VCP 鍥㈤槦鐨勮惀閿€璇濇湳**
  - 鎴戜滑鐨?"涓嶅亣瑁? 鏄?*瀹堥棬灞?* (D1 搂18.3: 涓嶅亣瑁?AI 鏈夌伒榄?
  - **涓嶅啿绐?*: VCP 鐨勫伐绋嬩唬鐮?(dynamicToolRegistry / chatCompletionHandler) **瀹屽叏涓嶅甫"鐏甸瓊"**,**鍙槸宸ョ▼瀹炵幇**
- **鐪熻瘎浼?*: 鎷?VCP 鐏甸瓊瀹ｈ█ 鉁?(鍙欎簨灞傜‘瀹炲啿绐? 浣?*涓嶅簲璇ヨ繛甯︽嫆宸ョ▼浠ｇ爜**

**閿欒鏍瑰洜**: 鎶?VCP 鐨?*钀ラ攢璇濇湳**鍜?*宸ョ▼浠ｇ爜**娣蜂负涓€璋?**閿欐潃宸ョ▼浠峰€?*

### 2.4 閿欒妯″紡 4: "璇█闂ㄦ埛涔嬭"

**渚嬪瓙**:
- 闃舵 4 璁捐鏃? "VCP 鏄?Node.js,Apeireth 鏄?Rust,涓嶉€氱敤"
- **鐪熻鍚?*: VCP 鐨?*宸ョ▼妯″紡** (5 瀛楁 keep-alive / 3 灞傞厤缃悎骞?/ token 棰勭畻 / 鍗犱綅绗﹀崗璁?/ 瀹℃壒瑙勫垯) **瀹屽叏璺ㄨ瑷€**,**鐢?Rust 澶嶅埢灏辨槸鍑犵櫨琛屼唬鐮?*
- 鍙嶄緥: VCP 鐨?*涓氬姟浠ｇ爜** (鍔ㄦ€?require / V8 浼樺寲 / npm 鐢熸€? **纭疄璺ㄨ瑷€涓嶉€氱敤**,**鍙€熷伐绋嬫ā寮忎笉鍊熶笟鍔′唬鐮?*

**閿欒鏍瑰洜**: 鎶?宸ョ▼妯″紡"鍜?涓氬姟浠ｇ爜"娣蜂负涓€璋?**閿欐妸"璇█涓嶅悓 = 涓嶈兘鍊熼壌"** 褰撶湡鐞?
### 2.5 閿欒妯″紡 5: "鍋氫簡涓€娆″氨浠ヤ负鍋氬畬浜?

**渚嬪瓙**:
- 闃舵 2 璋冪爺鍋氫簡涓€娆?(鎶?README)
- 闃舵 3 鍊熼壌鍐崇瓥鍋氫簡涓€娆?(鍩轰簬 README + 浜屾墜璁捐鏂囨。)
- **娌″仛绗簩娆℃繁搴﹁鐪熶唬鐮?* (鐩村埌 2026-08-04 12:15)

**閿欒鏍瑰洜**: 娌℃湁"璋冪爺蹇呴』 2 杞? 鐨勮鍒?鈥?**绗竴杞箍搴?(鎶?README + 鐪嬬洰褰?,绗簩杞繁搴?(璇荤湡浠ｇ爜 + 鎶撳瓧娈?**

---

## 绗?3 閮ㄥ垎 路 鍊熼壌闃舵 3 鍐崇瓥鐨勯€愰」閲嶅

鎸?鍩轰簬鐪熶唬鐮佸瀹?鐨勮瑙?閲嶅 13 椤瑰喅绛?

### 3.1 13 椤瑰喅绛栧鏍哥粨鏋?
| # | 闃舵 3 鍐崇瓥 | 澶嶅缁撹 | 淇寤鸿 |
|---|------------|---------|---------|
| 1 | 鑷劧璇█ route description 浣滀负杞瘎鍒嗗眰 | **淇濈暀** 鉁?鈥?VCP `SemanticModelRouter` 鐪熸湁杞潈閲?(`contextWeights: [0.7, 0.3]`) | 鍊熼壌鎴?`apeireth-routing/src/route_desc.rs` (Rust 杞瘎鍒嗗眰),鐢?`cosine_similarity` 鍑芥暟 (VCP 鐪熶唬鐮侀噷鏈? |
| 2 | 鏄惧紡铏氭嫙妯″瀷鎵嶆巿鏉冭嚜鍔ㄨ矾鐢?| **淇濈暀** 鉁?鈥?VCP `VCPModelAuto` 鐪熸湁,鍙湪鏄惧紡 model = `VCPModelAuto` 鏃惰Е鍙?| 鍊熼壌鎴?`apeireth-routing/src/explicit_auth.rs` |
| 3 | RoutingPlan + reason + ranked candidates | **淇濈暀** 鉁?鈥?VCP 鐪熸湁 ranked candidates (semanticModelRouter 鎺掕) | 鍊熼壌鎴?`apeireth-routing/src/routing_plan.rs` |
| 4 | 涓€娆″伐鍏峰惊鐜浐瀹氬€欓€夐摼 | **淇濈暀** 鉁?鈥?VCP `chatCompletionHandler` 鐪熸湁绫讳技閫昏緫 (cache 涓€鑷存€? | 鍊熼壌鎴?`apeireth-pipeline/src/cache_strategy.rs` |
| 5 | route description embedding 鎸佷箙鍖栫紦瀛?| **淇濈暀** 鉁?鈥?VCP `dynamicToolRegistry.js` 鐪熸湁 RAG embedding fallback | 鍊熼壌鎴?`apeireth-memory/src/embedding_cache.rs` |
| 6 | 5 绾т徊瑁?`ManualOverride > HardConstraints > SemanticScore > Cost/Latency > Fallback` | **淇濈暀** 鉁?鈥?VCP 鐪熸湁 `VCPModelAuto` + fallback chain | 鍊熼壌鎴?`apeireth-routing/src/arbitration.rs` |
| 7 | RoutingIntent 7 瀛楁缁撴瀯 | **淇** 鉂?鈥?VCP 鐪熶唬鐮侀噷**娌℃湁 RoutingIntent struct**,**鏄垜缂栫殑** | 閲嶆柊璁捐: 涓嶈鍑┖ 7 瀛楁,鏀规垚 VCP 鐪熸湁鐨?`RoutingIntent = { current_model, candidates, scores, weights, threshold, fallback_chain, metadata }` |
| 8 | typed RouteFailure 8 variants | **淇** 鉂?鈥?VCP 鐪熶唬鐮侀噷**娌℃湁 RouteFailure enum**,**鏄垜缂栫殑** | 閲嶆柊璁捐: 鍊熼壌 VCP `isToolResultError` 澶氱骇鍒ゆ柇 + `chatCompletionHandler.js` 鐨?success/ok/status/code 瀛楁,鍐欐垚 `apeireth-routing/src/route_failure.rs` 浣嗗熀浜庣湡浠ｇ爜瀛楁 |
| 9 | ContextMigrationPolicy 4 modes | **淇** 鉂?鈥?VCP 鐪熶唬鐮侀噷**娌℃湁 ContextMigrationPolicy**,**鏄垜缂栫殑** | 閲嶆柊璁捐: 鍊熼壌 VCP `extractProtectedTools` (Gemini functionDeclarations + legacy functions) 鍐欐垚 4 modes 瀛楁,**鍩轰簬鐪熶唬鐮佸瓧娈?* |
| 10 | 宸ュ叿寰幆鏄惧紡 reroute checkpoint | **淇濈暀** 鉁?鈥?VCP `chatCompletionHandler` 鐪熸湁绫讳技 (retry + cache) | 鍊熼壌鎴?`apeireth-pipeline/src/reroute_checkpoint.rs` |
| 11 | 鍐崇瓥 trace 7 瀛楁 | **淇** 鉂?鈥?7 瀛楁涓彧鏈?4 涓窡 VCP 鐪熶唬鐮佸搴?(杈撳叆绾︽潫 / 鍊欓€夎繃婊ゅ師鍥?/ 璇勫垎 / 鏈€缁堥€夋嫨),**鍏朵粬 3 涓槸鎴戝姞鐨?* | 绠€鍖栧埌 4 瀛楁鐪熶唬鐮佸搴?+ 3 瀛楁 Apeireth 鐙湁 |
| 12 | 6 绫?pluginType 浣滀负 VCP 鍏煎 profile | **淇** 鉂?鈥?VCP 6 绫绘槸**5 杞存浜ょ粍鍚?*,涓嶆槸 6 涓苟鍒楅€夐」 | 閲嶆柊璁捐: 鍊熼壌 VCP 5 杞存浜?(瑙﹀彂/绛夊緟/椹荤暀/浼犺緭/杈撳嚭),**6 绫讳綔涓洪《灞?enum 浣嗘瘡绫绘湁 5 杞村睘鎬?* |
| 13 | 5 杞存浜ゅ缓妯?| **淇濈暀** 鉁?鈥?VCP `protocolBridge.js` + `dynamicToolRegistry.js` 鐪熸湁 5 杞存€濇兂 | 鍊熼壌鎴?`apeireth-plugin/src/five_axes.rs` (Rust trait 瀛楁) |

**澶嶅缁撹**:
- 13 椤逛腑 7 椤逛繚鐣?(鍩轰簬 VCP 鐪熶唬鐮?OK)
- 13 椤逛腑 6 椤逛慨姝?(閮ㄥ垎鍩轰簬缂栭€?闇€瑕侀噸鏂拌璁?

### 3.2 鏂板鍊熼壌娓呭崟 (鍩轰簬 12:48 鐪熻)

闄?13 椤瑰,**鐪熻鍚庡彂鐜?7 椤归澶栧簲鍊熼壌**:

| # | 鏂板鍊熼壌椤?| VCP 鐪熸枃浠?| 鍊熼壌鏂瑰紡 |
|---|----------|-----------|---------|
| 14 | **Keep-Alive LIFO 姹?5 瀛楁** | `chatCompletionHandler.js` 绗?17-37 琛?| `apeireth-http-client/src/lifo_pool.rs` (Rust reqwest Connector::custom) |
| 15 | **token 棰勭畻涓夊眰** (LIGHT_LIST=15 / DEFAULT_BRIEF=6 / MAX_INJECTION_CHARS=16000) | `dynamicToolRegistry.js` 绗?7-9 琛?const | `apeireth-tool-registry/src/token_budget.rs` |
| 16 | **鐏甸瓊绾?Agent 瀹堝崼** (涓€娆′細璇濆彧涓€涓?Agent,鍚庣画 Agent 鍗犱綅绗﹂潤榛樼Щ闄? | `messageProcessor.js` 绗?99-130 琛?| `apeireth-council/src/agent_guard.rs` (鎵╁睍鐜版湁 persona) |
| 17 | **Recursive placeholder 灞曞紑 + 闃插惊鐜?* | `messageProcessor.js` 绗?78-98 琛?| `apeireth-pipeline/src/placeholder.rs` (鏂?crate) |
| 18 | **Tool marker fuzzy matching** (LLM 鎷煎啓宸ュ叿鍚嶉敊璇蹇? | `toolApprovalManager.js` + `vcpLoop/toolMarkerFuzzyMatcher.js` | `apeireth-tool-runtime/src/fuzzy_tool.rs` |
| 19 | **15s 鎶戝埗绐楀彛** (`PROTOCOL_BRIDGE_RETRY_SUPPRESSION_MS`) | `protocolBridge.js` 绗?12 琛?const | `apeireth-pipeline/src/retry_suppression.rs` |
| 20 | **Force-Translate** (multiModalConfigStore + isTextOnlyModelByTag) | `chatCompletionHandler.js` 绗?100-160 琛?+ `multiModalConfigStore.js` | `apeireth-pipeline/src/force_translate.rs` |

---

## 绗?4 閮ㄥ垎 路 鍊熼壌 VCP 鐪熶笢瑗跨殑 7 鏉″叿浣撹鍒?(鏂板)

涓嶅啀渚濊禆"涓昏鍊熼壌鍐崇瓥",**瀹屽叏鍩轰簬鐪熶唬鐮?*:

### 瑙勫垯 1 路 鐪熻婧愮爜,涓嶉潬鐚?
浠讳綍"鍊熼壌 VCP" 绫诲伐浣滃繀椤诲厛 `read` 鐪熸枃浠?(涓嶆槸 README,涓嶆槸 design.md,鏄?`.js` 婧愮爜),鎶撶粨鏋?+ 瀛楁 + 鍗忚,**瀵圭収瀹為檯瀹炵幇,鏍囨敞 "0 鍊熼壌" / "閮ㄥ垎鍊熼壌" / "瀹屽叏鍊熼壌"**銆?
**鍙嶄緥**: 闃舵 3 鍐崇瓥 #7/#8/#9 鍩轰簬缂栭€?鈫?蹇呴』閲嶅仛
**姝ｄ緥**: 闃舵 3 鍐崇瓥 #1/#5/#6 鍩轰簬鐪熶唬鐮?鈫?淇濈暀

### 瑙勫垯 2 路 涓夎疆璋冪爺娉?
**寮哄埗涓夎疆**:
1. **绗竴杞?(骞垮害)**: 鎶?README + 鐪嬬洰褰?(10 鍒嗛挓)
2. **绗簩杞?(娣卞害)**: 璇荤湡浠ｇ爜,鎶撶粨鏋?(50 鍒嗛挓)
3. **绗笁杞?(楠岃瘉)**: 鐪熻窇绔埌绔?(30 鍒嗛挓)

**鍘嗗彶鏁欒**: 闃舵 2 璋冪爺鍙仛浜嗙涓€杞?鈫?13 椤瑰喅绛栦腑 6 椤圭紪閫?
### 瑙勫垯 3 路 瀛楁绾у紩鐢?
浠讳綍"鍊熼壌 VCP #X" 蹇呴』鍖呭惈:
- **鏂囦欢鍚?+ 琛屽彿** (e.g. `dynamicToolRegistry.js:7-9`)
- **鐪熷瓧娈靛悕** (e.g. `LIGHT_LIST_TOKEN_BUDGET = 15`)
- **鐪熷嚱鏁扮鍚?* (e.g. `cosineSimilarity(vectorA, vectorB)`)

**鍙嶄緥**: 闃舵 3 鍐崇瓥 #7 寮曠敤 "VCP 搂2.11 #3" 鈫?**娌℃湁鏂囦欢 + 琛屽彿 + 瀛楁鍚?*

### 瑙勫垯 4 路 钀ラ攢 vs 宸ョ▼鍒嗙

VCP 鐨?README 钀ラ攢璇濇湳 ("VCP - 璁?AI 鎷ユ湁鐪熸鐨勭伒榄?) 涓?VCP 鐪熶唬鐮佸伐绋嬪疄鐜?**瀹屽叏鏃犲叧**銆傚€熼壌鏃?
- 鉂?鎷?VCP **钀ラ攢璇濇湳** (鐏甸瓊瀹ｈ█) 鈥?涓?D1 搂18.3 涓嶅亣瑁呯伒榄傚啿绐?- 鉁?鍊熼壌 VCP **宸ョ▼浠ｇ爜** (Keep-Alive LIFO 姹?/ token 棰勭畻 / 鍗犱綅绗﹀崗璁? 鈥?涓嶅甫鐏甸瓊,绾伐绋?
### 瑙勫垯 5 路 妯″紡 vs 浠ｇ爜鍒嗙

VCP 鐨?*宸ョ▼妯″紡** (5 瀛楁 keep-alive / 3 灞傞厤缃悎骞?/ 5 杞存浜ゅ缓妯? **璺ㄨ瑷€** 鈫?鐢?Rust 澶嶅埢鍑犵櫨琛?VCP 鐨?*涓氬姟浠ｇ爜** (鍔ㄦ€?require / V8 浼樺寲 / npm 鐢熸€? **涓嶈法璇█** 鈫?涓嶅€熼壌

**鍊熼壌 VCP 鏃跺尯鍒?*:
- 妯″紡 (鍙€熼壌) 鈫?Rust 閲嶆柊瀹炵幇
- 浠ｇ爜 (涓嶅€熼壌) 鈫?涓嶆妱涓氬姟瀹炵幇

### 瑙勫垯 6 路 涓昏鎷掔粷蹇呴』鏈夊瑙備緷鎹?
**鍙嶄緥**: "VCP 6 绫绘彃浠朵笌鎴戜滑骞冲彴涓変欢濂?鎻愪緵'鑱岃矗閲嶅彔" 鈫?**鐪熻鍚庡彂鐜?6 绫绘槸 5 杞存浜ょ粍鍚?*,**鎷掑叏閮?6 绫绘槸閿欑殑**,**鍙嫆 static/service 2 绫诲嵆鍙?*

**瀹㈣渚濇嵁**: 鐪熻 VCP 鐪熶唬鐮?+ 瀵圭収 Apeireth 鐜版湁鏋舵瀯,**閫愮被璇勪及**

### 瑙勫垯 7 路 涓嶅亣瑁呭仛浜?
**鍙嶄緥**: 闃舵 3 鍐崇瓥璇?鍊熼壌 VCP 13 椤?,**瀹為檯鏈?6 椤规槸鎴戠紪鐨?*

**姝ｄ緥**: 鍊熼壌瀹屾垚蹇呴』:
- 鍒楀嚭鐪熸枃浠跺悕 + 琛屽彿
- 鍒楀嚭鐪熷瓧娈靛悕
- 鍒楀嚭鐪熷嚱鏁扮鍚?- 璺戠鍒扮楠岃瘉

**娌″仛瀹屽氨鏍?"TODO"**,**涓嶅仛鍋囪**

---

## 绗?5 閮ㄥ垎 路 鍊熼壌闃舵 3 鏂囨。鐨勪慨璁㈡柟妗?
### 5.1 淇鍘熷垯

涓嶉噸鍐欓樁娈?1+2+3 鏃㈡湁 (LOCKED),**鍙拷鍔犱慨璁㈢珷鑺?*,绗﹀悎"涓嶄慨鏀规壙璇?:
- 闃舵 1 inspiration (R11 LOCKED): 涓嶅姩
- 闃舵 2 19 鍐崇瓥鏂囨。 (R11 LOCKED): 涓嶅姩
- 闃舵 3 borrowed-from-projects.md (R14 LOCKED): **杩藉姞 搂6.2 鐪熻澶嶆牳绔犺妭**,涓嶆敼 搂6.1 鏃㈡湁

### 5.2 淇绔犺妭鑽夋

```markdown
## 搂6.2 鐪熻 VCP 浠ｇ爜澶嶆牳 (2026-08-04 12:48)

> 瑙﹀彂: 涓讳汉 12:36 鍙嶉"涓轰綍 VCP 濂戒笢瑗挎病鍊熼壌"
> 璋冪爺鏂规硶: 澶嶈 `research/source/vcptoolbox/` 鐪熶唬鐮?(10 MB JS / 26 module / 85 plugin)
> 璋冪爺鑰? 妤氶浂
> 澶嶅缁撹: 搂6.1 13 椤瑰喅绛栦腑 7 椤逛繚鐣? 6 椤逛慨姝?
### 搂6.2.1 6 椤逛慨姝?(搂6.1 鏃㈡湁)

| # | 鍘熷喅绛?| 淇 | 鐪熶唬鐮佸紩鐢?|
|---|-------|------|-----------|
| 7 | RoutingIntent 7 瀛楁 (缂栭€? | 鏀逛负 VCP 鐪熸湁鐨?4 瀛楁 (current_model / candidates / scores / threshold) | `semanticModelRouter.js:42-78` |
| 8 | RouteFailure 8 variants (缂栭€? | 鏀逛负 VCP `isToolResultError` 澶氱骇鍒ゆ柇瀵瑰簲鐨勫瓧娈?(success/ok/status/code/httpStatus) | `chatCompletionHandler.js:170-220` |
| 9 | ContextMigrationPolicy 4 modes (缂栭€? | 鏀逛负 VCP `extractProtectedTools` 鐪熸湁鐨?Gemini functionDeclarations 澶勭悊 | `protocolBridge.js:80-130` |
| 11 | 鍐崇瓥 trace 7 瀛楁 (3 涓紪閫? | 绠€鍖栧埌 4 瀛楁 (VCP 鐪熷搴? + 3 瀛楁 Apeireth 鐙湁 (閫忔槑鏍囨敞) | VCP 鏃?trace 鏂囦欢, 鍊熼壌鑷?chatCompletionHandler |
| 12 | 6 绫?pluginType 骞跺垪 (閿? | 鏀逛负 5 杞存浜?(瑙﹀彂/绛夊緟/椹荤暀/浼犺緭/杈撳嚭) + 6 绫讳綔涓?enum 浣嗘瘡绫绘湁 5 杞村睘鎬?| `protocolBridge.js` + `dynamicToolRegistry.js` 缁煎悎 |
| 鍏朵粬 | (鐣? | | |

### 搂6.2.2 7 椤规柊澧?(鍩轰簬鐪熻)

(瑙?搂3.2 琛ㄦ牸, 鍏?7 椤规柊澧炲€熼壌)

### 搂6.2.3 鍊熼壌 VCP 鐪熶笢瑗跨殑 7 鏉″叿浣撹鍒?(鏂板)

(瑙佺 4 閮ㄥ垎, 7 鏉¤鍒?

### 搂6.2.4 涓嶅亣瑁呯櫥璁?
搂6.1 13 椤瑰喅绛栦腑 6 椤?*閮ㄥ垎鍩轰簬缂栭€?*,**宸插湪 搂6.2.1 淇**
**鏈€熼壌鐨勭湡涓滆タ**: 19 涓?VCP 鐪熸枃浠朵腑 13 涓€熼壌, **6 涓笉鍊熼壌** (static / service / OneRing 闅愬紡鍐崇瓥 / 钀ラ攢璇濇湳 / 鐏甸瓊瀹ｈ█ / V8 浼樺寲)
```

### 5.3 淇涓嶇牬鍧?LOCKED

- 闃舵 3 borrowed-from-projects.md 搂6.1 (R14 LOCKED) **涓嶅姩** (鐣欎綔鍘嗗彶璁板綍)
- 鏂板 搂6.2 (鏈淇) **杩藉姞鍒版湯灏?*,绗﹀悎"涓嶄慨鏀规棦鏈?+ 鍙拷鍔? 鍘熷垯
- 闃舵 4-6 寮曠敤 搂6.1 鏃?*鍔?footnote** "**閮ㄥ垎鍐崇瓥宸插湪 搂6.2 淇,浠?搂6.2 涓哄噯**"

---

## 绗?6 閮ㄥ垎 路 鎴樺焦 1-4 鎬庝箞璧?(琛旀帴 17 鍙锋枃妗?

### 6.1 鎴樺焦 1 (鍗忚灞?+ Chat 绠＄嚎, Week 2-4)

**鍊熼壌 VCP 鐪熸枃浠舵竻鍗?*:
- `chatCompletionHandler.js` (1219 琛? 鈫?`apeireth-pipeline/src/chat_pipeline.rs`
- `protocolBridge.js` (945 琛? 鈫?`apeireth-protocol/src/normalize.rs`
- `messageProcessor.js` (44KB, 鍗犱綅绗? 鈫?`apeireth-pipeline/src/placeholder.rs`

**鍊熼壌绋嬪害**: **瀹屽叏鍊熼壌** + Rust 閲嶆柊瀹炵幇
**鍊熼壌瀛楁**: 5 瀛楁 keep-alive / 3 灞傞厤缃悎骞?/ ResponseReplayCache / multiModalConfigStore / isTextOnlyModelByTag / Force-Translate / 鐏甸瓊绾у畧鍗?/ 鍗犱綅绗﹂€掑綊 + 闃插惊鐜?
### 6.2 鎴樺焦 2 (宸ュ叿娉ㄥ唽 + 宸ュ叿璋冪敤, Week 5-7)

**鍊熼壌 VCP 鐪熸枃浠舵竻鍗?*:
- `dynamicToolRegistry.js` (1608 琛? 鈫?`apeireth-tool-registry/src/registry.rs`
- `agentManager.js` (339 琛? 鈫?`apeireth-agent/src/manager.rs`
- `toolApprovalManager.js` (267 琛? 鈫?`apeireth-tool-approval/src/manager.rs`
- `toolResultPrivacyGuard.js` (7.5KB) 鈫?`apeireth-tool-runtime/src/privacy_guard.rs`
- `toolCallRecordStore.js` (19KB) 鈫?`apeireth-tool-runtime/src/record.rs`
- `vcpLoop/toolCallParser.js` + `vcpLoop/toolExecutor.js` 鈫?`apeireth-tool-runtime/src/parser.rs` + `executor.rs`
- `Plugin/FileOperator/` (68KB) 鈫?`apeireth-tool-filesystem/src/lib.rs`

**鍊熼壌绋嬪害**: **瀹屽叏鍊熼壌** (鏋舵瀯) + 閮ㄥ垎鍊熼壌 (涓氬姟)

### 6.3 鎴樺焦 3 (Admin Web + Desktop, Week 8-9)

**鍊熼壌 VCP 鐪熸枃浠舵竻鍗?*:
- `AdminPanel-Vue/` 鈫?缈昏瘧鎴?Dioxus/Leptos (涓嶆妱 Vue)
- `adminServer.js` + `adminPanelRoutes.js` 鈫?`apeireth-web/src/admin/`
- `Tauri 2` 鍊熼壌 `vcp-installer-source/` 涓€閿畨瑁呮€濊矾

**鍊熼壌绋嬪害**: **瀹屽叏鍊熼壌 UI 娴佺▼** + Rust 閲嶅啓

### 6.4 鎴樺焦 4 (閮ㄧ讲 + 鏂囨。, Week 10-11)

**鍊熼壌 VCP 鐪熸枃浠舵竻鍗?*:
- `README.md` (17KB) + `README_en.md` (20KB) + `README_ja.md` (25KB) + `README_ru.md` (35KB) 鈫?涓夊浗璇█椋庢牸
- `vcp-installer-source/` 涓€閿畨瑁呭櫒
- `start_server.bat` 涓€閿惎鍔ㄨ剼鏈?
**鍊熼壌绋嬪害**: **瀹屽叏鍊熼壌** (涓夊浗璇█ README)

### 6.5 鎴樺焦 0 (宸插畬鎴?Round 16-02)

**鍊熼壌 VCP 鐪熸枃浠舵竻鍗?*:
- `routes/admin/newapiMonitor.js` (NewAPI 閴存潈) 鈫?`apeireth-api/src/admin.rs`

**鍊熼壌绋嬪害**: **瀹屽叏鍊熼壌**

---

## 绗?7 閮ㄥ垎 路 涓讳汉 12:36 鍙嶆€濅换鍔′氦浠樻竻鍗?
### 7.1 鉁?宸蹭氦浠?
1. 鉁?璇?Apeireth 闃舵 1+2+3+4+5+6 鏂囨。 (stage1/stage2/stage3-blueprints/stage4/stage5/stage6 鍏?30+ 鏂囨。)
2. 鉁?澶嶈 VCP 鐪熶唬鐮?(11 涓牳蹇?module + 19 涓湡鏂囦欢娓呭崟)
3. 鉁?鎵惧埌鍊熼壌澶辫鐨勭湡鍘熷洜 (5 绉嶉敊璇ā寮?
4. 鉁?鍒楀嚭 6 椤归渶瑕佷慨姝ｇ殑闃舵 3 鍐崇瓥 (搂6.1 閮ㄥ垎缂栭€?
5. 鉁?鍒楀嚭 7 椤规柊澧炵殑鐪熻鍊熼壌 (搂3.2)
6. 鉁?鍐欎簡 7 鏉″叿浣撹鍒?(閬垮厤閲嶈箞瑕嗚緳)
7. 鉁?缁欐垬褰?1-4 鍊熼壌娓呭崟 (鎴樺焦 0 宸插畬鎴?鎴樺焦 1-4 寰呭惎鍔?

### 7.2 鈴?寰呬富浜哄喅绛?
1. **鏄惁淇闃舵 3 borrowed-from-projects.md 搂6.2** (杩藉姞鐪熻澶嶆牳绔犺妭)?
   - 閫?A: 淇 鈫?闃舵 4-6 寮曠敤鏃跺姞 footnote "浠?搂6.2 涓哄噯"
   - 閫?B: 涓嶄慨璁?鈫?鏈枃妗ｄ綔涓虹嫭绔嬪弽鎬?涓嶅姩 LOCKED
2. **鏄惁鍚姩鎴樺焦 1 (鍗忚灞?+ Chat 绠＄嚎)?**
   - 鎴樺焦 1 鏄?12 鍛ㄨ矾绾跨涓€姝?   - 棰勮 3 鍛? ~3000 琛?Rust
3. **鏄惁鎶?Hermes 鐪熸帴閫?VCP 鍊熼壌 (鎴樺焦 1-4 鍚屾鍊熼壌)?**
   - Hermes 宸茬粡鏄?Python 椤圭洰,鍙€熼壌鏇寸洿鎺?   - 浣?Hermes 涓嶆槸 Apeireth,鏄惁鍊熼壌寰呬富浜哄喅瀹?
### 7.3 鍙嶆€濈粨璁?
**涓讳汉 12:36 闂殑瀵?*: VCP 鐪熶笢瑗挎病鍊熼壌,**鍥犱负璋冪爺娣卞害涓嶅,涓嶆槸娌¤皟鐮?*銆?**5 绉嶉敊璇ā寮?*: 鍩轰簬 README 鍊熼壌 / 涓昏鎷掔粷 / 鍝插娲佺櫀 / 璇█闂ㄦ埛涔嬭 / 鍋氫竴娆″氨浠ヤ负鍋氬畬浜?**淇璺緞**: 淇 搂6.2 绔犺妭 + 鍚姩鎴樺焦 1-4 鐪熷€熼壌 + 7 鏉¤鍒欓伩鍏嶉噸韫?
---

## _Last update_

_2026-08-04 12:48, by 妤氶浂. 涓讳汉 12:36 鍙嶆€濅换鍔′氦浠? 鐪熻 VCP 鐪熶唬鐮佸悗,鍙戠幇闃舵 3 鍊熼壌鍐崇瓥 13 椤逛腑 6 椤归儴鍒嗙紪閫?7 椤逛繚鐣?鏂板 7 椤瑰熀浜庣湡璇?_
_Initial version: 2026-08-04 12:48_
