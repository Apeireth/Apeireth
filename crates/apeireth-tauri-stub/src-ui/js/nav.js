/* ===========================================================
 * Nav 切换 (5 nav: dialogue / growth / chronicle / bridge / settings)
 * 当前项 = 呼吸光点
 * =========================================================== */

(function () {
    "use strict";

    function switchTo(page) {
        // 当前 nav 激活
        document.querySelectorAll(".nav-item").forEach((el) => {
            el.setAttribute("data-active", el.getAttribute("data-page") === page ? "true" : "false");
        });
        // 当前 page 显示
        document.querySelectorAll(".page").forEach((p) => {
            p.hidden = p.getAttribute("data-page") !== page;
        });
        window.ApeirethStore.set({ currentPage: page });
    }

    function init() {
        // nav 点击
        document.querySelectorAll(".nav-link").forEach((link) => {
            link.addEventListener("click", (e) => {
                e.preventDefault();
                const item = link.closest(".nav-item");
                if (item) switchTo(item.getAttribute("data-page"));
            });
        });
        // URL hash 同步
        function onHashChange() {
            const hash = location.hash.replace("#", "");
            if (hash && ["dialogue", "growth", "chronicle", "bridge", "settings"].includes(hash)) {
                switchTo(hash);
            }
        }
        window.addEventListener("hashchange", onHashChange);
        // 初始: 启屏默认 = bridge
        const initial = window.ApeirethStore.get("launchPage") || "bridge";
        switchTo(initial);
        if (location.hash !== `#${initial}`) {
            history.replaceState(null, "", `#${initial}`);
        }
    }

    window.ApeirethNav = { init, switchTo };
})();
