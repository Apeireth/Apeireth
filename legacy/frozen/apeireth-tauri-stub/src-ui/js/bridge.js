/* ===========================================================
 * 舰桥 (BRIDGE) — 启屏默认页
 *   - ASI V0.5 大数字 + 弧线 + 12.94% gap
 *   - 30 crate 星图 (SVG)
 *   - 9 器官呼吸点 (压缩)
 * =========================================================== */

(function () {
    "use strict";

    // W1 mock (跟 Rust 端 mock_topology 对齐)
    const TOPOLOGY_MOCK = [
        // 总 (中心)
        { name: "apeireth-supervisor", display: "总监督", group: "总", x: 400, y: 300, active: 0.95, pid: 1, strategy: "permanent" },
        // 主核
        { name: "apeireth-core",       display: "核心",   group: "主核", x: 200, y: 200, active: 0.92, pid: 101, strategy: "rest_for_one" },
        { name: "apeireth-onion",      display: "洋葱",   group: "主核", x: 600, y: 200, active: 0.88, pid: 104, strategy: "rest_for_one" },
        { name: "apeireth-constraint", display: "约束",   group: "主核", x: 200, y: 400, active: 0.85, pid: 105, strategy: "rest_for_one" },
        { name: "apeireth-central",    display: "中央",   group: "主核", x: 600, y: 400, active: 0.90, pid: 100, strategy: "rest_for_one" },
        // 治理
        { name: "apeireth-sovereignty",display: "主权",   group: "治理", x: 100, y: 300, active: 0.93, pid: 102, strategy: "one_for_one" },
        { name: "apeireth-council",    display: "智囊团", group: "治理", x: 700, y: 300, active: 0.87, pid: 201, strategy: "one_for_one" },
        { name: "apeireth-life-force", display: "生命力", group: "治理", x: 400, y: 100, active: 0.96, pid: 209, strategy: "rest_for_one" },
        // 器官
        { name: "apeireth-perception",    display: "感知",   group: "器官", x: 50,  y: 100, active: 0.85, pid: 0, strategy: "—" },
        { name: "apeireth-cognition",     display: "认知",   group: "器官", x: 150, y: 50,  active: 0.92, pid: 0, strategy: "—" },
        { name: "apeireth-consciousness", display: "意识",   group: "器官", x: 300, y: 30,  active: 0.95, pid: 0, strategy: "—" },
        { name: "apeireth-memory",        display: "记忆",   group: "器官", x: 500, y: 30,  active: 0.78, pid: 0, strategy: "—" },
        { name: "apeireth-motivation",    display: "动机",   group: "器官", x: 650, y: 50,  active: 0.88, pid: 0, strategy: "—" },
        { name: "apeireth-value",         display: "价值",   group: "器官", x: 750, y: 100, active: 0.90, pid: 0, strategy: "—" },
        { name: "apeireth-relation",      display: "关系",   group: "器官", x: 780, y: 250, active: 0.83, pid: 0, strategy: "—" },
        { name: "apeireth-action",        display: "行动",   group: "器官", x: 750, y: 480, active: 0.86, pid: 0, strategy: "—" },
        // 工具
        { name: "apeireth-api",       display: "API",     group: "工具", x: 80,  y: 500, active: 0.82, pid: 0, strategy: "transient" },
        { name: "apeireth-bus",       display: "总线",    group: "工具", x: 200, y: 550, active: 0.88, pid: 0, strategy: "transient" },
        { name: "apeireth-upgrade",   display: "升级",    group: "工具", x: 320, y: 580, active: 0.75, pid: 0, strategy: "transient" },
        { name: "apeireth-extension", display: "扩展",    group: "工具", x: 450, y: 580, active: 0.70, pid: 0, strategy: "transient" },
        { name: "apeireth-pybridge",  display: "桥",      group: "工具", x: 580, y: 550, active: 0.65, pid: 0, strategy: "transient" },
        { name: "apeireth-tools",     display: "工具集",  group: "工具", x: 720, y: 500, active: 0.78, pid: 0, strategy: "transient" },
        // 测量
        { name: "apeireth-asi",       display: "ASI",     group: "测量", x: 400, y: 200, active: 0.91, pid: 0, strategy: "—" },
        { name: "apeireth-bench",     display: "基准",    group: "测量", x: 350, y: 480, active: 0.68, pid: 0, strategy: "—" },
        { name: "apeireth-test",      display: "测试",    group: "测量", x: 500, y: 480, active: 0.72, pid: 0, strategy: "—" },
        { name: "apeireth-verify",    display: "验证",    group: "测量", x: 450, y: 480, active: 0.69, pid: 0, strategy: "—" },
        // 其他
        { name: "apeireth-cli",       display: "CLI",     group: "总",   x: 100, y: 400, active: 0.80, pid: 0, strategy: "—" },
        { name: "apeireth-evolution", display: "演化",    group: "总",   x: 300, y: 100, active: 0.73, pid: 0, strategy: "—" },
        { name: "apeireth-web",       display: "Web R18", group: "总",   x: 600, y: 480, active: 0.85, pid: 0, strategy: "—" },
        { name: "apeireth-philosophy",display: "哲学",    group: "总",   x: 400, y: 400, active: 0.78, pid: 0, strategy: "—" },
    ];

    function renderStarMap() {
        const svg = document.getElementById("starMap");
        if (!svg) return;

        const W = 800, H = 600;
        svg.setAttribute("viewBox", `0 0 ${W} ${H}`);

        // 5 大组标签
        const groupLabels = [
            { x: 400, y: 580, label: "工具 TRANSIENT" },
            { x: 130, y: 30,  label: "主核 / 治理 / 器官" },
        ];

        // 按组聚合坐标范围
        const groupCenters = {};
        TOPOLOGY_MOCK.forEach((n) => {
            if (!groupCenters[n.group]) groupCenters[n.group] = { xs: [], ys: [] };
            groupCenters[n.group].xs.push(n.x);
            groupCenters[n.group].ys.push(n.y);
        });

        // 5 大组: 中心点 (用于画星图背景星座连线)
        const groups = Object.keys(groupCenters);

        // 背景星座连线 (5 大组中心 → 总监督 中心)
        let lines = "";
        const cx = 400, cy = 300;
        groups.forEach((g) => {
            const xs = groupCenters[g].xs;
            const ys = groupCenters[g].ys;
            const avgX = xs.reduce((a, b) => a + b, 0) / xs.length;
            const avgY = ys.reduce((a, b) => a + b, 0) / ys.length;
            lines += `<line class="star-line" x1="${avgX}" y1="${avgY}" x2="${cx}" y2="${cy}" />`;
        });

        // 节点
        let nodes = "";
        TOPOLOGY_MOCK.forEach((n) => {
            // 亮度 = active 0.5..1, 大小 = active 2..4
            const r = 2 + n.active * 2.5;
            const opacity = 0.4 + n.active * 0.5;
            nodes += `
                <g class="star-crate" data-name="${n.name}" data-active="true" data-group="${n.group}">
                    <circle class="star-crate-glow" cx="${n.x}" cy="${n.y}" r="${r * 2.5}"></circle>
                    <circle class="star-crate-core" cx="${n.x}" cy="${n.y}" r="${r}" opacity="${opacity}"></circle>
                    <text class="star-crate-label" x="${n.x}" y="${n.y - r - 4}">${n.display}</text>
                </g>
            `;
        });

        // 5 大组 label
        let groupLabelsSvg = "";
        groupLabels.forEach((g) => {
            groupLabelsSvg += `<text class="star-group-label" x="${g.x}" y="${g.y}">${g.label}</text>`;
        });

        svg.innerHTML = lines + nodes + groupLabelsSvg;

        // 悬浮高亮
        svg.querySelectorAll(".star-crate").forEach((el) => {
            el.addEventListener("mouseenter", () => {
                const name = el.getAttribute("data-name");
                const node = TOPOLOGY_MOCK.find((n) => n.name === name);
                if (node) {
                    // 工程模式: 显示 PID + strategy (W1 简化: 浮窗 tooltip)
                    if (window.ApeirethStore.get("mode") === "engineer") {
                        console.log(`[${node.name}] pid=${node.pid} strategy=${node.strategy} active=${node.active.toFixed(2)}`);
                    }
                }
            });
        });
    }

    function renderBridgeOrgans() {
        const target = document.getElementById("bridgeOrgans");
        if (!target) return;
        const organs = [
            { name: "perception", display: "感知" },
            { name: "cognition", display: "认知" },
            { name: "memory", display: "记忆" },
            { name: "motivation", display: "动机" },
            { name: "value", display: "价值" },
            { name: "relation", display: "关系" },
            { name: "action", display: "行动" },
            { name: "consciousness", display: "意识" },
            { name: "life_force", display: "生命力" },
        ];
        target.innerHTML = organs.map((o) => `
            <div class="bridge-organ">
                <span class="bridge-organ-dot"></span>
                <span>${o.display}</span>
            </div>
        `).join("");
    }

    function updateAsi() {
        const v = document.getElementById("asiValue");
        const arc = document.getElementById("asiArcProgress");
        if (v) v.textContent = "0.8595";
        if (arc) {
            const ratio = 0.8595 / 0.98;
            arc.setAttribute("x2", (200 * ratio).toString());
        }
    }

    function updateMeta() {
        const s = window.ApeirethStore.get();
        const map = {
            metaMode: s.mode,
            metaTheme: s.theme,
        };
        Object.entries(map).forEach(([id, val]) => {
            const el = document.getElementById(id);
            if (el) el.textContent = val;
        });
    }

    window.ApeirethBridge = { renderStarMap, renderBridgeOrgans, updateAsi, updateMeta };
})();
