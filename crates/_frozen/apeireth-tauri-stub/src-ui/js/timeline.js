/* ===========================================================
 * 8 阶段时间线 (R19 砍衰老病死, 后端 10 enum 仍存)
 * =========================================================== */

(function () {
    "use strict";

    const STAGES = [
        { idx: 1, zh: "孕育", en: "Gestation" },
        { idx: 2, zh: "诞生", en: "Birth" },
        { idx: 3, zh: "幼儿", en: "Infancy" },
        { idx: 4, zh: "成长", en: "Growth" },
        { idx: 5, zh: "成熟", en: "Maturity" },
        { idx: 6, zh: "繁衍", en: "Reproduction" },
        { idx: 7, zh: "迁移", en: "Migration" },
        { idx: 8, zh: "重生", en: "Rebirth" },
    ];

    const ACTIVE_STAGE = 4; // R19 W1 mock: 当前在"成长"

    function render() {
        const target = document.getElementById("timeline");
        if (!target) return;

        target.innerHTML = STAGES.map((s) => {
            const isActive = s.idx === ACTIVE_STAGE;
            const isPast = s.idx < ACTIVE_STAGE;
            return `
                <div class="timeline-stage"
                     data-stage="${s.idx}"
                     data-active="${isActive}"
                     data-past="${isPast}">
                    <div class="timeline-idx">${String(s.idx).padStart(2, '0')}</div>
                    <div class="timeline-zh">${s.zh}</div>
                    <div class="timeline-en">${s.en}</div>
                </div>
            `;
        }).join("");
    }

    window.ApeirethTimeline = { render };
})();
