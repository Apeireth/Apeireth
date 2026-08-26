/* ===========================================================
 * Apeireth R19 — 50 行 pub/sub store
 * 不引依赖, 不引框架, 完全 vanilla JS
 * =========================================================== */

(function () {
    "use strict";

    const Store = {
        // ---------- 状态 ----------
        state: {
            mode: "focus",            // focus | engineer
            theme: "archaic",         // archaic | era
            language: "zh",           // zh | en
            launchPage: "bridge",     // bridge | dialogue
            splashEnabled: true,
            breathEnabled: true,
            currentPage: "bridge",    // 启屏默认舰桥
            splashDone: false,        // 开屏是否完成
        },

        // ---------- 订阅者 ----------
        subscribers: new Map(),

        // ---------- 订阅 ----------
        on(key, fn) {
            if (!this.subscribers.has(key)) this.subscribers.set(key, new Set());
            this.subscribers.get(key).add(fn);
            return () => this.subscribers.get(key).delete(fn);
        },

        // ---------- 通知 ----------
        emit(key, value) {
            const subs = this.subscribers.get(key);
            if (subs) subs.forEach((fn) => fn(value));
        },

        // ---------- 设置状态 ----------
        set(partial) {
            const changed = {};
            for (const k in partial) {
                if (this.state[k] !== partial[k]) {
                    changed[k] = partial[k];
                    this.state[k] = partial[k];
                }
            }
            // 同步到 <html> 属性 (CSS 主题/模式联动)
            if (changed.mode) document.documentElement.setAttribute("data-mode", changed.mode);
            if (changed.theme) document.documentElement.setAttribute("data-theme", changed.theme);
            if (changed.breathEnabled !== undefined) {
                document.body.classList.toggle("breath-disabled", !changed.breathEnabled);
            }
            if (changed.currentPage) {
                this.emit("page", changed.currentPage);
            }
            Object.keys(changed).forEach((k) => this.emit(k, changed[k]));
            return changed;
        },

        // ---------- 读取 ----------
        get(key) {
            return key ? this.state[key] : this.state;
        },
    };

    // 暴露到 window
    window.ApeirethStore = Store;
})();
