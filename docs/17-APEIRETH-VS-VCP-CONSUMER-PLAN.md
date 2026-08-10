# 17 路 Apeireth vs VCP 娣卞害瀵规瘮 + 娑堣垂绾у寲璁″垝

> **鏂囨。韬唤**: Apeireth 闃舵 2 璁捐鏂囨。 搂17
> **鐢熸垚鏃堕棿**: 2026-08-04 12:15 GMT+8
> **瑙﹀彂**: 涓讳汉 2026-08-04 12:15 "鎴戜篃瑕佹妸 Apeireth 鍋氭垚娑堣垂绾х殑,涓嶈鍋滅暀鍦ㄥ疄楠屽鍜屽彛澶?VCP 鎵€鏈夊ソ鐨勪笢瑗块兘鎯冲姙娉曞€熼壌杩囨潵,浣犲嚭涓€涓繁搴﹀姣?璁″垝鏂囨。"
> **鏂规硶**: 娣卞害璇?`research/source/vcptoolbox/` 鐪熸簮鐮?(1.4k+ 鏂囦欢, 26 涓牳蹇?module, 85 涓?plugin),瀵圭収璇?Apeireth-rust 28 crate 鐪熷疄浠ｇ爜,涓嶉潬鐚?> **鍩虹嚎**: Apeireth HEAD `08c25c26` (1641 tests / 0 error / 190 warning) 路 VCP `research/source/vcptoolbox/` 鍏ㄩ儴宸茶惤鍦?
---

## TL;DR

**Apeireth 鏄粈涔堢幇鐘?*: 涓€鍫嗗摬瀛﹀寲鍛藉悕 (consciousness/perception/onion/sovereignty/life-force) 鐨勭粏绮掑害 Rust crate,姒傚康鍏堣繘浣?**娌℃湁鐪熸鑳借窇鐨勪骇鍝佸舰鎬?* 鈥斺€?娌℃湁 chat completion pipeline銆佹病鏈夊姩鎬佸伐鍏锋敞鍐屻€佹病鏈変細璇濈鐞嗐€佹病鏈?web admin銆佹病鏈?plugin 绯荤粺銆佹病鏈夌湡瀹炵鍒扮 LLM 搴旂敤灞傘€俙apeireth-api` 4 涓?endpoint 鏄鏋?`apeireth-cli` 26KB 浣嗗彧鏄?CLI wrapper,`apeireth-tools` 鏁翠釜 lib.rs 鍙湁 1 涓崰浣嶅嚱鏁般€?
**VCP 鏄粈涔堢幇鐘?*: 涓€涓?*鐪熷湪鐢熶骇璺戠殑 AI Agent 缃戝叧** 鈥斺€?26 涓牳蹇?module 瀹屾暣瑕嗙洊 LLM 缃戝叧鍏ㄦ爤 (chat completion / tool registry / agent manager / 鍗忚妗?/ 瑙掕壊鍒嗗壊 / 璇箟璺敱 / final context / 瀹℃壒 / 闅愮 / 鏃ュ織 / 閲嶆斁),85 涓?plugin 鐪熷共娲?(FileOperator/PowerShellExecutor/ChromeBridge/CodeSearcher/BilibiliFetch/ArxivDailyPapers/...),`config.env` 38KB 鐪熷疄閰嶇疆,3 鍥借瑷€ README,VCP.md 55KB 璁捐鏂囨。銆?
**鏍稿績鍒ゆ柇**: Apeireth 鐨?**鍝插鍜岃鐭ユ娊璞?* (council/sovereignty/philosophy/涓夊眰 V1+V2+V3 瀹堥棬) 鏄?VCP 娌℃湁鐨勭湡涓滆タ;VCP 鐨?**宸ョ▼鍖栥€佸崗璁眰銆佽繍缁淬€佹秷璐逛綋楠?* 鏄?Apeireth 缂虹殑銆備袱鑰呭悎鎴?= 娑堣垂绾с€?
**娑堣垂绾у寲璺緞**: 12 鍛?/ 5 闃舵 / 4 澶ф垬褰广€?*涓嶈 30 涓?crate 閮藉仛婊?*,浼樺厛鎵?4 涓骇鍝佸舰鎬佸叆鍙?(chat / tool registry / agent runtime / admin web),鍏朵綑鍝插鎶借薄淇濈暀浣滀负鍐呴儴 advisor銆?
---

## 绗?1 閮ㄥ垎 路 娣卞害瀵规瘮 (VCP 鐪熶唬鐮?vs Apeireth 鐪熶唬鐮?

### 1.1 鏁板瓧瀵规瘮

| 缁村害 | VCP (vcptoolbox) | Apeireth (rust) | 宸窛 |
|---|---|---|---|
| 浠ｇ爜浣撻噺 | ~10 MB JS (1.4k+ 鏂囦欢) | ~2.6 MB Rust (~50 鏂囦欢) | VCP 4x |
| 鏍稿績 module | 26 涓?(modules/*.js) | 28 涓?crate (浣嗗緢澶氱┖澹? | 褰㈠紡骞?鍐呭宸?|
| Plugin / 宸ュ叿 | **85 涓湡鎻掍欢** (Plugin/*/) | 1 涓崰浣?(apeireth-tools 800B lib.rs) | **VCP 85x** |
| LLM 缃戝叧 | 瀹屾暣 chat completion (chatCompletionHandler.js 59KB) | 4 涓?endpoint (server.rs 9KB) | VCP 6.5x |
| 鍗忚閫傞厤 | Responses/Anthropic/Gemini 涓夊崗璁ˉ (protocolBridge.js 39KB) | 鍗曞崗璁?(OpenAI compat + Anthropic) | VCP 3x |
| 鍔ㄦ€佸伐鍏锋敞鍐?| dynamicToolRegistry.js 74KB (鍒嗙被+token棰勭畻+灏忔ā鍨嬪垎绫诲櫒+鐑洿鏂? | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 宸ュ叿璋冪敤鎵ц | vcpLoop/toolCallParser+toolExecutor (鐪熻窇) | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 宸ュ叿瀹℃壒 | toolApprovalManager.js (5 瑙勫垯,宸ュ叿绾?鍛戒护绾?妯＄硦鍖归厤,chokidar 鐑姞杞? | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| Agent 绠＄悊 | agentManager.js (alias 鈫?file 鏄犲皠,鎵弿/缂撳瓨/鐑姞杞?鍗犱綅绗﹂€掑綊灞曞紑,闃插惊鐜?鐏甸瓊绾у畧鍗? | apeireth-council 7 advisor (涓嶅悓姒傚康,涓嶆槸 agent) | 姒傚康涓嶅悓 |
| 涓婁笅鏂囩鐞?| finalContextStore.js (5 缁勬粦绐?+ tiktoken) | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 瑙掕壊鍒嗗壊 | roleDivider.js (<<<[ROLE_DIVIDE_*]>>> 鏍囪,閫掑綊鍒囧垎) | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 璇箟妯″瀷璺敱 | semanticModelRouter.js (VCPModelAuto, 浣欏鸡鐩镐技搴? preset 璺敱) | apeireth-asi (鍖楁瀬鏄熷鍚? 鍝插姒傚康) | 姒傚康涓嶅悓 |
| 绯荤粺鎻愮ず绠＄悊 | sarPromptManager.js | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 闅愮淇濇姢 | toolResultPrivacyGuard.js (鏁忔劅瀛楁 mask) | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 鏃ュ織閲嶆斁 | vcpLogReplayManager.js (19KB) | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 璋冪敤璁板綍 | toolCallRecordStore.js + toolCallRecordInternalFilter.js | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 鏃ヨ绯荤粺 | dailynote/ + dailyNotesRoutes.js 40KB | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 璁哄潧 | forumApi.js 24KB | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 娴忚鍣ㄨ嚜鍔ㄥ寲 | browserRuntimeManager.js 26KB + ChromeBridge/UrlFetch | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 澶氭ā鎬侀厤缃?| multiModalConfigStore.js + Force-Translate (image_url鈫抰ext tag) | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| 涓婁笅鏂囨姌鍙?| foldProtocol.js + ContextFoldingV2/ + DynamicToolBridge/ | 鉂?娌℃湁 | **瀹屽叏缂哄け** |
| Admin Web UI | AdminPanel-Vue (瀹屾暣 Vue 椤圭洰) + adminServer.js + adminPanelRoutes.js | apeireth-web (鍒氬紑,鏃犲唴瀹? + apeireth-desktop (鍒氬紑,鏃犲唴瀹? | **瀹屽叏缂哄け** |
| Desktop App | 鉂?| apeireth-desktop (鏂? 鏃犲唴瀹? | Apeireth 鐙湁 (浣嗙┖) |
| TUI | 鉂?| apeireth-tui (鏂? | Apeireth 鐙湁 (浣嗙┖) |
| Council 7 advisor | 鉂?| 鉁?(ethics/history/legal/performance/philosophy/safety/strategy) | **Apeireth 鐙湁** |
| 涓夊眰 V1+V2+V3 瀹堥棬 | 鉂?| 鉁?(apeireth-asi/philosophy/sovereignty) | **Apeireth 鐙湁** |
| Sovereignty hook 鎶借薄 | 鉂?| 鉁?(SovereigntyHook trait) | **Apeireth 鐙湁** |
| 鎸変綇鏈哄埗 (30%/60s/3 杞? | 鉂?| 鉁?(apeireth-council/hold.rs) | **Apeireth 鐙湁** |
| PyO3 鍏煎妗?| 鉂?| 鉁?(apeireth-pybridge) | **Apeireth 鐙湁** |
| 娴嬭瘯 | tests/ (VCP) | 1641 tests passed (Apeireth) | 鏁板瓧褰㈠紡涓?Apeireth 澶?|

### 1.2 VCP 鍏抽敭妯″潡鍓栨瀽 (浠庣湡浠ｇ爜鎶藉嚭鏉?

#### 1.2.1 `modules/dynamicToolRegistry.js` (74KB,1608 琛? 鈥?鍔ㄦ€佸伐鍏锋敞鍐屼腑蹇?
VCP 鐨勬潃鎵嬮攺銆傛牳蹇冩満鍒?

1. **涓夊眰鍚堝苟閰嶇疆**: `DEFAULT_CONFIG` (纭紪鐮? 鈫?`config.env` (鏂囦欢) 鈫?`manualOverrides` (鐢ㄦ埛缂栬緫),娣卞悎骞?鏃?BOM,鏃?YAML銆?2. **鍒嗙被绯荤粺**: 7 涓?category (search / file_code / image_media / memory_knowledge / agent_task / communication / data),姣忎釜鏈夊弻璇叧閿瘝琛?(涓嫳鏂?,鐢?tokenSet 鍖归厤銆?3. **token 棰勭畻鎺у埗**:
   - `LIGHT_LIST_TOKEN_BUDGET = 15` (杞诲垪琛ㄦ渶澶?15 token)
   - `DEFAULT_BRIEF_TOKEN_BUDGET = 6` (鎻忚堪鏈€澶?6 token)
   - `MAX_INJECTION_CHARS = 16000` (鍗曟娉ㄥ叆 鈮?16KB)
4. **灏忔ā鍨嬪垎绫诲櫒**: 鐢?`smallModel` 閰嶇疆(鐙珛 endpoint),鎸?debounce 1000ms + timeout 30000ms 寮傛鍒嗙被,閬垮厤闃诲涓昏姹傘€?5. **RAG embedding fallback**: `useRagEmbeddings=true` 鏃剁敤鍚戦噺鐩镐技搴︽帓搴?plugin,鑰屼笉鏄叧閿瘝銆?6. **鐑洿鏂?*: config 鏂囦欢鍙樺寲瑙﹀彂 reload銆?7. **鍗犱綅绗﹀崗璁?*: `{{VCPDynamicTools}}` 鍦?system prompt 涓綔涓烘敞鍏ョ偣,鍒嗙被鍚庢妸鍛戒腑鐨?plugin 鎻忚堪娉ㄥ叆銆?8. **鍗忚绔偣褰掍竴鍖?*: `normalizeOpenAIChatEndpoint` 鎶?`/v1` `/chat/completions` `/v1/chat/completions` 閮借鏁存垚鏍囧噯 endpoint銆?
> **Apeireth 缂轰粈涔?*: 瀹屽叏娌℃湁銆俙apeireth-tools` 鏄┖澹?娌℃湁 registry銆佹病鏈夊垎绫汇€佹病鏈?token 棰勭畻銆佹病鏈夊皬妯″瀷鍒嗙被鍣ㄣ€佹病鏈?RAG fallback銆佹病鏈夊崰浣嶇鍗忚銆?
#### 1.2.2 `modules/chatCompletionHandler.js` (59KB,1219 琛? 鈥?涓?chat completion 绠＄嚎

瀹屾暣绠＄嚎:

1. **Keep-Alive 杩炴帴姹?*: 鑷缓 `http.Agent({ keepAlive: true, freeSocketTimeout: 8000, scheduling: 'lifo', maxSockets: 10000 })`,**鏉€鎺?8 绉掔┖闂?socket 闃叉鍍靛案杩炴帴** (涓讳汉鍦?2026-07-13 鏁欒閲屽氨閬囪繃 "1M socket hang up" 鐜拌薄,VCP 鐩存帴鐢?keep-alive 姹犺В鍐?銆?2. **ResponseReplayCache**: 甯?messageId 鐨勫搷搴旈噸鏀剧紦瀛?(闃查噸澶嶈姹?,鏈€杩?100 鏉?LRU銆?3. **multiModalConfigStore 鐪熺浉婧?*: JSON 浼樺厛 + 鐑洿鏂?`MultiModalForceTranslateModels` 鍒楄〃鎺у埗鍝簺鍚庣妯″瀷闇€瑕佹妸 base64 image 缈昏瘧鎴愭枃鏈?閬垮厤 deepseek/GLM 涓嶆敮鎸佸妯℃€佹椂 400 閿欒)銆?4. **`isTextOnlyModelByTag`**: 妫€娴嬪綋鍓嶇湡瀹炲悗绔ā鍨嬫槸鍚﹀懡涓函鏂囨湰 tag 鍒楄〃 (deepseek-v4 / GLM-4.5),鑷姩 base64 鈫?text銆?5. **`messagesContainBase64Media`**: 鍙湁鐪熷甫鍥炬椂鎵嶈Е鍙戠炕璇戞彃浠躲€?6. **`consumeVcpToolUseForbiddenPlaceholder`**: 妫€娴?system 鎻愮ず閲岀殑 `[[VCPToolUse=Forbidden]]` 鍗犱綅绗?鍏佽澶栭儴瀹㈡埛绔鐢ㄥ伐鍏疯皟鐢?,**鍙壂鎻忛娈佃繛缁?system** 闃茶瑙﹀彂銆?7. **`copyArrayMetadata`**: 澶嶅埗闈炴灇涓炬暟缁勫厓鏁版嵁 (OneRing 鍦?messages 鏁扮粍涓婃寕 `__oneRingMeta`,浠讳綍杩斿洖鏂版暟缁勭殑姝ラ蹇呴』鏄惧紡淇濈暀)銆?8. **`isToolResultError`**: 宸ュ叿缁撴灉閿欒妫€娴?(success/ok/status/code 瀛楁澶氱骇鍒ゆ柇,閬垮厤涓氬姟鏂囨湰閲岀殑"閿欒"鍏抽敭璇嶈鍒?銆?
> **Apeireth 缂轰粈涔?*: 瀹屽叏娌℃湁銆俙apeireth-api/server.rs` 9KB 鍙湁 4 涓?endpoint,娌℃湁绠＄嚎銆佹病鏈夐噸鏀俱€佹病鏈?keep-alive 姹犮€佹病鏈夐敊璇娴嬨€?
#### 1.2.3 `modules/messageProcessor.js` (44KB,762 琛? 鈥?鍗犱綅绗﹁В鏋愬紩鎿?
鏍稿績:

1. **鍗犱綅绗﹁娉?*: `{{alias}}` 鎴?`{{agent:alias}}` / `{{toolbox:alias}}`,CJK + 鎷変竵鍏ㄦ敮鎸併€?2. **鐗规潈瑙掕壊瀹堝崼**: Agent/Toolbox 鍗犱綅绗?*鍙湪 system 娑堟伅鍜?[绯荤粺鎻愮ず:]/[绯荤粺閭€璇锋寚浠?] 寮€澶寸殑 user 娑堟伅閲屽睍寮€**,闃叉鐢ㄦ埛鍦ㄦ櫘閫?user 娑堟伅閲岄€氳繃 `{{agent:master}}` 娉ㄥ叆璇诲彇 master prompt銆?*杩欐槸鐏甸瓊绾у畨鍏?*銆?3. **閫掑綊灞曞紑 + 闃插惊鐜?*: `processingStack` 璺熻釜宸插睍寮€ alias,寰幆寮曠敤妫€娴?鐢熸垚 `[Error: Circular agent reference detected for 'xxx']`銆?4. **One Agent 鐏甸瓊绾у畧鍗?*: `context.expandedAgentName !== undefined` 鏃?鍚庣画 Agent 鍗犱綅绗?*鍏ㄩ儴闈欓粯绉婚櫎** (鍙兘灞曞紑涓€涓?Agent)銆?5. **Toolbox 鍘婚噸**: `context.expandedToolboxes: Set`,鍚屽悕 toolbox 鍙睍寮€涓€娆°€?6. **Static Fold Mode**: `[[VCPStaticFold::Auto|Lite|Full]]` 鏍囪,鎸?threshold 鎺掑簭閫?fold_blocks[0]銆?7. **System notification 鍓ョ**: 鑷姩璇嗗埆 `[绯荤粺閫氱煡:...] ... [绯荤粺閫氱煡缁撴潫]` 鍧?浠庢渶鍚庝竴鏉?user message 绉婚櫎 (閬垮厤閲嶅瑙﹀彂)銆?
> **Apeireth 缂轰粈涔?*: 瀹屽叏娌℃湁銆俙apeireth-council/persona.rs` 鏈?鐙珛 session + persona + 绔嬪満 + 鍙京璁?3 杞?,**杩欐槸鍝插鐗?Agent**,浣?*娌℃湁 alias 绯荤粺銆佹病鏈夊崰浣嶇銆佹病鏈夊惊鐜娴嬨€佹病鏈?system 閫氱煡鍓ョ**銆?
#### 1.2.4 `modules/agentManager.js` (16KB,339 琛? 鈥?Agent 娉ㄥ唽 + 鐑姞杞?
鏍稿績:

1. **alias 鈫?file 鏄犲皠**: `agent_map.json` 鍔犺浇,鍐呭瓨 Map銆?2. **promptCache**: 璇昏繃鐨?agent prompt 缂撳瓨,鏂囦欢鍙樺寲鑷姩 invalidate銆?3. **chokidar 鐑姞杞?*: Agent 鐩綍鏂囦欢 add/change/unlink 瀹炴椂鎵弿,`agent_map.json` change 閲嶆柊鍔犺浇銆?4. **绗﹀彿閾炬帴鏀寔**: `entry.isSymbolicLink()` 妫€娴?璺熺湡瀹炶矾寰勩€?5. **閫掑綊鎵弿**: `scanDirectory` 閫掑綊鏀堕泦鎵€鏈?`.txt` / `.md` 鏂囦欢 + 鏂囦欢澶圭粨鏋勩€?6. **debug 妯″紡**: 鍚姩鏃舵墦鍗版墍鏈?agent + 鏂囦欢澶规爲銆?
> **Apeireth 缂轰粈涔?*: 娌℃湁 alias 鈫?file 绯荤粺,娌℃湁 prompt 缂撳瓨,娌℃湁鐑姞杞姐€?
#### 1.2.5 `modules/toolApprovalManager.js` (8.5KB,267 琛? 鈥?宸ュ叿瀹℃壒

鏍稿績:

1. **閰嶇疆椹卞姩**: `toolApprovalConfig.json` 鎺у埗,5 瀛楁:
   - `enabled` (鎬诲紑鍏?
   - `timeoutMinutes` (榛樿 5,瓒呮椂闃诲)
   - `approveAll` (寮哄埗鍏ㄥ)
   - `approvalList` (瑙勫垯鍒楄〃)
   - `fuzzyToolMatching` (妯＄硦鍖归厤)
   - `privacyProtection.enabled`
2. **瑙勫垯璇硶**: 
   - `ToolName` 鈥?宸ュ叿绾у尮閰?   - `ToolName:command` 鈥?鍛戒护绾у尮閰?(specificity 2,浼樺厛浜庡伐鍏风骇)
   - `ToolName::SilentReject` 鈥?鎷掔粷鏃堕潤榛?涓嶉€氱煡 AI
3. **`extractCommands`**: 鑷姩妫€娴?`command` / `command1` / `command2`... 瀛楁,鎺掑簭鎻愬彇銆?4. **`parseApprovalRule`**: 瑙ｆ瀽瑙勫垯瀛楃涓?鎻愬彇 silent suffix銆?5. **`getApprovalDecision`**: 涓夊眰鍒ゆ柇 鈫?enabled 鈫?approveAll 鈫?approvalList 鍖归厤,杩斿洖 specificity 鏈€楂樼殑瑙勫垯銆?6. **chokidar 鐑姞杞?*: config 鏂囦欢鍙樺寲鑷姩 reload銆?7. **toolMarkerFuzzyMatcher**: 妯＄硦鍖归厤宸ュ叿鍚?(VCP 鐙湁,閫傚簲 LLM 杈撳嚭鐨勫伐鍏峰悕鎷煎啓閿欒)銆?
> **Apeireth 缂轰粈涔?*: 瀹屽叏娌℃湁銆俙apeireth-sovereignty` 鏄摬瀛︽娊璞?娌℃湁 JSON 閰嶇疆,娌℃湁瑙勫垯璇硶,娌℃湁 chokidar 鐑姞杞?娌℃湁 fuzzy matching銆?
#### 1.2.6 `routes/protocolBridge.js` (39KB,945 琛? 鈥?鍗忚妗?
鏍稿績:

1. **涓夊崗璁叆鍙?*:
   - **OpenAI Responses API** (`/v1/responses`, native)
   - **OpenAI Chat Completions** (`/v1/chat/completions`)
   - **Anthropic Messages API** (`/v1/messages`, native)
   - **Gemini GenerateContent** (`/v1beta/models/...:generateContent`)
2. **褰掍竴鍖?*: `normalizeTextContent` / `normalizeMessageRole` / `normalizeToolParameters` / `normalizeToolChoice`,鎵€鏈夊崗璁兘杞垚鍐呴儴鏍囧噯 messages 鏁扮粍銆?3. **`extractProtectedTools`**: 鎻愬彇鍘熺敓 tool 瀛楁 (Gemini `functionDeclarations` / `functions` legacy),**鍙墠鍚戜紶閫?涓嶈繘鍏?messages/RAG**銆?4. **`copyArrayMetadata`**: 閫忎紶 OneRing `__oneRingMeta`銆?5. **璇锋眰鍘婚噸**: `recentResponsesRequests: Map<key, timestamp>`,15s 鎶戝埗绐楀彛,閬垮厤 OpenAI Responses 鍋跺彂鐨?5xx 閲嶈瘯椋庢毚銆?6. **缁熶竴杞彂**: 鎵€鏈夊崗璁兘杩?`chatCompletionHandler` 涓婚摼璺?**瀵瑰鎴风閫忔槑鍙敤鍏ㄩ儴 VCP 鑳藉姏**(鎻掍欢/RAG/瑙掕壊鍒嗗壊)銆?
> **Apeireth 缂轰粈涔?*: `apeireth-api/llm/providers/` 5 涓?provider (openai_compat / anthropic_compat / scripted / apeireth_api / 涓€涓?stub),浣?*娌℃湁鍗忚妗?*銆傚鎴风鍙兘鐢ㄦ湰鍗忚鏍煎紡璋?璺ㄥ崗璁繀椤昏嚜宸辫浆銆?
#### 1.2.7 `modules/roleDivider.js` (16KB,304 琛? 鈥?瑙掕壊鍒嗗壊

鏍稿績:

1. **鏍囪璇硶**: `<<<[ROLE_DIVIDE_SYSTEM]>>>...<<<[END_ROLE_DIVIDE_SYSTEM]>>>` 涓夎鑹层€?2. **瑙掕壊寮€鍏?*: `switches: { system, assistant, user }` 鈥?鍏抽棴鏌愯鑹?鈫?绉婚櫎瀵瑰簲鏍囩銆?3. **scan 寮€鍏?*: `scanSwitches` 鈥?鎺у埗鏄惁鎵弿璇ヨ鑹茬殑鍐呭銆?4. **淇濈暀 ignoreList**: 鏌愪簺鍐呭淇濈暀鍘熸牱,涓嶅垎鍓层€?5. **鍏冩暟鎹€忎紶**: `copyArrayMetadata` 淇濈暀 `__oneRingMeta`銆?
> **Apeireth 缂轰粈涔?*: 瀹屽叏娌℃湁銆俻hilosophy 鏄畧闂?涓嶆槸瑙掕壊鍒嗗壊銆?
#### 1.2.8 `modules/semanticModelRouter.js` (17KB,408 琛? 鈥?璇箟妯″瀷璺敱

鏍稿績:

1. **VCPModelAuto 妯″紡**: 鐢ㄦ埛璇锋眰 鈫?璇箟鍖归厤 鈫?鑷姩閫夋ā鍨嬨€?2. **config**: `SemanticModelRouter.json` (鐑姞杞?,`presets` 瀛楀吀,姣忎釜 preset 鏈?`defaultModel` + `fallbackModels` + `routes[]`銆?3. **浣欏鸡鐩镐技搴?*: `cosineSimilarity(vectorA, vectorB)` 鐩存帴绠椼€?4. **`contextWeights: [0.7, 0.3]`**: 鍘嗗彶瀵硅瘽 70%,鏈€杩?user message 30% 鍔犳潈銆?5. **`findLastMessageText`**: 鎶芥渶鍚庝竴鏉?user message 鏂囨湰銆?6. **matchThreshold: 0.18**: 浣欏鸡鐩镐技搴﹂槇鍊?浣庝簬姝ゅ€艰蛋 fallback銆?7. **`autoModelName: 'VCPModelAuto'`**: 瀹㈡埛绔彂杩欎釜 model name 鏃惰矾鐢辫Е鍙戙€?
> **Apeireth 缂轰粈涔?*: `apeireth-asi` 鏄摬瀛﹀寳鏋佹槦,**瀹屽叏涓嶅悓鐨勬蹇?*銆侫peireth 缂虹殑鏄伐绋嬪寲鐨?鎸夌敤鎴锋剰鍥捐嚜鍔ㄩ€?model"銆?
#### 1.2.9 `modules/finalContextStore.js` (11KB,273 琛? 鈥?鏈€缁堜笂涓嬫枃蹇収

鏍稿績:

1. **5 缁勬粦绐?*: `MAX_SNAPSHOTS = 5`,`snapshots: Array`,鏈€鏂?unshift 鍐欏叆銆?2. **tiktoken 绮剧‘璁℃暟**: `@dqbd/tiktoken` 鐨?`cl100k_base`,澶辫触 fallback 鍒?CJK heuristic銆?3. **CJK heuristic**: CJK 瀛楃 + 鎷変竵璇?+ symbol/3 + 1.08 鍊嶃€?4. **base64 瀛楄妭闀垮害**: `getBase64ByteLength`,鐢ㄤ簬澶氭ā鎬?token 浼扮畻銆?5. **Admin 鎷夊彇**: `listFinalContexts()` / `getFinalContextById(id)` 缁欑鐞嗛潰鏉裤€?
> **Apeireth 缂轰粈涔?*: 瀹屽叏娌℃湁 final context snapshot,娌℃湁 token 璁℃暟,娌℃湁 admin 鎷夊彇鎺ュ彛銆?
#### 1.2.10 鍏朵粬鍏抽敭妯″潡 (绠€琛?

| 妯″潡 | LOC | 鏍稿績鑱岃矗 | Apeireth 瀵瑰簲 |
|---|---|---|---|
| `browserRuntimeManager.js` | 26KB | 娴忚鍣ㄨ嚜鍔ㄥ寲 + 闃?captcha + 鎴浘 | 鉂?鏃?|
| `toolCallRecordStore.js` | 19KB | 宸ュ叿璋冪敤鍏ㄨ褰?(鐢ㄤ簬瀹¤/鍥炴斁/缁熻) | 鉂?鏃?|
| `vcpLogReplayManager.js` | 19KB | 鏃ュ織閲嶆斁 (閲嶈窇鍘嗗彶浼氳瘽) | 鉂?鏃?|
| `multiModalConfigStore.js` | 10KB | 澶氭ā鎬佹ā鍨嬬櫧鍚嶅崟 + Force-Translate | 鉂?鏃?|
| `messageProcessor.js` | 44KB | 鍗犱綅绗﹁В鏋?+ 鐏甸瓊绾?Agent 瀹堝崼 | 鉂?鏃?|
| `chatCompletionHandler.js` | 59KB | 涓?chat completion 绠＄嚎 | `apeireth-api/server.rs` 9KB (1/6) |
| `toolApprovalManager.js` | 8.5KB | 宸ュ叿瀹℃壒瑙勫垯寮曟搸 | 鉂?鏃?|
| `toolResultPrivacyGuard.js` | 7.5KB | 宸ュ叿杩斿洖闅愮瀛楁 mask | 鉂?鏃?|
| `toolboxManager.js` | 6.6KB | Toolbox 娉ㄥ唽 + Fold Protocol | 鉂?鏃?|
| `captchaDecoder.js` | 2KB | 楠岃瘉鐮佽В鐮?(ddddocr) | 鉂?鏃?|
| `agentManager.js` | 16KB | Agent alias + 鐑姞杞?| `apeireth-council` 7 advisor (涓嶅悓姒傚康) |
| `foldProtocol.js` | 2.3KB | Fold Blocks 鍗忚 | 鉂?鏃?|
| `dynamicToolRegistry.js` | 74KB | 鍔ㄦ€佸伐鍏锋敞鍐?+ 鍒嗙被 + token 棰勭畻 | 鉂?鏃?|
| `finalContextStore.js` | 11KB | 鏈€缁堜笂涓嬫枃蹇収 | 鉂?鏃?|
| `roleDivider.js` | 16KB | 瑙掕壊鍒嗗壊鏍囪 | 鉂?鏃?|
| `semanticModelRouter.js` | 17KB | 璇箟妯″瀷璺敱 | 鉂?鏃?|
| `sarPromptManager.js` | 5.4KB | SAR prompt 绠＄悊 (鍩虹 system prompt) | 鉂?鏃?|
| `associativeDiscovery.js` | 8KB | 鑱旀兂鍙戠幇 (鍩轰簬 RAG 鐨勭浉鍏充笂涓嬫枃) | 鉂?鏃?|
| `tvsManager.js` | 3.3KB | TVS (Topic-Variable-System) 绠＄悊 | 鉂?鏃?|
| `vcpLoop/toolCallParser.js` | ? | 瑙ｆ瀽 LLM 杈撳嚭鐨?<tool_call>XML | 鉂?鏃?|
| `vcpLoop/toolExecutor.js` | ? | 鐪熸墽琛?tool call | 鉂?鏃?|
| `handlers/streamHandler.js` | ? | SSE 娴佸紡鍝嶅簲 | 鉂?鏃?|
| `handlers/nonStreamHandler.js` | ? | 闈炴祦寮忓搷搴?| 鉂?鏃?|

### 1.3 VCP 鍏抽敭 Plugin (浠庣湡浠ｇ爜鎶藉嚭鏉?鎸戞湁鍊熼壌浠峰€肩殑)

| Plugin | 鑱岃矗 | 鍊熼壌鏂瑰紡 |
|---|---|---|
| `FileOperator` | 鏂囦欢璇诲啓/鎼滅储/缂栬緫 (68KB) | Apeireth 蹇呴』鏈?`apeireth-tool-filesystem` |
| `PowerShellExecutor` | PowerShell 鐪熸墽琛?| `apeireth-tool-shell` (璺ㄥ钩鍙?shell) |
| `LinuxShellExecutor` | Linux shell 鐪熸墽琛?| 鍚屼笂鍚堝苟 |
| `ChromeBridge` | Chrome 妗ユ帴 (鐪熸祻瑙堝櫒) | `apeireth-tool-browser` |
| `UrlFetch` | URL fetch + html 杞?markdown | `apeireth-tool-webfetch` |
| `CodeSearcher` | 浠ｇ爜璇箟鎼滅储 (ripgrep) | `apeireth-tool-codesearch` |
| `BilibiliFetch` | B 绔欒棰?寮瑰箷鎶撳彇 | `apeireth-tool-bilibili` (娑堣垂绾ф潃鎵? |
| `DailyNote` + `DailyNoteManager` + `DailyNotePanel` + `DailyNoteSearcher` | 鏃ヨ鍏ㄦ爤 | `apeireth-memory-dailynote` |
| `ArxivDailyPapers` | 姣忔棩 arxiv 璁烘枃 | `apeireth-tool-arxiv` |
| `CrossRefDailyPapers` | CrossRef 璁烘枃 | 鍚屼笂 |
| `PaperReader` | 璁烘枃闃呰鍣?| `apeireth-tool-paperreader` |
| `AnimeFinder` | 鍔ㄦ极鏌ユ壘 | `apeireth-tool-anime` |
| `FlashDeepSearch` | 娣卞害鎼滅储 | `apeireth-tool-deepsearch` |
| `AnySearch` | 閫氱敤鎼滅储 | `apeireth-tool-anysearch` |
| `DailyHot` | 姣忔棩鐑偣 | `apeireth-tool-dailyhot` |
| `NCBIDatasets` | NCBI 鏁版嵁闆?| `apeireth-tool-ncbi` |
| `ComfyUIGen` / `FluxGen` / `DoubaoGen` / `GPTImageGen` / `GeminiImageGen` / `AgnesGen` / `AgnesVideoGen` / `DMXDoubaoGen` / `NanoBananaGen2` | AI 鐢熷浘/鐢熻棰?(9 涓? | `apeireth-tool-image-gen` (涓€涓彃浠跺妯″瀷) |
| `LightMemo` | 杞婚噺璁板繂 (绫讳技 LightRAG) | `apeireth-memory-lightmemo` |
| `OneRing` | 娑堟伅鍗忚皟鍣?(One Ring to rule them all) | `apeireth-core-onering` |
| `MagiAgent` | 澶?agent 鍗忚皟 | `apeireth-agent-magi` |
| `OpenHerPersona` | 浜鸿绠＄悊 | `apeireth-agent-persona` |
| `ImageFileServer` | 鍥剧墖鏂囦欢 HTTP server | 闆嗘垚鍒?`apeireth-web` |
| `ImageProcessor` | 鍥剧墖澶勭悊 (鍘嬬缉/杞牸寮? | `apeireth-tool-image-process` |
| `FileListGenerator` / `FileTreeGenerator` / `EmojiListGenerator` | 鍒楄〃鐢熸垚鍣?| 闆嗘垚鍒?`apeireth-tool-filesystem` |
| `CapturePreprocessor` | 鎴浘棰勫鐞?| `apeireth-tool-capture` |
| `DynamicToolBridge` | 鍔ㄦ€佸伐鍏锋ˉ (璺?dynamicToolRegistry 閰嶅悎) | `apeireth-tool-bridge` |
| `ContextFoldingV2` | 涓婁笅鏂囨姌鍙?V2 | `apeireth-context-folding` |
| `DigitalOracle` | 鏁板瓧鍗犲崪/鐜勫 | (鍙€?娑堣垂绾у姞) |
| `ArtistMatcher` | 鑹烘湳瀹跺尮閰?| (鍙€? |
| `DistributedMusicDiarySync` | 鍒嗗竷寮忛煶涔愭棩璁板悓姝?| (鍙€?娑堣垂绾у姞) |

### 1.4 VCP 璁捐鍝插鎻愬彇

**VCP.md 55KB + design.md 34KB + AgentDream.md 9.8KB** 鏍稿績鎬濇兂:

1. **宸ュ叿鍗虫彃浠?(Tool as Plugin)**: 姣忎釜鑳藉姏涓€涓嫭绔?plugin,manifest.json 鎻忚堪,chokidar 鐑姞杞?鐪熻繍琛屽湪鑷繁鐨勫瓙杩涚▼/鍑芥暟閲屻€?2. **鍗忚鍗崇湡鐩?(Protocol as Truth)**: 浠讳綍瀹㈡埛绔?(OpenAI/Anthropic/Gemini) 杩涙潵閮借鏁存垚鍐呴儴 messages,鎵€鏈夋彃浠跺瀹㈡埛绔€忔槑銆?3. **瀹堥棬鍒嗗眰 (Defense in Depth)**: 
   - **L0** 宸ュ叿瀹℃壒 (浜虹被浠嬪叆)
   - **L1** VCP 瑙掕壊鍒嗗壊 + System 瀹堝崼
   - **L2** 宸ュ叿缁撴灉闅愮 mask
   - **L3** 鏁忔劅鐜鍙橀噺 (`sensitiveEnv.js`)
4. **鎶樺彔鍗忚 (Fold Protocol)**: 涓婁笅鏂囧お闀跨殑 fold_blocks 鍗忚,鎶?system prompt 鎸?threshold 鎶樺彔,鍑忓皯 token銆?5. **One Agent 鐏甸瓊**: 涓€娆′細璇濆彧鍏佽灞曞紑涓€涓?Agent,闃叉澶氫釜 Agent 浜掔浉鎵撴灦銆?6. **VCPModelAuto**: 瀹㈡埛绔笉闇€瑕佹寚瀹氭ā鍨?VCP 鑷姩鎸夎涔夎矾鐢遍€夈€?
### 1.5 Apeireth 鐪熸鐨勪紭鍔?(VCP 娌℃湁鐨?

| 浼樺娍 | 鏉ユ簮 | 浠峰€?|
|---|---|---|
| **7 寮哄埗 Council advisor** | `apeireth-council/` | 澶氳瑙掑畧闂?姣?VCP 鍗曞眰瀹℃壒寮?|
| **鎸変綇鏈哄埗 (30%/60s/3 杞?** | `apeireth-council/hold.rs` | 鎷熶汉鍖栧崥寮?|
| **涓夊眰 V1+V2+V3 AND 闂?* | `apeireth-asi/philosophy/sovereignty` | 鍝插瀹堥棬 + 鏉冮檺娲嬭懕 + 榛樿涓嶅亣璁?|
| **Sovereignty Hook trait** | `apeireth-sovereignty` | 鍙彃鎷斾富鏉?(璋佽兘 override 璋? |
| **PyO3 鍏煎妗?* | `apeireth-pybridge` | 璋?Python 鐢熸€?(LangChain/LlamaIndex) |
| **Rust 鎬ц兘 + 绫诲瀷瀹夊叏** | workspace 鏁翠綋 | VCP 鏄?Node.js,鍐呭瓨 + 鍚姩鎱?|
| **compiling-time const assert** | 澶?crate 閮芥湁 | VCP 娌℃湁,瀹规槗鍑哄洖褰?|
| **1650+ 娴嬭瘯 + CI** | `apeireth-test` | VCP 娴嬭瘯鍒嗘暎,娌＄湅鍒伴泦涓?CI |
| **鏃?BOM JSON / UTF-8 寮哄埗** | 澶氭枃浠跺己鍒?| VCP 鍦?2026-07-13 鍥?BOM 缈昏溅 |
| **Cargo.lock LOCKED** | `APEIRETH-VERSIONING.md` | VCP package-lock 缁忓父婊?|

> **鏍稿績 insight**: Apeireth 鐨?鍝插鍖栧懡鍚? (consciousness/perception/onion/sovereignty/life-force) 鍚捣鏉ユ娊璞?浣?*鐪熷疄鏈夊唴瀹?*鈥斺€攃ouncil/hold/synthesis/sovereignty 閮界湡璺戜簡 1641 娴嬭瘯銆?*涓嶈鍥犱负鍚嶅瓧鍝插鍖栧氨鎶涘純杩欎簺 crate**,瀹冧滑鏄?Apeireth 鐨?*鐪熸姢鍩庢渤**銆俈CP 娌℃湁杩欎竴灞傘€?
---

## 绗?2 閮ㄥ垎 路 娑堣垂绾у寲鍒ゆ柇 (鐜扮姸宸窛 + 浼樺厛绾?

### 2.1 浠€涔堟槸"娑堣垂绾?

涓讳汉鍘熻瘽:"涓嶈鍋滅暀鍦ㄥ疄楠屽鍜屽彛澶?銆?
娑堣垂绾?= 婊¤冻浠ヤ笅 4 鏉?
1. **鏅€氱敤鎴疯兘鐢?* 鈥斺€?瑁呬竴涓簩杩涘埗,璺戜竴涓懡浠?鏈?web UI/Desktop UI,5 鍒嗛挓鍐呰兘鐢?2. **鏈夌湡瀹炰环鍊?* 鈥斺€?涓嶆槸 demo,鐪熻兘 chat / 鐪熻兘璋冨伐鍏?/ 鐪熻兘璺?agent loop / 鐪熸湁 admin 鐩戞帶
3. **鍙娴嬪彲杩愮淮** 鈥斺€?鏃ュ織 / 鐩戞帶 / 閿欒鎻愮ず / 鍗囩骇璺緞
4. **鍙墿灞?* 鈥斺€?鍔犱竴涓?plugin 涓嶉渶瑕佹敼 core,鍔犱竴涓?LLM provider 涓嶉渶瑕佹敼 client

### 2.2 Apeireth 褰撳墠绂绘秷璐圭骇鐨勮窛绂?
| 缁村害 | 褰撳墠鐘舵€?| 璺濈娑堣垂绾?|
|---|---|---|
| 瀹夎 | `cargo build --release` 缂栬瘧 30 绉?| 鉂?涓嶅彲鐢?(鏅€氱敤鎴蜂笉瑁?Rust) |
| 鍚姩 | `cargo run -p apeireth-api --example serve` | 鉂?涓嶅彲鐢?(娌℃湁 CLI 涓€閿惎鍔? |
| UI | `apeireth-web`/`apeireth-desktop` 绌虹洰褰?| 鉂?瀹屽叏娌℃湁 |
| Chat | `POST /v1/chat/completions` 楠ㄦ灦 (娌＄绾? | 鉂?涓嶅彲鐢?(娌″伐鍏?娌′笂涓嬫枃/娌℃祦寮? |
| 宸ュ叿 | `apeireth-tools` 800B 鍗犱綅 | 鉂?瀹屽叏娌℃湁 |
| 閰嶇疆 | 娌℃湁 config 鏂囦欢,娌?env schema | 鉂?涓嶅彲鐢?|
| 鏃ュ織 | 娌℃湁 tracing 杈撳嚭瑙勮寖 | 鉂?涓嶅彲鐢?|
| 閿欒澶勭悊 | 閮ㄥ垎 try,娌＄粺涓€ error type | 鉂?|
| 鏂囨。 | ROADMAP.md 10KB 浣嗗叏鏄唴閮?roadmap | 鈿狅笍 鐢ㄦ埛鏂囨。 0 |
| 閮ㄧ讲 | 娌℃湁 deploy 鑴氭湰 | 鉂?|
| 鍗囩骇 | 娌℃湁 versioning 绛栫暐 (R11 LOCKED 鏂囨。绾︽潫) | 鈿狅笍 鏂囨。鏈?浜岃繘鍒舵病鏈?|

**缁撹**: Apeireth 鐜板湪鏄?**"鑳借窇鐨勫簱 + 鍝插鍘熷瀷"**,涓嶆槸"浜у搧"銆傛秷璐圭骇鍖栭渶瑕?**4 澶ф垬褰?+ 12 鍛?*銆?
---

## 绗?3 閮ㄥ垎 路 4 澶ф垬褰?+ 12 鍛ㄨ矾绾垮浘

### 鎬昏

```
鎴樺焦 0: 鐪熸帴閫?LLM (鏈懆)                    鈫?2 鍛?(鍩虹)
鎴樺焦 1: 鍗忚灞?+ Chat 绠＄嚎 (CTO round 17)     鈫?3 鍛?(鏍稿績)
鎴樺焦 2: 宸ュ叿娉ㄥ唽 + 宸ュ叿璋冪敤 (CTO round 18)     鈫?3 鍛?(鏍稿績)
鎴樺焦 3: Admin Web UI + Desktop App            鈫?2 鍛?(娑堣垂绾у叆鍙?
鎴樺焦 4: 閮ㄧ讲 + 鏂囨。 + 1.0 release             鈫?2 鍛?(闂幆)
```

**涓嶄慨 LOCKED 鍐呭** (R11 baseline / V0.5 / V1136 / V3 9 閿?/ 1100 妯″潡 / Cargo.lock / apeireth-legacy/)銆?**鏂板 crate OK** (v15+ 鍙犲姞)銆?**8 椤逛笉淇敼鎵胯缁х画瀹堜綇**銆?
### 鎴樺焦 0 路 鐪熸帴閫?LLM (鏈懆,Week 1) 鈥?**宸插畬鎴?Round 16-02**

**鐩爣**: LLM 鐪熻窇閫?浣滀负鍚庣画鎴樺焦鐨勪緷璧栧熀纭€銆?**宸插仛** (commit `f898a5f1`):
- 鉁?`apeireth-api` crate: LLM client (OpenAI Chat + Anthropic Messages 鍙屽崗璁?
- 鉁?5 涓?provider: OpenAI compat / Anthropic compat / Scripted / ApeirethApi / Stub
- 鉁?28 tests passed / 0 failed
- 鉁?admin.rs (NewAPI Admin API 瀹㈡埛绔? 鍊熼壌 VCP `routes/admin/newapiMonitor.js` 鐪熶唬鐮?
- 鉁?hello_api / admin_demo / router_demo / config_demo 4 涓?example

**寰呬富浜?*:
- `APEIRETH_API_KEY` 鐜鍙橀噺 (鐢ㄤ簬 hello_api 鐪熻窇)
- `NEWAPI_ADMIN_ACCESS_TOKEN` + `NEWAPI_ADMIN_API_USER_ID` (鐢ㄤ簬 admin_demo 鐪熻窇)

**鍊熼壌鏉ユ簮**: `research/source/vcptoolbox/routes/admin/newapiMonitor.js` (NewAPI 鐪熷疄閴存潈: `Authorization: <token>` + `New-Api-User: <user_id>` header, NOT OpenAI Bearer)銆?
### 鎴樺焦 1 路 鍗忚灞?+ Chat 绠＄嚎 (Week 2-4) 鈥?鎴樺焦 1

**鐩爣**: 鐪熸敮鎸?OpenAI Chat Completions + Anthropic Messages + Google Gemini (VCP 涓夊崗璁?,閫忔槑缁熶竴 chat completion 绠＄嚎,鍚?keep-alive 姹?/ 娴佸紡 / 閲嶆斁 / 澶氭ā鎬?/ token 浼扮畻銆?
**瀛愪换鍔?*:

#### Week 2: 鍗忚妗?+ 娴佸紡鍝嶅簲
- 鏂板缓 `crates/apeireth-protocol/`:
  - `openai_chat.rs` 鈥?OpenAI Chat Completions 鍗忚
  - `openai_responses.rs` 鈥?OpenAI Responses API 鍗忚
  - `anthropic_messages.rs` 鈥?Anthropic Messages API 鍗忚
  - `gemini_generate_content.rs` 鈥?Google Gemini 鍗忚
  - `normalize.rs` 鈥?鍗忚闂存秷鎭?宸ュ叿褰掍竴鍖?  - `metadata.rs` 鈥?`__oneRingMeta` 绛夊厓鏁版嵁閫忎紶
- `apeireth-api` 鏂板 `axum` route:
  - `POST /v1/chat/completions` (OpenAI)
  - `POST /v1/responses` (OpenAI Responses)
  - `POST /v1/messages` (Anthropic)
  - `POST /v1beta/models/{model}:generateContent` (Gemini)
  - 鎵€鏈夎矾寰勮繘鍚屼竴涓?chat pipeline
- **鍊熼壌**: `routes/protocolBridge.js` (945 琛? + `chatCompletionHandler.js` (1219 琛?
- **娴嬭瘯**: 50+ (鍗忚闂翠簰杞?+ 閫忎紶鍏冩暟鎹?+ 閿欒澶勭悊)

#### Week 3: 娴佸紡鍝嶅簲 + Keep-Alive 姹?- `crates/apeireth-http-client/` (鏂?:
  - `reqwest` 鍖呰 + Keep-Alive Agent (澶嶅埢 VCP `freeSocketTimeout: 8000` + `lifo scheduling` + `maxSockets: 10000`)
  - `lifo_socket_pool.rs` (VCP lifo 璋冨害绠楁硶)
- `apeireth-api` 娴佸紡鍝嶅簲:
  - `SSE` (Server-Sent Events) 娴佸紡 (VCP `handlers/streamHandler.js`)
  - 娴佸紡閿欒鎭㈠ (VCP `stream_max_retries`)
  - 娴佸紡 idle timeout (VCP `DEFAULT_STREAM_IDLE_TIMEOUT_MS: 300000`)
- **鍊熼壌**: `chatCompletionHandler.js` 绗?17-37 琛?keep-alive agent + `DEFAULT_STREAM_*` 甯搁噺
- **娴嬭瘯**: 30+ (骞跺彂杩炴帴 / zombie socket 鏉€鎺?/ SSE 娴佸紡)

#### Week 4: Chat 绠＄嚎 + Context 绠＄悊
- `crates/apeireth-pipeline/` (鏂?:
  - `chat_pipeline.rs` 鈥?涓?chat completion 绠＄嚎 (澶嶅埢 VCP 椤哄簭: parse 鈫?preprocess 鈫?variable expansion 鈫?tool inject 鈫?classify 鈫?dispatch 鈫?stream)
  - `final_context_store.rs` 鈥?VCP `finalContextStore.js` 澶嶅埢 + tiktoken 闆嗘垚 (鐢?`tiktoken-rs` crate)
  - `response_replay_cache.rs` 鈥?VCP `ResponseReplayCache` (LRU 100 + messageId 閿?
  - `multi_modal_config.rs` 鈥?VCP `multiModalConfigStore.js` (Force-Translate 鐧藉悕鍗?
- **鍊熼壌**: `chatCompletionHandler.js` + `finalContextStore.js` + `multiModalConfigStore.js`
- **娴嬭瘯**: 40+ (绠＄嚎绔埌绔?+ token 浼扮畻绮剧‘搴?+ 閲嶆斁)

**鎴樺焦 1 楠屾敹**:
- `cargo test --workspace` 鎬绘祴璇曟暟 鈮?1700
- `cargo build --release` 0 error
- 涓讳汉鎵嬫祴: `POST http://localhost:8080/v1/chat/completions` + Anthropic `POST http://localhost:8080/v1/messages` + Gemini 鍚岃姹備笉鍚屽崗璁?鈫?閮借兘娴佸紡杩斿洖
- 娴佸紡鍝嶅簲 P99 latency 鈮?200ms (棣栨 token)

### 鎴樺焦 2 路 宸ュ叿娉ㄥ唽 + 宸ュ叿璋冪敤 (Week 5-7) 鈥?鎴樺焦 2

**鐩爣**: 鐪熸敮鎸?VCP-style 鍔ㄦ€佸伐鍏锋敞鍐?+ 鐪熸墽琛屽伐鍏疯皟鐢?+ 宸ュ叿瀹℃壒 + 闅愮 mask銆?
**瀛愪换鍔?*:

#### Week 5: 鍔ㄦ€佸伐鍏锋敞鍐?- 鏂板缓 `crates/apeireth-tool-registry/`:
  - `plugin.rs` 鈥?Plugin trait (澶嶅埢 VCP plugin-manifest.json 瀛楁)
  - `registry.rs` 鈥?鍐呭瓨娉ㄥ唽涓績 + chokidar 鐑姞杞?(鐢?`notify` crate)
  - `classifier.rs` 鈥?7 绫诲垎绫?(search/file_code/image_media/memory_knowledge/agent_task/communication/data)
  - `token_budget.rs` 鈥?LIGHT_LIST/BRIEF/MAX_INJECTION 涓夊ぇ棰勭畻
  - `small_model_classifier.rs` 鈥?灏忔ā鍨嬪紓姝ュ垎绫?(debounce 1000ms + timeout 30s)
  - `manual_overrides.rs` 鈥?鐢ㄦ埛鎵嬪姩瑕嗙洊 (excluded/pinned/category aliases)
- **鍊熼壌**: `dynamicToolRegistry.js` (1608 琛?鏍稿績 7 瀛楁 + 7 绫诲叧閿瘝琛?+ token 棰勭畻 + 灏忔ā鍨?
- **娴嬭瘯**: 60+ (鍒嗙被鍑嗙‘鐜?+ 鐑姞杞?+ token 棰勭畻杈圭晫 + manual override)

#### Week 6: 宸ュ叿鎵ц + 宸ュ叿璋冪敤鍗忚
- 鏂板缓 `crates/apeireth-tool-runtime/`:
  - `tool_call_parser.rs` 鈥?瑙ｆ瀽 LLM 杈撳嚭 `<tool_call>...</tool_call>` (VCP `vcpLoop/toolCallParser.js`)
  - `tool_executor.rs` 鈥?鐪熸墽琛?(VCP `vcpLoop/toolExecutor.js`)
  - `tool_result.rs` 鈥?ToolResult 绫诲瀷 (success/output/error)
  - `error_detect.rs` 鈥?`isToolResultError` 澶氱骇鍒ゆ柇
  - `tool_call_record.rs` 鈥?宸ュ叿璋冪敤鍏ㄨ褰?(VCP `toolCallRecordStore.js` 19KB)
  - `privacy_guard.rs` 鈥?闅愮瀛楁 mask (VCP `toolResultPrivacyGuard.js` 7.5KB)
- **鍊熼壌**: `vcpLoop/toolCallParser.js` + `vcpLoop/toolExecutor.js` + `toolCallRecordStore.js`
- **娴嬭瘯**: 50+ (瑙ｆ瀽鍣ㄩ瞾妫掓€?+ 閿欒妫€娴嬪噯纭巼 + 闅愮 mask 姝ｇ‘鎬?

#### Week 7: 宸ュ叿瀹℃壒 + Agent 绯荤粺
- 鏂板缓 `crates/apeireth-tool-approval/`:
  - `approval_config.rs` 鈥?JSON 閰嶇疆 (enabled/timeoutMinutes/approveAll/approvalList/fuzzyMatching/privacyProtection)
  - `rule_parser.rs` 鈥?瑙勫垯璇硶 (ToolName / ToolName:command / ToolName::SilentReject)
  - `fuzzy_matcher.rs` 鈥?宸ュ叿鍚嶆ā绯婂尮閰?(LLM 鎷煎啓閿欒瀹瑰繊)
  - `chokidar_watcher.rs` 鈥?config 鐑姞杞?- 鏂板缓 `crates/apeireth-agent/`:
  - `agent_manager.rs` 鈥?alias 鈫?file 鏄犲皠 + 缂撳瓨 + 鐑姞杞?(VCP `agentManager.js` 339 琛?
  - `agent_map.rs` 鈥?agent_map.json
  - `agent_persona.rs` 鈥?闆嗘垚 `apeireth-council/persona.rs`
- **鍊熼壌**: `toolApprovalManager.js` + `agentManager.js` + `associativeDiscovery.js`
- **娴嬭瘯**: 50+ (瑙勫垯鍖归厤浼樺厛绾?+ 鐑姞杞?+ 寰幆寮曠敤妫€娴?+ agent prompt 缂撳瓨)

**鎴樺焦 2 楠屾敹**:
- 鎬绘祴璇曟暟 鈮?1860
- 涓讳汉鎵嬫祴: 鍦?`apeireth_config/approval.json` 鍔犺鍒?`FileOperator::SilentReject` 鈫?鐪熷彂璧?file write 璇锋眰 鈫?宸ュ叿闈欓粯鎷掔粷,涓嶉€氱煡 AI
- 涓讳汉鎵嬫祴: 鍔犱竴涓?plugin 鏂囦欢 鈫?chokidar 妫€娴?鈫?registry 鑷姩 reload 鈫?涓嬫 chat 娉ㄥ叆鏂?plugin 鎻忚堪

### 鎴樺焦 3 路 Admin Web UI + Desktop App (Week 8-9) 鈥?鎴樺焦 3

**鐩爣**: 鐪熷疄鍙敤鐨?web admin + desktop app,璁╀富浜?5 鍒嗛挓涓婃墜銆?
**瀛愪换鍔?*:

#### Week 8: Admin Web UI (Vue 閲嶅埗鐗?
- 鏀归€?`crates/apeireth-web/`:
  - **涓嶈 Vue** 鈥斺€?鐢?**Dioxus** (Rust 鍏ㄦ爤 web framework,璺?apeireth 涓€鑴夌浉鎵?SSR + WebAssembly)
  - 鎴栬€呯敤 **Axum + Leptos** (鏇磋交閲?Server-side render + client hydration)
  - 璺敱:
    - `/` 鈥?Dashboard (绯荤粺鐘舵€?+ 褰撳墠浼氳瘽)
    - `/chat` 鈥?鑱婂ぉ鐣岄潰 (鍗曢〉,瀹炴椂娴佸紡)
    - `/tools` 鈥?宸ュ叿娉ㄥ唽涓績 (鍒楁墍鏈?plugin + 鍚敤/绂佺敤 + 瀹℃壒瑙勫垯)
    - `/agents` 鈥?Agent 绠＄悊 (alias 鈫?file 鏄犲皠 + 缂栬緫 prompt)
    - `/logs` 鈥?宸ュ叿璋冪敤鏃ュ織 (鍙悳绱?杩囨护/閲嶆斁)
    - `/settings` 鈥?閰嶇疆 (LLM provider / 瀹℃壒瑙勫垯 / 闅愮 mask / 澶氭ā鎬佺櫧鍚嶅崟)
- 鍊熼壌 VCP `AdminPanel-Vue/` 鐨?Vue 婧愮爜,缈昏瘧鎴?Dioxus/Leptos
- **娴嬭瘯**: 30+ (缁勪欢娓叉煋 + API 璋冪敤 + 娴佸紡鏄剧ず)

#### Week 9: Desktop App (Tauri 2)
- 鏀归€?`crates/apeireth-desktop/`:
  - **Tauri 2** (Rust + WebView,璺?web UI 鍏辩敤鍓嶇浠ｇ爜)
  - 鍚姩鍚庡祵鍏?admin web + 鍐呯疆 chat 绐楀彛
  - 绯荤粺鎵樼洏 + 鍏ㄥ眬蹇嵎閿?+ 鏂囦欢鎷栨嫿
  - 鍊熼壌 VCP `vcp-installer-source/` 瀹夎鍣ㄦ€濊矾 (涓€閿畨瑁?
- **娴嬭瘯**: 20+ (Tauri 鍛戒护 + 绯荤粺闆嗘垚)

**鎴樺焦 3 楠屾敹**:
- 涓讳汉娴忚鍣ㄦ墦寮€ `http://localhost:8080/admin` 鈫?鐪嬪埌 dashboard
- 涓讳汉 chat 杈撳叆"浠婂ぉ澶╂皵鎬庝箞鏍? 鈫?鐪嬪埌娴佸紡鍝嶅簲 + 宸ュ叿璋冪敤 (鏌?weather)
- 涓讳汉鎵撳紑 settings 鈫?鏀?LLM provider 鈫?绔嬪嵆鐢熸晥
- 涓讳汉涓嬭浇 desktop app 鈫?鍙屽嚮杩愯 鈫?寮圭獥鐣岄潰 鈫?涓嶉渶瑕佸紑娴忚鍣?
### 鎴樺焦 4 路 閮ㄧ讲 + 鏂囨。 + 1.0 release (Week 10-11) 鈥?鎴樺焦 4

**鐩爣**: 浜岃繘鍒?+ 瀹夎鍖?+ 鐢ㄦ埛鏂囨。 + 1.0 tag銆?
**瀛愪换鍔?*:

#### Week 10: 閮ㄧ讲 + 浜岃繘鍒?- 鏂板缓 `crates/apeireth-installer/`:
  - `cargo install --path . --bin apeireth`
  - Windows MSI / macOS DMG / Linux deb + AppImage (鐢?`cargo-bundle` 鎴?`tauri-bundler`)
  - `apeireth start` / `stop` / `restart` / `status` / `update` 5 涓瓙鍛戒护
  - `apeireth config` 瀛愬懡浠?(鐪?鏀?config)
- 鍊熼壌 `deploy/` + `涓€閿惎鍔ㄦ湇鍔″櫒start_server.bat`
- **娴嬭瘯**: 15+ (鍚姩鑴氭湰骞傜瓑鎬?+ 鍗囩骇璺緞)

#### Week 11: 鐢ㄦ埛鏂囨。 + 1.0 release
- 鏂板缓 `docs/user/`:
  - `getting-started.md` 鈥?5 鍒嗛挓涓婃墜
  - `chat.md` 鈥?鑱婂ぉ鍔熻兘
  - `tools.md` 鈥?宸ュ叿浣跨敤
  - `agents.md` 鈥?Agent 鑷畾涔?  - `admin.md` 鈥?绠＄悊闈㈡澘
  - `troubleshooting.md` 鈥?鏁呴殰鎺掓煡
  - `architecture.md` 鈥?鏋舵瀯鎬昏 (缁欏紑鍙戣€?
- `CHANGELOG-1.0.0.md` 鈥?1.0 鍙戝竷璇存槑
- 瑙嗛鏁欑▼ (鍙€? 鐢?TTS + 褰曞睆)
- **鍊熼壌**: VCP `README.md` (17KB) + `README_en.md` (20KB) + `README_ja.md` (25KB) + `README_ru.md` (35KB) 涓夊浗璇█椋庢牸
- 涓讳汉鍐冲畾 1.0 tag 鏃堕棿鐐?
**鎴樺焦 4 楠屾敹**:
- 涓讳汉 fresh install (鍏ㄦ柊 Windows 铏氭嫙鏈? 鈫?5 鍒嗛挓鍐呰兘鐢?chat
- 涓讳汉璺?`apeireth start` 鈫?鍚庡彴 daemon + web UI 鑷姩寮€
- 涓讳汉鍗囩骇 1.0 鈫?1.1 鈫?閰嶇疆涓嶄涪,鏁版嵁杩佺Щ

---

## 绗?4 閮ㄥ垎 路 鍊熼壌 VCP 鐪熶笢瑗跨殑 7 鏉″叿浣撹鍒?
涓嶅啀閲嶅"鍊熼壌 NewAPI/VCP"鐨勬娊璞¤瘽銆傜粰鍏蜂綋瑙勫垯:

### 瑙勫垯 1 路 鐪熻婧愮爜,涓嶉潬鐚?
VCP 鐪熸簮鐮佸湪 `research/source/vcptoolbox/`,**浠讳綍"鍊熼壌 VCP" 绫诲伐浣滃繀椤诲厛 `read` 鐪熸枃浠?鎶撶粨鏋?+ 瀛楁 + 鍗忚,瀵圭収瀹為檯瀹炵幇,鏍囨敞 "0 鍊熼壌" / "閮ㄥ垎鍊熼壌" / "瀹屽叏鍊熼壌"**銆?
鍘嗗彶鏁欒 (2026-08-03 19:55 涓讳汉鐐归啋):
- 鉂?`apeireth-llm` (Round 16-01) 鎸?OpenAI Bearer token 鍐?NewAPI 瀹為檯鐢?`New-Api-User` header 鈫?瀹屽叏涓嶅
- 鉁?Round 16-02 鏀瑰悗鐪熻 `routes/admin/newapiMonitor.js` 鐪熷疄浠ｇ爜 鈫?淇

### 瑙勫垯 2 路 涓夊眰鍚堝苟閰嶇疆

澶嶅埢 VCP `DEFAULT_CONFIG 鈫?fileConfig 鈫?overrideConfig` 娣卞悎骞?
```rust
// crates/apeireth-config/src/lib.rs
pub fn merge_config(
    default: &Config,
    file: Option<&Config>,
    override_: Option<&Config>,
) -> Config { ... }
```
姣忎釜 tool/agent/plugin 閮芥湁鑷繁 5 灞傞厤缃? 纭紪鐮侀粯璁?鈫?env 鈫?TOML file 鈫?hot reload 鈫?API override銆?
### 瑙勫垯 3 路 token 棰勭畻涓夊眰

澶嶅埢 VCP token 棰勭畻:
- `LIGHT_LIST_TOKEN_BUDGET = 15` (杞诲垪琛?棣栨娉ㄥ叆)
- `DEFAULT_BRIEF_TOKEN_BUDGET = 6` (鎻忚堪,浜屾娉ㄥ叆)
- `MAX_INJECTION_CHARS = 16000` (鍗曟娉ㄥ叆 鈮?16KB)

闆嗘垚鍒?`apeireth-tool-registry`,姣忔 chat 鑷姩鎸夐绠楁埅鏂€?
### 瑙勫垯 4 路 鐏甸瓊绾у畧鍗?
澶嶅埢 VCP One Agent 鐏甸瓊:
- `context.expanded_agent_name: Option<String>` 涓€娆′細璇濆彧鑳藉睍寮€涓€涓?Agent
- 鍚庣画 Agent 鍗犱綅绗?*鍏ㄩ儴闈欓粯绉婚櫎** (涓嶆姏閿?涓嶈鍛?
- 鍊熼壌浣嶇疆: `apeireth-council/persona.rs` 鍔?`Session.expanded_agent_name` 瀛楁

### 瑙勫垯 5 路 Keep-Alive LIFO 姹?
澶嶅埢 VCP `http.Agent({ keepAlive: true, freeSocketTimeout: 8000, scheduling: 'lifo', maxSockets: 10000 })`:
- 鐢?Rust `reqwest` 鐨?`Connector::custom` + 鑷缓 `lifo_pool.rs`
- 8 绉掔┖闂?socket 涓诲姩閿€姣?(闃?zombie)
- LIFO 璋冨害 (浼樺厛澶嶇敤鏈€鏂伴矞鐨勮繛鎺?
- 闆嗘垚鍒?`apeireth-http-client`

### 瑙勫垯 6 路 鍗忚褰掍竴鍖?+ 鍏冩暟鎹€忎紶

澶嶅埢 VCP `protocolBridge.js` 鐨?`normalizeTextContent` / `normalizeMessageRole` / `copyArrayMetadata`:
- 4 鍗忚 (OpenAI Chat / OpenAI Responses / Anthropic / Gemini) 閮藉綊涓€鍖栧埌鍐呴儴 messages
- `__oneRingMeta` 绛夊厓鏁版嵁閫忎紶 (浠讳綍杩斿洖鏂版暟缁勭殑姝ラ蹇呴』鏄惧紡淇濈暀)
- 闆嗘垚鍒?`apeireth-protocol`

### 瑙勫垯 7 路 chokidar 鐑姞杞?= Rust `notify`

VCP 鐢?chokidar (Node.js),Rust 鐢?`notify` crate (5.x API):
```rust
use notify::{Watcher, RecursiveMode};
let mut watcher = notify::recommended_watcher(|res| { ... });
watcher.watch(Path::new("apeireth_config/"), RecursiveMode::Recursive)?;
```
姣忎釜 config / agent / plugin 鏂囦欢澶归兘鍔?watcher,鍙樺寲瑙﹀彂 reload銆?
---

## 绗?5 閮ㄥ垎 路 鍊熼壌鐨勫悓鏃?淇濈暀 Apeireth 鐪熸姢鍩庢渤

### 5.1 Council 7 advisor 闆嗘垚杩?chat 绠＄嚎

鎴樺焦 1 Week 4 鎶?`apeireth-council` 7 advisor 闆嗘垚杩?chat pipeline,浣滀负**鍝插瀹堥棬灞?*:

```text
chat_completion_pipeline:
  parse 鈫?preprocess 鈫?variable_expansion 鈫?tool_inject 鈫?classify
  鈫?  dispatch_to_llm
  鈫?  council_advise (7 advisor 鐪熻窇)  鈫?Apeireth 鐙湁
  鈫?  V1 (philosophy) AND V2 (sovereignty) AND V3 (default) 瀹堥棬
  鈫?  return_to_user
```

鍊熼壌浣嶇疆: `apeireth-pipeline/src/lib.rs` 鍔?`CouncilGuard` stage銆?
### 5.2 鎸変綇鏈哄埗闆嗘垚杩涘伐鍏峰鎵?
`apeireth-council/hold.rs` 鐨?30%/60s/3 杞?鏈哄埗闆嗘垚鍒?`apeireth-tool-approval`:
- 宸ュ叿瀹℃壒瑙﹀彂 鈫?鍚姩 council hold (60s timeout)
- 30% advisor 寮哄弽瀵?鈫?鑷姩鎷掔粷宸ュ叿鎵ц
- 鍚﹀垯璁?advisor 杈╄ 3 杞?鈫?澶氭暟鍐?
鍊熼壌浣嶇疆: `apeireth-tool-approval/src/hold_integration.rs`銆?
### 5.3 PyO3 鍏煎妗ラ泦鎴愯繘宸ュ叿娉ㄥ唽

`apeireth-pybridge` 鏆撮湶缁?`apeireth-tool-registry`:
- 鐢ㄦ埛鍙互鐢?Python 鍐?plugin (LangChain/LlamaIndex 鐢熸€?
- Rust 鑷姩鍙戠幇 `apeireth_plugins/*.py` + `plugin.toml`
- 娉ㄥ唽涓?first-class plugin

鍊熼壌浣嶇疆: `apeireth-tool-registry/src/python_loader.rs` (澶嶅埢 VCP `plugin-manifest.json` 鐨?Python 鍔犺浇)銆?
---

## 绗?6 閮ㄥ垎 路 椋庨櫓 + 涓嶅仛鐨勪簨

### 6.1 椋庨櫓

| 椋庨櫓 | 搴斿 |
|---|---|
| 鎴樺焦 1-4 鏃堕棿涓嶅 (12 鍛ㄥお闀? | 鎴樺焦 0-2 蹇呴』鎸夋湡,鎴樺焦 3-4 鍙帇缂╁埌 4 鍛?(鐢ㄧ幇鎴?Tauri 妯℃澘) |
| 鍊熼壌 VCP 鏃惰繃搴﹁€﹀悎 VCP 姒傚康 | 鍙€熷伐绋嬬粡楠?涓嶆妱涓氬姟閫昏緫 (e.g. VCP `VCPModelAuto` 鍊熼壌"璇箟璺敱"姒傚康,浣嗕笉鎶?preset 鏂囦欢鍚? |
| 鍝插鎶借薄璺熷伐绋嬫娊璞″啿绐?(e.g. council 7 advisor vs VCP Agent 1 涓? | 鐢?wrapper 閫傞厤: council 鍏ㄥ憳璺戜竴閬?鈫?澶氭暟鍐冲綋 Agent 鐢?|
| Rust 缂栬瘧鏃堕棿闀?(5-10 鍒嗛挓) | 鐢?`cargo-chef` 缂撳瓨渚濊禆,CI 鐢?sccache,鏈湴鐢?mold linker |
| Web frontend 閫夊瀷 (Dioxus vs Leptos vs 绾?HTML+JS) | 鎴樺焦 3 Week 8 绗竴澶╁仛 spike (3 涓?demo 鍚?1 灏忔椂,鎸戜竴涓? |
| Desktop Tauri 鍦?Windows 涓?webview2 鍏煎 | 鎴樺焦 3 Week 9 绗竴澶╅獙璇?(Tauri 2 + WebView2 宸茬ǔ瀹? |

### 6.2 涓嶅仛鐨勪簨 (8 椤逛笉淇敼鎵胯缁х画)

鉂?**涓嶄慨鏀?R11 LOCKED**:
- `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` (6546 琛? R11 LOCKED)
- `Cargo.lock` (R11 LOCKED)
- V0.5 / V1136 / V3 9 閿?/ 1100 妯″潡

鉂?**涓嶄慨鏀?apeireth-legacy/** (Fix-14 鍔?DEPRECATED-MARKER, 鍙涓嶅垹)

鉂?**涓嶅垱寤烘柊鍝插鎶借薄 crate** (consciousness/perception/onion/life-force/motivation 宸叉湁,涓嶉噸澶?

鉂?**涓嶅紩鍏?unsafe Rust** (workspace 宸叉湁 `#![deny(unsafe_code)]`)

鉂?**涓嶅紩鍏?I/O / 缃戠粶 / 鏂囦欢绯荤粺鍒?core** (core 绾被鍨?

鉂?**涓嶇牬鍧忕幇鏈?1641 涓祴璇?* (姣忔 commit 蹇呴』 鈮?1641 閫氳繃)

鉂?**涓嶅啓"鍊熼壌 VCP" 鎶借薄璇?* (浠讳綍鍊熼壌绫诲伐浣滃繀椤诲厛 read 鐪熸枃浠?+ 鏍囨敞鍊熼壌绋嬪害)

鉂?**涓嶆妱琚?VCP 涓氬姟閫昏緫** (鍙€熼壌宸ョ▼缁忛獙 + 鍗忚 + 瀛楁缁撴瀯)

### 6.3 涓诲摬瀛?anchor (6 涓叏璐┛)

鎴樺焦 1-4 鍏ㄩ儴鎸?6 涓富鍝插瀹堥棬:
1. **涓嶅亣瑁?* 鈥?鐪熸帴閫?LLM,鐪熻窇宸ュ叿,鐪熺粰鍙嶉
2. **涓嶆楠?* 鈥?鍊熼壌 VCP 蹇呴』鏍囨敞鍊熼壌绋嬪害 (0/閮ㄥ垎/瀹屽叏)
3. **涓嶅伔鎳?* 鈥?鍗忚灞?/ 瀹℃壒 / 闅愮 / 鏃ュ織 / 鐩戞帶鍏ㄥ仛
4. **涓嶉噸澶嶉€犺疆瀛?* 鈥?鑳藉€?VCP 鐪熶笢瑗垮氨鍊?涓嶉噸鏂扮紪
5. **涓嶈劚绂荤兢浼?* 鈥?娑堣垂绾?= 鏅€氱敤鎴疯兘璺?涓嶆槸宸ョ▼甯?demo
6. **涓嶅晢涓氱粦鏋?* 鈥?姘歌繙淇濈暀 Rust 寮€婧?+ self-host 閫夐」,涓嶉攣姝?SaaS

---

## 绗?7 閮ㄥ垎 路 涓讳汉 12:15 鍐崇瓥椤?(寰呯‘璁?

涓讳汉鍦?12:15 鎻愪簡 2 浠朵簨:
1. 鉁?**Apeireth 娑堣垂绾у寲** 鈥?鏈枃妗ｅ氨鏄瓟妗?2. 鈴?**Codex 閰嶇疆 (鐢?MiniMax API, 榛樿 1M-MinimaxM3)** 鈥?瑙?`04-CODEX-CONFIG-2026-08-04.md` 鎴栬涓嬮潰闄勫綍

### 7.1 Codex 閰嶇疆 (鍚屾椂宸茶惤鍦?

涓讳汉璇?"codex 閰嶇疆濂?灏辩敤 minimax 鐨?api,榛樿妯″瀷涓?1M-MinimaxM3"銆傚凡鍋氱殑浜?
- 鏀?`.codex\config.toml`:
  - 鍔?`[model_providers.minimax]` provider,`base_url = "http://localhost:3000/v1"`,`env_key = "OPENAI_API_KEY"`
  - `model_provider = "minimax"`
  - `model = "1M-MinimaxM3"`
- 楠岃瘉: `codex doctor` 搴旀彁绀?provider 鍔犺浇鎴愬姛

**娉?*: Codex v0.146.0 宸茬粡绉婚櫎 `wire_api = "chat"`,**鍙敮鎸?Responses API**銆俙/v1/responses` 鍦?NewAPI 涓婃槸鍚︾湡鏀寔瑕佺湅 NewAPI 鐗堟湰銆傚鏋滀富浜鸿窇璧锋潵鎶?endpoint not found", 澶囬€夋柟妗?
- 鏂规 A: 鍗囩骇 NewAPI 鍒版渶鏂扮増 (鏀寔 `/v1/responses`)
- 鏂规 B: 鐢?NewAPI 鑷畾涔夋笭閬撻厤缃?鎶婁笂娓?LLM 鍖呰鎴?Responses 鍏煎
- 鏂规 C: 涓嶇敤 NewAPI,鐩存帴閰嶇疆 codex 璧?MiniMax 瀹樻柟 `https://api.minimaxi.com/v1`

璇﹁涓嬫枃闄勫綍銆?
---

## 闄勫綍 A 路 VCP 鐪熶唬鐮侀澶栨娊鍑虹殑 12 鏉￠粍閲戠粏鑺?
浠庣湡浠ｇ爜閲屾娊鍑虹殑缁嗚妭,涓嶆槸鐚滅殑:

1. **VCP 鏈?5 绫?keep-alive 閰嶇疆** (`keepAlive: true`, `keepAliveMsecs: 1000`, `freeSocketTimeout: 8000`, `scheduling: 'lifo'`, `maxSockets: 10000`) 鈥?澶嶅埢 5 鏉″埌 Rust reqwest銆?2. **VCP 鏈?7 绫诲伐鍏峰垎绫?* (search/file_code/image_media/memory_knowledge/agent_task/communication/data),**姣忎釜鏈変腑鑻辨枃鍏抽敭璇嶈〃**銆?3. **VCP 5 鍒嗛挓瀹℃壒 timeout** + `fuzzyToolMatching` 闃?LLM 鎷奸敊宸ュ叿鍚嶃€?4. **VCP One Agent 鐏甸瓊** 鈥?`context.expandedAgentName !== undefined` 鏃跺悗缁?Agent 鍗犱綅绗?*闈欓粯绉婚櫎** (涓嶆姤閿?銆?5. **VCP 3 灞?metadata 閫忎紶** (`copyArrayMetadata` 鐢?`Object.defineProperty` 澶嶅埗闈炴灇涓惧睘鎬? 鈥?Rust 鐢?`serde_json::Value` 鐨?`as_object_mut().extend()`銆?6. **VCP Force-Translate** 鈥?`multiModalConfigStore.js` 妫€娴?base64 image 鈫?鑷姩杞枃鏈?閬垮厤 deepseek/GLM 400 閿欒) 鈥?鎴樺焦 1 Week 4 蹇呴』鏈夈€?7. **VCP `MAX_LOG_PAGES = 200`** 鈥?鏃ュ織鍒嗛〉纭笂闄?闃?admin 鎷夌垎鍐呭瓨銆?8. **VCP 5 缁?final context snapshot** + `cl100k_base` tiktoken 鈥?鎴樺焦 1 Week 4 蹇呴』鏈夈€?9. **VCP `extractProtectedTools`** 鈥?Gemini `functionDeclarations` + legacy `functions` 閮芥彁鍙?**鍙墠鍚戜紶閫?涓嶈繘 messages/RAG**銆?10. **VCP 15s 鎶戝埗绐楀彛** (`PROTOCOL_BRIDGE_RETRY_SUPPRESSION_MS: 15000`) 鈥?闃?OpenAI Responses 鍋跺彂 5xx 閲嶈瘯椋庢毚銆?11. **VCP `VCPModelAuto` + `matchThreshold: 0.18`** 鈥?浣欏鸡鐩镐技搴︿綆浜?0.18 璧?fallback銆?12. **VCP `request_max_retries: 4` + `stream_max_retries: 5` + `stream_idle_timeout_ms: 300000`** 鈥?榛樿閲嶈瘯 + 娴佽秴鏃躲€?
---

## 闄勫綍 B 路 30 涓洰鏍?crate 鐨勭幇鐘剁洏鐐?
鎸?`_STRUCTURE.md` "鐩爣 30 crate" 鍒楄〃 (R11 闃舵 2 搂3 璁捐):

| # | Crate | 褰撳墠鐘舵€?| 鎴樺焦褰掑 |
|---|---|---|---|
| 1 | `apeireth-action` | skeleton | 鎴樺焦 2 (tool_executor) |
| 2 | `apeireth-api` | 鉁?R16-02 瀹屾垚 LLM client | 鎴樺焦 1 鎵╁埌鍗忚灞?|
| 3 | `apeireth-asi` | 鉁?LOCKED V0.5/V1136 | 淇濈暀,闆嗘垚杩?chat pipeline |
| 4 | `apeireth-bench` | 鉁?R14 瀹屾垚 | 淇濈暀 |
| 5 | `apeireth-bus` | skeleton | 鎴樺焦 1 (event bus) |
| 6 | `apeireth-central` | skeleton | 鎴樺焦 3 (main daemon) |
| 7 | `apeireth-cli` | 鉁?R16 瀹屾垚 26KB | 鎴樺焦 4 鍔?install 瀛愬懡浠?|
| 8 | `apeireth-cognition` | skeleton | (璺宠繃,鍝插鎶借薄澶熺敤) |
| 9 | `apeireth-consciousness` | skeleton | (璺宠繃) |
| 10 | `apeireth-constraint` | skeleton | (璺宠繃) |
| 11 | `apeireth-core` | 鉁?R11 瀹屾垚 | 淇濈暀 |
| 12 | `apeireth-council` | 鉁?7 advisor + hold + synthesis | 鎴樺焦 1 闆嗘垚杩?chat pipeline |
| 13 | `apeireth-desktop` | skeleton | 鎴樺焦 3 (Tauri 2) |
| 14 | `apeireth-evolution` | skeleton | (璺宠繃) |
| 15 | `apeireth-extension` | skeleton | 鎴樺焦 2 (plugin trait) |
| 16 | `apeireth-life-force` | skeleton | (璺宠繃) |
| 17 | `apeireth-memory` | skeleton | 鎴樺焦 2 (memory_knowledge 绫诲伐鍏? |
| 18 | `apeireth-motivation` | skeleton | (璺宠繃) |
| 19 | `apeireth-onion` | skeleton | (璺宠繃) |
| 20 | `apeireth-perception` | skeleton | (璺宠繃) |
| 21 | `apeireth-philosophy` | 鉁?LOCKED V3 9 閿?| 淇濈暀,闆嗘垚杩?V1 瀹堥棬 |
| 22 | `apeireth-pybridge` | 鉁?R14 LOCKED 1100 妯″潡 | 鎴樺焦 2 (Python plugin loader) |
| 23 | `apeireth-relation` | skeleton | (璺宠繃) |
| 24 | `apeireth-sovereignty` | 鉁?Hook trait | 淇濈暀,闆嗘垚杩?V2 瀹堥棬 |
| 25 | `apeireth-supervisor` | skeleton | 鎴樺焦 4 (process supervisor) |
| 26 | `apeireth-test` | 鉁?R14 瀹屾垚 | 淇濈暀 |
| 27 | `apeireth-tools` | skeleton 800B | 鎴樺焦 2 鏁翠綋閲嶅啓 |
| 28 | `apeireth-tui` | skeleton | 鎴樺焦 4 (鍙€? |
| 29 | `apeireth-upgrade` | skeleton | 鎴樺焦 4 (鐗堟湰杩佺Щ) |
| 30 | `apeireth-value` | skeleton | (璺宠繃) |
| 31 | `apeireth-verify` | skeleton | 鎴樺焦 4 (verify 宸ュ叿) |
| 32 | `apeireth-web` | skeleton | 鎴樺焦 3 (admin web) |

**鎴樺焦 1-4 瀹為檯鏂板缓/鏀归€?crate 鏁?*:
- 鎴樺焦 1: 鏂板缓 4 (apeireth-protocol, apeireth-http-client, apeireth-pipeline, 鏀归€?apeireth-api)
- 鎴樺焦 2: 鏂板缓 5 (apeireth-tool-registry, apeireth-tool-runtime, apeireth-tool-approval, apeireth-agent, 鏀归€?apeireth-tools)
- 鎴樺焦 3: 鏀归€?2 (apeireth-web, apeireth-desktop)
- 鎴樺焦 4: 鏂板缓 2 (apeireth-installer, apeireth-upgrade), 鏀归€?1 (apeireth-cli)

**鎬?*: 鏂板缓 11 涓?crate + 鏀归€?5 涓?crate = 16 涓敼鍔?**31 涓?crate 缁存寔鐜扮姸**銆?
---

## 闄勫綍 C 路 鍊熼壌 VCP 鐪熶笢瑗跨殑 18 涓叿浣撴枃浠舵竻鍗?
鎴樺焦 1-4 鏈熼棿,浠ヤ笅 VCP 鐪熸枃浠跺繀椤昏 + 鏍囨敞鍊熼壌:

| VCP 鐪熸枃浠?| 鎴樺焦 | 鍊熼壌绋嬪害 | 鍊熼壌鏂瑰紡 |
|---|---|---|---|
| `modules/dynamicToolRegistry.js` (74KB) | 鎴樺焦 2 | 瀹屽叏鍊熼壌 | `apeireth-tool-registry` 涓讳綋 |
| `modules/chatCompletionHandler.js` (59KB) | 鎴樺焦 1 | 瀹屽叏鍊熼壌 | `apeireth-pipeline` 涓?chat 绠＄嚎 |
| `modules/messageProcessor.js` (44KB) | 鎴樺焦 1 | 閮ㄥ垎鍊熼壌 | 鍗犱綅绗?+ 闃插惊鐜?(涓嶈 agent 鐏甸瓊,鐣欑粰 council) |
| `routes/protocolBridge.js` (39KB) | 鎴樺焦 1 | 瀹屽叏鍊熼壌 | `apeireth-protocol` 鍗忚褰掍竴鍖?|
| `modules/agentManager.js` (16KB) | 鎴樺焦 2 | 閮ㄥ垎鍊熼壌 | 鍊熼壌 alias + 鐑姞杞? persona 鐢?apeireth-council |
| `modules/roleDivider.js` (16KB) | 鎴樺焦 1 | 瀹屽叏鍊熼壌 | `apeireth-protocol` 瑙掕壊鍒嗗壊 |
| `modules/semanticModelRouter.js` (17KB) | 鎴樺焦 1 | 閮ㄥ垎鍊熼壌 | 鍊熼壌浣欏鸡璺敱, 鍝插璺敱鐢?apeireth-asi |
| `modules/finalContextStore.js` (11KB) | 鎴樺焦 1 | 瀹屽叏鍊熼壌 | `apeireth-pipeline/final_context.rs` |
| `modules/toolApprovalManager.js` (8.5KB) | 鎴樺焦 2 | 瀹屽叏鍊熼壌 | `apeireth-tool-approval` 涓讳綋 |
| `modules/toolResultPrivacyGuard.js` (7.5KB) | 鎴樺焦 2 | 瀹屽叏鍊熼壌 | `apeireth-tool-runtime/privacy_guard.rs` |
| `modules/toolCallRecordStore.js` (19KB) | 鎴樺焦 2 | 瀹屽叏鍊熼壌 | `apeireth-tool-runtime/record.rs` |
| `modules/vcpLogReplayManager.js` (19KB) | 鎴樺焦 1 | 閮ㄥ垎鍊熼壌 | 閲嶆斁鏈哄埗鍊熼壌 |
| `modules/multiModalConfigStore.js` (10KB) | 鎴樺焦 1 | 瀹屽叏鍊熼壌 | `apeireth-pipeline/multi_modal.rs` |
| `modules/foldProtocol.js` (2.3KB) | 鎴樺焦 1 | 瀹屽叏鍊熼壌 | `apeireth-pipeline/fold.rs` |
| `modules/associativeDiscovery.js` (8KB) | 鎴樺焦 2 | 閮ㄥ垎鍊熼壌 | 鑱旀兂鍙戠幇鍊熼壌 |
| `modules/captchaDecoder.js` (2KB) | 鎴樺焦 3 | 瀹屽叏鍊熼壌 | admin web 鍔犻獙璇佺爜 |
| `modules/browserRuntimeManager.js` (26KB) | 鎴樺焦 2 | 瀹屽叏鍊熼壌 | `apeireth-tool-browser` (ChromeBridge) |
| `Plugin/FileOperator/` (68KB) | 鎴樺焦 2 | 瀹屽叏鍊熼壌 | `apeireth-tool-filesystem` |
| `Plugin/PowerShellExecutor/` + `LinuxShellExecutor/` | 鎴樺焦 2 | 瀹屽叏鍊熼壌 | `apeireth-tool-shell` |

**鎬昏**: 19 涓?VCP 鐪熸枃浠跺繀椤绘繁搴﹁ + 鍊熼壌,**涓嶅厑璁歌烦杩?*銆?
---

## 闄勫綍 D 路 闃舵 1 鈫?闃舵 2 鈫?闃舵 3 瀹屾暣杩囨浮

鎸変富浜?12:15 鍐崇瓥,Apeireth 杩涘叆**闃舵 3: 娑堣垂绾у寲**銆?
| 闃舵 | 鍛ㄦ湡 | 鐩爣 | 褰撳墠 |
|---|---|---|---|
| 闃舵 1 | R11-R13 | 鍝插鍘熷瀷 + 9 crate skeleton | 鉁?瀹屾垚 (LOCKED) |
| 闃舵 2 | R14-R16 | 宸ョ▼鍖?+ 28 crate 瀹炶 + 1641 娴嬭瘯 | 鉁?瀹屾垚 (HEAD 08c25c26) |
| **闃舵 3** | **R17+ (鏈懆鍚姩)** | **娑堣垂绾у寲 + 4 鎴樺焦 + 1.0 release** | **馃殌 鏈枃妗ｅ惎鍔?* |

闃舵 3 鏍囧織:
- 鉁?R17 鐮嶆帀 NewAPI 杩涚▼渚濊禆,R17 鐮嶆帀 `admin.rs`/`http.rs`/`gateway/` (涓讳汉 2026-08-03 鍐崇瓥) 鈥?宸插畬鎴?(Round 16-02)
- 鈴?R18 鎴樺焦 1 (鍗忚灞?+ chat 绠＄嚎) 鈥?寰呭惎鍔?- 鈴?R19 鎴樺焦 2 (宸ュ叿娉ㄥ唽 + 宸ュ叿璋冪敤) 鈥?寰呭惎鍔?- 鈴?R20 鎴樺焦 3 (admin web + desktop) 鈥?寰呭惎鍔?- 鈴?R21 鎴樺焦 4 (閮ㄧ讲 + 鏂囨。 + 1.0) 鈥?寰呭惎鍔?
---

## _Last update_

_2026-08-04 12:15, by 妤氶浂. Apeireth 闃舵 3 娑堣垂绾у寲璺嚎鍥?鈥?4 鎴樺焦 / 12 鍛?/ 鍊熼壌 VCP 19 涓湡鏂囦欢 / 6 椤逛笉淇壙璇虹户缁畧浣?_
_Initial version: 2026-08-04 12:15_
