/* ===========================================================
 * 开屏 (W2 反馈后简化: 0-1.5s 沉默 → 发丝线 + ΑΠΕΙΡΕΘ → 1.2s 明暗呼吸进入 app)
 *   砍掉: 北极星 (polygon + drop-shadow) — 飞起来不行
 *   砍掉: 呼吸停留 (W1 4.5-6.0s phase 4)
 *   保留: 沉默之前是无限 / 渗出而非点亮 / Esc 跳过
 * =========================================================== */

(function () {
    "use strict";

    function startSplash() {
        const splash = document.getElementById("splash");
        if (!splash) return;

        // 默认 splashEnabled = true, 否则直接跳过
        if (!window.ApeirethStore.get("splashEnabled")) {
            finishSplash();
            return;
        }

        // 时间轴 (W2 简化: 总 3.0s)
        const phase2Timer = setTimeout(() => splash.setAttribute("data-phase", "2"), 1500);
        const phase3Timer = setTimeout(finishSplash, 3000);

        // 跳过: Esc / 点击 / 任何键
        function skip(e) {
            if (e && e.type === "keydown" && e.key !== "Escape") return;
            clearTimeout(phase2Timer);
            clearTimeout(phase3Timer);
            splash.setAttribute("data-phase", "2");
            setTimeout(finishSplash, 1200);
            document.removeEventListener("keydown", skip);
            splash.removeEventListener("click", skip);
        }

        document.addEventListener("keydown", skip);
        splash.addEventListener("click", skip);
    }

    function finishSplash() {
        const splash = document.getElementById("splash");
        const app = document.querySelector(".app");
        if (splash) {
            splash.setAttribute("data-done", "true");
        }
        if (app) {
            app.setAttribute("data-state", "ready");
        }
        window.ApeirethStore.set({ splashDone: true });
    }

    window.ApeirethSplash = { start: startSplash };
})();
