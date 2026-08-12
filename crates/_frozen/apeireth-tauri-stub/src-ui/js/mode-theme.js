/* ===========================================================
 * 2×2 模式 × 主题 切换
 *   模式: focus | engineer
 *   主题: archaic | era
 * 切换 = 1.2s 明暗呼吸遮罩
 * =========================================================== */

(function () {
    "use strict";

    function setMode(mode) {
        window.ApeirethStore.set({ mode });
        // UI 同步
        document.querySelectorAll(".pill-mode").forEach((p) => {
            p.setAttribute("data-active", p.getAttribute("data-mode") === mode ? "true" : "false");
        });
    }

    function setTheme(theme) {
        window.ApeirethStore.set({ theme });
        document.querySelectorAll(".pill-theme").forEach((p) => {
            p.setAttribute("data-active", p.getAttribute("data-theme") === theme ? "true" : "false");
        });
    }

    function init() {
        // 主题 pills
        document.querySelectorAll(".pill-theme").forEach((p) => {
            p.addEventListener("click", () => setTheme(p.getAttribute("data-theme")));
        });
        // 模式 pills
        document.querySelectorAll(".pill-mode").forEach((p) => {
            p.addEventListener("click", () => setMode(p.getAttribute("data-mode")));
        });
        // 快捷键 Ctrl/Cmd+Shift+E 切模式
        document.addEventListener("keydown", (e) => {
            if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "e") {
                e.preventDefault();
                const cur = window.ApeirethStore.get("mode");
                setMode(cur === "focus" ? "engineer" : "focus");
            }
        });

        // 同步初始
        const s = window.ApeirethStore.get();
        setMode(s.mode);
        setTheme(s.theme);
    }

    window.ApeirethModeTheme = { init, setMode, setTheme };
})();
