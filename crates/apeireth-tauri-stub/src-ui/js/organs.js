/* ===========================================================
 * 9 器官呼吸星群渲染 (3×3 星座)
 *
 * W1 mock: 直接用前端 mock 数据
 * W2: 通过 window.__TAURI__.invoke('get_organ_status') 拉真值
 * =========================================================== */

(function () {
    "use strict";

    // W1 mock (跟 Rust 端 mock_organ_status 对齐, W2 替换)
    const ORGANS_MOCK = [
        { name: "perception",      display: "感知", metaphor: "五感",       health: 0.85, primary: "5/5",               secondary: "Text · Voice · Vision",   tertiary: "events/s: 12" },
        { name: "cognition",       display: "认知", metaphor: "大脑",       health: 0.92, primary: "0.8595",            secondary: "12 键: 全过",            tertiary: "cycle: 8.2k" },
        { name: "consciousness",   display: "意识", metaphor: "心智",       health: 0.95, primary: "Awake",            secondary: "Reflecting ↔ Dreaming", tertiary: "12 次转换" },
        { name: "memory",          display: "记忆", metaphor: "海马体",     health: 0.78, primary: "6 流 · 1247",      secondary: "Episode: 824",          tertiary: "did:apeireth:web-001" },
        { name: "motivation",      display: "动机", metaphor: "多巴胺",     health: 0.88, primary: "0.91",             secondary: "目标: assist-and-reflect", tertiary: "E 层: 4 证据" },
        { name: "value",           display: "价值", metaphor: "前额叶",     health: 0.90, primary: "5/5 一致",          secondary: "硬门槛: 100%",          tertiary: "E 层无冲突" },
        { name: "relation",        display: "关系", metaphor: "镜像神经元", health: 0.83, primary: "3 + 1 + 1 + 1",    secondary: "共生 · 协调 · 嵌入 · 与自身", tertiary: "Self: did:apeireth:web-001" },
        { name: "action",          display: "行动", metaphor: "肌肉",       health: 0.86, primary: "Execute 62%",      secondary: "Express 35%",           tertiary: "Silence 3%" },
        { name: "life_force",      display: "生命力", metaphor: "免疫",     health: 0.97, primary: "0.97",             secondary: "dormant",              tertiary: "did:apeireth:web-001" },
    ];

    function render() {
        const target = document.getElementById("organs");
        if (!target) return;

        // W1 注入 mock, W2 用 window.__TAURI__.invoke('get_organ_status')
        const organs = ORGANS_MOCK;
        target.innerHTML = organs.map((o) => `
            <div class="organ" data-organ="${o.name}">
                <div class="organ-breath" aria-hidden="true"></div>
                <div class="organ-header">
                    <span class="organ-zh">${o.display}</span>
                    <span class="organ-metaphor">${o.metaphor}</span>
                </div>
                <div class="organ-primary">${o.primary}</div>
                <div class="organ-secondary">${o.secondary}</div>
                <div class="organ-tertiary">${o.tertiary}</div>
                <div class="organ-bar">
                    <div class="organ-bar-fill" style="--health: ${o.health}"></div>
                </div>
            </div>
        `).join("");

        // 下一帧触发 transition
        requestAnimationFrame(() => {
            target.querySelectorAll(".organ-bar-fill").forEach((el) => {
                el.setAttribute("data-loaded", "true");
            });
        });
    }

    window.ApeirethOrgans = { render };
})();
