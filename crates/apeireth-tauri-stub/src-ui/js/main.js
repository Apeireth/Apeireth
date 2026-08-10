/* ===========================================================
 * 主入口: 串起所有模块
 *   1. 初始化 store (从 <html> data-* 读初始 mode/theme)
 *   2. 启动开屏 (splash.js)
 *   3. 开屏完成后渲染各模块
 *   4. 监听 store 变化, 联动 CSS / UI
 * =========================================================== */

(function () {
    "use strict";

    function initStore() {
        const html = document.documentElement;
        const s = window.ApeirethStore.get();
        s.mode = html.getAttribute("data-mode") || "focus";
        s.theme = html.getAttribute("data-theme") || "archaic";
        s.launchPage = "bridge";
        // 同步一次 (让 store 触发 emit)
        window.ApeirethStore.set({ mode: s.mode, theme: s.theme });
    }

    function bindStore() {
        // 主题/模式变化时, 重渲染舰桥 meta
        window.ApeirethStore.on("mode", () => {
            if (window.ApeirethBridge) window.ApeirethBridge.updateMeta();
        });
        window.ApeirethStore.on("theme", () => {
            if (window.ApeirethBridge) window.ApeirethBridge.updateMeta();
        });
    }

    function renderAll() {
        if (window.ApeirethOrgans) window.ApeirethOrgans.render();
        if (window.ApeirethTimeline) window.ApeirethTimeline.render();
        if (window.ApeirethBridge) {
            window.ApeirethBridge.renderStarMap();
            window.ApeirethBridge.renderBridgeOrgans();
            window.ApeirethBridge.updateAsi();
            window.ApeirethBridge.updateMeta();
        }
        if (window.ApeirethDialogue) window.ApeirethDialogue.init();
        if (window.ApeirethModeTheme) window.ApeirethModeTheme.init();
        if (window.ApeirethNav) window.ApeirethNav.init();
    }

    function onSplashDone() {
        window.ApeirethStore.on("splashDone", (done) => {
            if (done) renderAll();
        });
        // 兼容: splash 可能立即完成 (例如设置里 splashEnabled=false)
        if (window.ApeirethStore.get("splashDone")) {
            renderAll();
        }
    }

    function main() {
        initStore();
        bindStore();
        onSplashDone();
        if (window.ApeirethSplash) {
            window.ApeirethSplash.start();
        } else {
            renderAll();
        }
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", main);
    } else {
        main();
    }
})();
