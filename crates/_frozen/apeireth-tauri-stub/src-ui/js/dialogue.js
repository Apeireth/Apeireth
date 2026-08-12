/* ===========================================================
 * 对话 (DIALOGUE) — 5 步 cycle 进度 + 用户消息 log
 * W2: 接 apeireth-api 后端 (POST /v1/chat/completions 流式 SSE)
 *
 * **架构**: 浏览器 → http://127.0.0.1:8080/v1/chat/completions → apeireth-api
 *          → 4 协议管线 → LLM provider
 * **降级**: 后端不可达 (daemon 未启 / CORS 失败) → 退回 W1 mock, status_bar 显示 ✗
 * **SSE 流式**: 真逐 chunk 推 DOM (跟 apeireth-tui http_llm::call_llm_http_stream 一致)
 * =========================================================== */

(function () {
    "use strict";

    let cycleStep = 0;

    // 后端默认 URL (跟 R27 TUI onboarding [0] server 模式一致)
    const API_BASE = window.__APEIRETH_API__ || "http://127.0.0.1:8080";
    const API_HEALTH = `${API_BASE}/health`;
    const API_CHAT = `${API_BASE}/v1/chat/completions`;

    // 模型名 (跟 TUI llm_config.json 对齐; 默认 minimaxi)
    const MODEL_NAME = window.__APEIRETH_MODEL__ || "MiniMax-M3";
    const SYSTEM_PROMPT = "你是 ΑΠΕΙΡΕΘ (Apeireth), 一个 AI 助手。请简洁、有思想地回答用户的问题。";

    let apiOnline = false;     // 上次探测结果
    let lastProbeAt = 0;       // 上次探测时间戳 (ms)
    const PROBE_INTERVAL_MS = 10000;  // 10s 内不重复探测

    function advanceCycle() {
        const el = document.getElementById("dialogueCycle");
        if (!el) return;
        el.querySelectorAll(".cycle-step").forEach((s) => {
            const step = parseInt(s.getAttribute("data-step"), 10);
            s.setAttribute("data-active", step === cycleStep + 1 ? "true" : "false");
        });
    }

    function appendMsg(role, content) {
        const log = document.getElementById("dialogueLog");
        if (!log) return;
        const el = document.createElement("div");
        el.className = `dialogue-msg dialogue-msg-${role}`;
        el.innerHTML = `
            <div class="dialogue-msg-role">${role === "user" ? "USER" : "主 AI"}</div>
            <div class="dialogue-msg-content">${content}</div>
        `;
        log.appendChild(el);
        log.scrollTop = log.scrollHeight;
        return el;
    }

    function appendStreaming(role) {
        const log = document.getElementById("dialogueLog");
        if (!log) return null;
        const el = document.createElement("div");
        el.className = `dialogue-msg dialogue-msg-${role}`;
        el.innerHTML = `
            <div class="dialogue-msg-role">${role === "user" ? "USER" : "主 AI"}</div>
            <div class="dialogue-msg-content"></div>
        `;
        log.appendChild(el);
        log.scrollTop = log.scrollHeight;
        return el.querySelector(".dialogue-msg-content");
    }

    // ----- 后端连通探测 (轻量, 5s 超时) -----
    async function probeApi() {
        const now = Date.now();
        if (now - lastProbeAt < PROBE_INTERVAL_MS) return apiOnline;
        lastProbeAt = now;
        try {
            const ctrl = new AbortController();
            const t = setTimeout(() => ctrl.abort(), 5000);
            const r = await fetch(API_HEALTH, { method: "GET", signal: ctrl.signal, mode: "cors" });
            clearTimeout(t);
            apiOnline = r.ok;
        } catch (_) {
            apiOnline = false;
        }
        updateBackendBadge();
        return apiOnline;
    }

    function updateBackendBadge() {
        const badge = document.getElementById("backendBadge");
        if (!badge) return;
        badge.setAttribute("data-online", apiOnline ? "true" : "false");
        badge.textContent = apiOnline
            ? `API ${API_BASE.replace(/^https?:\/\//, "")} ●`
            : `API offline ✗`;
        badge.title = apiOnline
            ? `apeireth-api 在线 @ ${API_BASE}`
            : `apeireth-api 离线 — 启 daemon: cargo run -p apeireth-api`;
    }

    // ----- W2: 真接后端流式 chat -----
    async function chatViaBackend(input, history) {
        const messages = [{ role: "system", content: SYSTEM_PROMPT }];
        for (const m of history) {
            if (m.role === "user" || m.role === "assistant") {
                messages.push({ role: m.role, content: m.content });
            }
        }
        messages.push({ role: "user", content: input });

        const body = {
            model: MODEL_NAME,
            messages,
            stream: true,
            temperature: 0.7,
            max_tokens: 4096,
        };

        const ctrl = new AbortController();
        const t = setTimeout(() => ctrl.abort(), 60000);

        const resp = await fetch(API_CHAT, {
            method: "POST",
            mode: "cors",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
            signal: ctrl.signal,
        });
        clearTimeout(t);

        if (!resp.ok) {
            const errBody = await resp.text().catch(() => "");
            throw new Error(`HTTP ${resp.status}: ${errBody.slice(0, 200)}`);
        }
        if (!resp.body) {
            // 非流式响应 (daemon 没启 stream)
            const j = await resp.json();
            const txt = (j.choices && j.choices[0] && j.choices[0].message && j.choices[0].message.content) || "(空响应)";
            return txt;
        }

        // SSE 流式: data: {json}\n\n 逐 chunk
        const reader = resp.body.getReader();
        const dec = new TextDecoder("utf-8");
        let buf = "";
        let fullText = "";
        while (true) {
            const { value, done } = await reader.read();
            if (done) break;
            buf += dec.decode(value, { stream: true });
            let nl;
            while ((nl = buf.indexOf("\n\n")) !== -1) {
                const evt = buf.slice(0, nl);
                buf = buf.slice(nl + 2);
                for (const line of evt.split("\n")) {
                    if (!line.startsWith("data:")) continue;
                    const data = line.slice(5).trim();
                    if (data === "[DONE]") continue;
                    try {
                        const j = JSON.parse(data);
                        const delta = j.choices && j.choices[0] && j.choices[0].delta && j.choices[0].delta.content;
                        if (delta) fullText += delta;
                    } catch (_) { /* skip malformed SSE line */ }
                }
            }
        }
        return fullText || "(流式空响应)";
    }

    async function send(input) {
        appendMsg("user", input);
        // 5 步 cycle 动画 (跟 W1 视觉一致)
        for (let i = 0; i < 5; i++) {
            cycleStep = i + 1;
            advanceCycle();
            await new Promise((r) => setTimeout(r, 120));
        }

        const online = await probeApi();
        const target = appendStreaming("ai");

        if (!online) {
            // 降级 W1 mock (跟 apeireth-tui 兜底路径一致)
            target.textContent = `(W1 mock, 后端未连) 收到: ${input}`;
            updateBackendBadge();
        } else {
            try {
                target.textContent = "";
                const reply = await chatViaBackend(input, collectHistory(20));
                target.textContent = reply || "(空回复)";
            } catch (e) {
                target.textContent = `(后端错误) ${e.message || e}`;
                apiOnline = false;
                updateBackendBadge();
            }
        }

        setTimeout(() => {
            cycleStep = 0;
            advanceCycle();
        }, 600);
    }

    function collectHistory(limit) {
        const log = document.getElementById("dialogueLog");
        if (!log) return [];
        const msgs = [];
        const nodes = log.querySelectorAll(".dialogue-msg");
        const slice = nodes.length > limit ? nodes.slice(nodes.length - limit) : nodes;
        for (const n of slice) {
            const role = n.classList.contains("dialogue-msg-user") ? "user" : "assistant";
            const txt = n.querySelector(".dialogue-msg-content");
            if (txt && txt.textContent) msgs.push({ role, content: txt.textContent });
        }
        return msgs;
    }

    function init() {
        const form = document.getElementById("dialogueForm");
        const input = document.getElementById("dialogueInput");
        if (!form || !input) return;
        form.addEventListener("submit", (e) => {
            e.preventDefault();
            const v = input.value.trim();
            if (!v) return;
            input.value = "";
            send(v);
        });
        // 启动时探测 + 注册周期探测 (跟 R27 TUI status_bar 一致)
        probeApi();
        setInterval(probeApi, PROBE_INTERVAL_MS);
    }

    window.ApeirethDialogue = { init, send, probeApi };
})();