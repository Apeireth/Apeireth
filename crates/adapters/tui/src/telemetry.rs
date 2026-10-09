//! 遥测面板组数据模型: 伙伴的内在遥测 (记忆账本 / 缓存命中 / 上下文波形 /
//! 治理灯阵)。纯状态 + 纯渲染助手, 无 I/O —— 数据来源见后端端口
//! [`crate::backend::CockpitBackend::fetch_telemetry`]。
//!
//! 数据诚实纪律: 没数据 = 显式 "—" / 暗格, 绝不画假曲线、绝不编数。
//! 数据源未接线的行/面板显式标 "未接线" (沿用 P1 桩的诚实语法)。

use std::collections::VecDeque;

use crate::theme::MotionTokens;

/// 记忆账本点阵: 每格 = 单位计数 (像内存点阵), 超出容量以 `›` 标溢出。
pub const LEDGER_CELLS: usize = 10;
/// 治理灯阵槽位数 (最近 N 个治理判定)。
pub const LAMP_SLOTS: usize = 12;
/// 上下文波形默认宽度 (格)。
pub const WAVE_WIDTH: usize = 14;

/// 点阵点亮格。
pub const CELL_LIT: &str = "▪";
/// 点阵暗格 (无该单位计数 / 无数据)。
pub const CELL_DARK: &str = "▫";
/// 点阵溢出标记 (计数超出格容量, 精确值照数字给)。
pub const CELL_OVER: &str = "›";
/// 诚实占位: 数据缺位时显式 "—", 不猜不装。
pub const HONEST_PLACEHOLDER: &str = "—";

/// 波形帧循环字形 (5 档, 从低到高)。
pub const WAVE_GLYPHS: [&str; 5] = ["▁", "▂", "▃", "▅", "▇"];

/// 治理判定三色语义: 绿=放行 / 琥珀=审批 / 红=拒绝。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovVerdict {
    /// 放行。
    Allow,
    /// 挂起等待审批。
    Approve,
    /// 拒绝。
    Reject,
}

impl GovVerdict {
    /// 从治理事件判定字段解析 (契约标签 `allow` / `require_approval` / `deny`)。
    /// 未知判定不猜色: 返回 None, 灯位保持暗格。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "allow" => Some(Self::Allow),
            "require_approval" => Some(Self::Approve),
            "deny" => Some(Self::Reject),
            _ => None,
        }
    }

    /// 上屏名 (无障碍语义, 不只靠颜色区分)。
    pub fn label(self) -> &'static str {
        match self {
            Self::Allow => "放行",
            Self::Approve => "审批",
            Self::Reject => "拒绝",
        }
    }
}

/// 一个治理判定事件 (灯阵一盏灯的来源)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceEvent {
    /// 事件时刻 (epoch 毫秒, 事件流时间序)。
    pub at_ms: i64,
    /// 能力 id (灯阵去重键的一部分)。
    pub capability: String,
    /// 判定。
    pub verdict: GovVerdict,
}

/// 记忆账本计数四行 (会话/记忆/保护/教训); 每行独立诚实: 探不到就 None。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LedgerCounts {
    /// 会话数。
    pub sessions: Option<u64>,
    /// 记忆件数。
    pub memories: Option<u64>,
    /// 保护件数。
    pub protected: Option<u64>,
    /// 教训数。
    pub lessons: Option<u64>,
}

impl LedgerCounts {
    /// 四行 (行名, 计数) 全集, 顺序即上屏顺序。
    pub fn rows(&self) -> [(&'static str, Option<u64>); 4] {
        [
            ("会话", self.sessions),
            ("记忆", self.memories),
            ("保护", self.protected),
            ("教训", self.lessons),
        ]
    }

    /// 有真实计数的行数 (角标签 `LEDGER · {n} 库` 的 `n`)。
    pub fn backed_rows(&self) -> usize {
        self.rows()
            .iter()
            .filter(|(_, value)| value.is_some())
            .count()
    }
}

/// 点阵一行 (纯字符串, 测试直接断言): `会话 ▪▪▪▫▫▫▫▫▫▫ 3`。
///
/// 每格 = 单位计数; 计数超出格容量时全亮 + `›` 溢出标记 (精确值照数字给);
/// 无数据 = 全暗格 + "—" (不编数)。
pub fn ledger_line(label: &str, value: Option<u64>, cells: usize) -> String {
    let (lit, overflow) = match value {
        Some(count) => {
            let lit = (count as usize).min(cells);
            (lit, count as usize > cells)
        }
        None => (0, false),
    };
    let mut dots = String::new();
    for index in 0..cells {
        if index < lit {
            dots.push_str(CELL_LIT);
        } else {
            dots.push_str(CELL_DARK);
        }
    }
    if overflow {
        dots.push_str(CELL_OVER);
    }
    let value = value
        .map(|count| count.to_string())
        .unwrap_or_else(|| HONEST_PLACEHOLDER.to_string());
    format!("{label} {dots} {value}")
}

/// 缓存命中率大数字仪表: 变化时短暂高亮 (新值闪一帧), 无数据显式 "—"。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CacheGauge {
    value: Option<u32>,
    changed_at_frame: Option<u64>,
}

impl CacheGauge {
    /// 喂入新读数: 数值变化才算「新值」, 闪一帧。
    pub fn set(&mut self, value: Option<u32>, frame: u64) {
        if value.is_some() && value != self.value {
            self.changed_at_frame = Some(frame);
        }
        self.value = value;
    }

    /// 当前读数。
    pub fn value(&self) -> Option<u32> {
        self.value
    }

    /// 该帧是否处于变化闪烁 (亮度脉冲一帧)。
    pub fn flash_active(&self, frame: u64) -> bool {
        self.changed_at_frame == Some(frame)
    }

    /// 大数字文本 (`87%` / `—`)。
    pub fn render(&self) -> String {
        match self.value {
            Some(rate) => format!("{rate}%"),
            None => HONEST_PLACEHOLDER.to_string(),
        }
    }
}

/// 上下文用量波形: 滚动 sparkline (每动画帧走纸一格, 采样 = 最新真实用量)。
///
/// 走纸语义: 每 tick 推进一格, 新格取最近一次真实采样 (保持式走纸, 如走纸
/// 记录仪); 从未有过采样 = 显式 "—", 绝不画假曲线。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextWave {
    samples: VecDeque<u32>,
    last: Option<u32>,
    capacity: usize,
}

impl Default for ContextWave {
    fn default() -> Self {
        Self::new(WAVE_WIDTH)
    }
}

impl ContextWave {
    /// 指定窗口宽度 (格) 的波形。
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: VecDeque::new(),
            last: None,
            capacity: capacity.max(1),
        }
    }

    /// 真实采样 (上下文用量读数) 进一格。
    pub fn observe(&mut self, total_tokens: u32) {
        self.last = Some(total_tokens);
        self.push_cell(total_tokens);
    }

    /// 动画帧走纸一格: 无真实采样时不画格 (诚实空转)。
    pub fn advance(&mut self) {
        if let Some(last) = self.last {
            self.push_cell(last);
        }
    }

    fn push_cell(&mut self, value: u32) {
        self.samples.push_back(value);
        while self.samples.len() > self.capacity {
            self.samples.pop_front();
        }
    }

    /// 采样数 (测试用)。
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// 波形行: `▁▂▃▅▇` 按用量比例取档; 无采样显式 "—"。
    ///
    /// 纵向刻度: 窗口已知按窗口归一 (绝对刻度); 窗口未知按观测峰值归一
    /// (相对形状, 绝对刻度缺口在角标签 `CTX · ?` 如实标注)。
    pub fn render(&self, width: usize, window: Option<u32>) -> String {
        if self.samples.is_empty() {
            return HONEST_PLACEHOLDER.to_string();
        }
        let observed_max = self.samples.iter().copied().max().unwrap_or(0);
        let scale = window.unwrap_or(observed_max).max(1);
        let width = width.max(1);
        let start = self.samples.len().saturating_sub(width);
        self.samples
            .iter()
            .skip(start)
            .map(|value| {
                let ratio = (f64::from(*value) / f64::from(scale)).clamp(0.0, 1.0);
                let index = (ratio * (WAVE_GLYPHS.len() as f64 - 1.0)).round() as usize;
                WAVE_GLYPHS[index.min(WAVE_GLYPHS.len() - 1)]
            })
            .collect()
    }
}

/// 治理灯阵一格: 判定 (None = 暗格) + 渐入亮度 (0-15)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LampCell {
    /// 判定; None = 暗格 (无事件 / 未知判定不猜色)。
    pub verdict: Option<GovVerdict>,
    /// 亮度 (0-15): 新灯从暗到亮渐入。
    pub level: u8,
}

/// 治理灯阵: 最近 N 个治理判定的小灯阵 (按事件流时间序点亮)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LampArray {
    lamps: VecDeque<(GovernanceEvent, u64)>,
    capacity: usize,
}

impl Default for LampArray {
    fn default() -> Self {
        Self::new(LAMP_SLOTS)
    }
}

impl LampArray {
    /// 指定槽位数的灯阵。
    pub fn new(capacity: usize) -> Self {
        Self {
            lamps: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    /// 已点亮灯数。
    pub fn len(&self) -> usize {
        self.lamps.len()
    }

    /// 是否一盏都没有。
    pub fn is_empty(&self) -> bool {
        self.lamps.is_empty()
    }

    /// 吸收一帧事件流: 按 (时刻, 能力, 判定) 去重, 新灯记渐入起点帧;
    /// 灯位按事件时刻升序 (时间序点亮), 溢出丢最旧。
    pub fn absorb(&mut self, events: &[GovernanceEvent], frame: u64) {
        for event in events {
            let known = self.lamps.iter().any(|(lamp, _)| {
                lamp.at_ms == event.at_ms
                    && lamp.capability == event.capability
                    && lamp.verdict == event.verdict
            });
            if !known {
                self.lamps.push_back((event.clone(), frame));
            }
        }
        let mut ordered: Vec<(GovernanceEvent, u64)> = self.lamps.drain(..).collect();
        ordered.sort_by_key(|(event, _)| event.at_ms);
        while ordered.len() > self.capacity {
            ordered.remove(0);
        }
        self.lamps = ordered.into();
    }

    /// 灯阵格 (定长 [`LAMP_SLOTS`] 格: 左→右时间序, 缺位补暗格)。
    pub fn cells(&self, frame: u64, motion: &MotionTokens) -> Vec<LampCell> {
        let mut cells: Vec<LampCell> = self
            .lamps
            .iter()
            .map(|(event, born)| {
                let age = frame.saturating_sub(*born);
                LampCell {
                    verdict: Some(event.verdict),
                    level: motion.lamp_level(age),
                }
            })
            .collect();
        while cells.len() < self.capacity {
            cells.push(LampCell {
                verdict: None,
                level: 0,
            });
        }
        cells
    }
}

/// 一帧遥测快照 (后端端口回灌; 每节独立诚实: 探不到给 None + 备注)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TelemetrySnapshot {
    /// 记忆账本计数。
    pub ledger: LedgerCounts,
    /// 缓存命中率 (接口无此字段 = None → 上屏 "—")。
    pub cache_hit_rate: Option<u32>,
    /// 治理判定事件流 (时间序)。
    pub governance: Vec<GovernanceEvent>,
    /// 来源备注 (未接线 / 窗口饱和 / 端点失败, 如实上屏)。
    pub notes: Vec<String>,
}

/// 遥测面板组的应用状态 (渲染层只读)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TelemetryModel {
    /// 记忆账本点阵。
    pub ledger: LedgerCounts,
    /// 缓存大数字仪表。
    pub cache: CacheGauge,
    /// 上下文用量波形。
    pub wave: ContextWave,
    /// 治理灯阵。
    pub lamps: LampArray,
    /// 来源备注。
    pub notes: Vec<String>,
}

impl TelemetryModel {
    /// 回灌一帧遥测快照 (灯阵按当前帧记渐入起点)。
    pub fn absorb(&mut self, snapshot: TelemetrySnapshot, frame: u64) {
        self.ledger = snapshot.ledger;
        self.cache.set(snapshot.cache_hit_rate, frame);
        self.lamps.absorb(&snapshot.governance, frame);
        self.notes = snapshot.notes;
    }

    /// 回合计量采样 (真实读数: 缓存命中率 + 上下文用量)。
    pub fn observe_usage(&mut self, cache_hit_rate: Option<u32>, total_tokens: u32, frame: u64) {
        self.cache.set(cache_hit_rate, frame);
        self.wave.observe(total_tokens);
    }

    /// 动画帧: 波形走纸一格 (reduced 档无动画帧, 波形静止)。
    pub fn on_tick(&mut self, motion: &MotionTokens) {
        if motion.animations_enabled() {
            self.wave.advance();
        }
    }
}

/// 大号等宽数字字形 (5 行 × 3 列/字, 像素块): `0-9` / `%` / `—` / 空格。
pub fn big_digit_rows(ch: char) -> [&'static str; 5] {
    match ch {
        '0' => ["███", "█ █", "█ █", "█ █", "███"],
        '1' => ["  █", "  █", "  █", "  █", "  █"],
        '2' => ["███", "  █", "███", "█  ", "███"],
        '3' => ["███", "  █", "███", "  █", "███"],
        '4' => ["█ █", "█ █", "███", "  █", "  █"],
        '5' => ["███", "█  ", "███", "  █", "███"],
        '6' => ["███", "█  ", "███", "█ █", "███"],
        '7' => ["███", "  █", "  █", "  █", "  █"],
        '8' => ["███", "█ █", "███", "█ █", "███"],
        '9' => ["███", "█ █", "███", "  █", "███"],
        '%' => ["█ █", "  █", " █ ", "█  ", "█ █"],
        '—' => ["   ", "   ", "███", "   ", "   "],
        _ => ["   ", "   ", "   ", "   ", "   "],
    }
}

/// 大号数字渲染 (5 行, 字间 1 空格): `87%` / `—`。
pub fn big_number_rows(text: &str) -> Vec<String> {
    let mut rows = vec![String::new(); 5];
    for (index, ch) in text.chars().enumerate() {
        let glyph = big_digit_rows(ch);
        for (row, line) in rows.iter_mut().enumerate() {
            if index > 0 {
                line.push(' ');
            }
            line.push_str(glyph[row]);
        }
    }
    rows
}

/// 上下文窗口角标签: `128K` / `?` (窗口未知如实标注)。
pub fn window_tag(window: Option<u32>) -> String {
    match window {
        None => "?".to_string(),
        Some(value) if value >= 1_000_000 && value % 1_000_000 == 0 => {
            format!("{}M", value / 1_000_000)
        }
        Some(value) if value >= 1_000 && value % 1_000 == 0 => format!("{}K", value / 1_000),
        Some(value) => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{CockpitTheme, MotionMode};

    /// 点阵计数渲染: 每格 = 单位计数, 溢出标 `›`, 无数据全暗 + "—"。
    #[test]
    fn ledger_dot_matrix_counts_one_cell_per_unit() {
        assert_eq!(ledger_line("会话", Some(3), 10), "会话 ▪▪▪▫▫▫▫▫▫▫ 3");
        assert_eq!(ledger_line("记忆", Some(10), 10), "记忆 ▪▪▪▪▪▪▪▪▪▪ 10");
        // 溢出: 全亮 + 溢出标记, 精确值照数字给 (不截断成假数)。
        assert_eq!(ledger_line("保护", Some(23), 10), "保护 ▪▪▪▪▪▪▪▪▪▪› 23");
        // 零也是真实数据: 全暗但数字是 0, 不是 "—"。
        assert_eq!(ledger_line("教训", Some(0), 10), "教训 ▫▫▫▫▫▫▫▫▫▫ 0");
        // 无数据: 显式 "—", 绝不编数。
        assert_eq!(ledger_line("教训", None, 10), "教训 ▫▫▫▫▫▫▫▫▫▫ —");
    }

    /// 账本四行: 行集齐全, 角标签计数只数有真实数据的行。
    #[test]
    fn ledger_rows_report_backed_count_honestly() {
        let ledger = LedgerCounts {
            sessions: Some(2),
            memories: Some(5),
            protected: Some(1),
            lessons: None,
        };
        assert_eq!(ledger.rows().len(), 4);
        assert_eq!(ledger.backed_rows(), 3);
        assert_eq!(LedgerCounts::default().backed_rows(), 0);
    }

    /// sparkline 帧循环推进: 每 tick 走纸一格, 无采样显式 "—" (不画假曲线)。
    #[test]
    fn context_wave_advances_one_cell_per_tick() {
        let mut wave = ContextWave::new(4);
        assert_eq!(wave.render(4, Some(100)), HONEST_PLACEHOLDER);

        wave.observe(50);
        assert_eq!(wave.sample_count(), 1);
        wave.advance();
        assert_eq!(wave.sample_count(), 2, "每 tick 推进一格");
        wave.advance();
        wave.advance();
        wave.advance();
        assert_eq!(wave.sample_count(), 4, "窗口容量滚动 (旧格滑出)");

        // 比例取档: 50/100 → 中档, 满量程 → 顶档。
        assert_eq!(wave.render(4, Some(100)), "▃▃▃▃");
        let mut full = ContextWave::new(3);
        full.observe(100);
        full.observe(0);
        assert_eq!(full.render(3, Some(100)), "▇▁");

        // 从未采样: advance 不得画格 (绝不画假曲线)。
        let mut empty = ContextWave::new(3);
        empty.advance();
        empty.advance();
        assert_eq!(empty.sample_count(), 0);
        assert_eq!(empty.render(3, None), HONEST_PLACEHOLDER);
    }

    /// 大数字闪烁: 新值到达闪一帧, 同值不闪, 无值不闪。
    #[test]
    fn big_digit_flashes_one_frame_on_change() {
        let mut gauge = CacheGauge::default();
        gauge.set(Some(87), 5);
        assert!(gauge.flash_active(5), "新值到达当帧闪烁");
        assert!(!gauge.flash_active(6), "只闪一帧");
        assert_eq!(gauge.render(), "87%");

        gauge.set(Some(87), 7);
        assert!(!gauge.flash_active(7), "同值不算新值");
        gauge.set(Some(91), 8);
        assert!(gauge.flash_active(8), "变化帧再次闪烁");

        gauge.set(None, 9);
        assert!(!gauge.flash_active(9), "无数据不闪");
        assert_eq!(gauge.render(), HONEST_PLACEHOLDER);
    }

    /// 治理灯阵点亮序: 按事件流时间序点亮 (左→右), 判定三色语义, 缺位暗格。
    #[test]
    fn governance_lamps_light_in_event_time_order() {
        let motion = CockpitTheme::new(MotionMode::Full).motion;
        let mut lamps = LampArray::new(4);
        lamps.absorb(
            &[
                GovernanceEvent {
                    at_ms: 300,
                    capability: "tool.c".into(),
                    verdict: GovVerdict::Reject,
                },
                GovernanceEvent {
                    at_ms: 100,
                    capability: "tool.a".into(),
                    verdict: GovVerdict::Allow,
                },
                GovernanceEvent {
                    at_ms: 200,
                    capability: "tool.b".into(),
                    verdict: GovVerdict::Approve,
                },
            ],
            0,
        );

        let cells = lamps.cells(6, &motion);
        assert_eq!(cells.len(), 4);
        let order: Vec<Option<GovVerdict>> = cells.iter().map(|cell| cell.verdict).collect();
        assert_eq!(
            order,
            vec![
                Some(GovVerdict::Allow),
                Some(GovVerdict::Approve),
                Some(GovVerdict::Reject),
                None,
            ],
            "灯位按事件时刻升序, 缺位补暗格"
        );

        // 三色语义: 绿=放行 / 琥珀=审批 / 红=拒绝 (标签不只靠颜色)。
        assert_eq!(GovVerdict::Allow.label(), "放行");
        assert_eq!(GovVerdict::Approve.label(), "审批");
        assert_eq!(GovVerdict::Reject.label(), "拒绝");
        assert_eq!(GovVerdict::parse("allow"), Some(GovVerdict::Allow));
        assert_eq!(
            GovVerdict::parse("require_approval"),
            Some(GovVerdict::Approve)
        );
        assert_eq!(GovVerdict::parse("deny"), Some(GovVerdict::Reject));
        // 未知判定不猜色。
        assert_eq!(GovVerdict::parse("maybe"), None);

        // 重复灌入同一批事件不重复点灯 (轮询去重)。
        lamps.absorb(
            &[GovernanceEvent {
                at_ms: 100,
                capability: "tool.a".into(),
                verdict: GovVerdict::Allow,
            }],
            9,
        );
        assert_eq!(lamps.len(), 3);
    }

    /// 治理灯点亮动画: 新灯从暗到亮渐入; reduced 档瞬时点亮。
    #[test]
    fn governance_lamp_fades_in_from_dark_and_is_instant_in_reduced() {
        let full = CockpitTheme::new(MotionMode::Full).motion;
        let mut lamps = LampArray::new(3);
        lamps.absorb(
            &[GovernanceEvent {
                at_ms: 1,
                capability: "tool.a".into(),
                verdict: GovVerdict::Allow,
            }],
            10,
        );
        let born = lamps.cells(10, &full)[0];
        let grown = lamps.cells(10 + full.lamp_fade_frames, &full)[0];
        assert!(born.level < grown.level, "新灯从暗到亮渐入");
        assert_eq!(grown.level, 15);

        let reduced = CockpitTheme::new(MotionMode::Reduced).motion;
        assert_eq!(lamps.cells(10, &reduced)[0].level, 15, "reduced 档瞬时点亮");
    }

    /// 无数据诚实占位: 账本 "—" / 波形 "—" / 大数字 "—" / 灯阵暗格。
    #[test]
    fn no_data_renders_honest_placeholders_and_dark_cells() {
        let model = TelemetryModel::default();
        for (label, value) in model.ledger.rows() {
            let line = ledger_line(label, value, LEDGER_CELLS);
            assert!(line.ends_with(HONEST_PLACEHOLDER), "{line}");
        }
        assert_eq!(model.cache.render(), HONEST_PLACEHOLDER);
        assert_eq!(
            model.wave.render(WAVE_WIDTH, None),
            HONEST_PLACEHOLDER,
            "无采样绝不画曲线"
        );
        let motion = CockpitTheme::new(MotionMode::Full).motion;
        for cell in model.lamps.cells(0, &motion) {
            assert_eq!(cell.verdict, None, "无事件 = 暗格");
            assert_eq!(cell.level, 0);
        }
    }

    /// 大号数字字形: 文本逐字放大成 5 行等宽像素块。
    #[test]
    fn big_number_glyphs_render_five_rows() {
        let rows = big_number_rows("87%");
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0], "███ ███ █ █");
        for row in &rows {
            // 三字 + 两个字间空格 = 11 列等宽。
            assert_eq!(row.chars().count(), 11, "{rows:?}");
        }
        let dash = big_number_rows(HONEST_PLACEHOLDER);
        assert_eq!(dash[2].trim(), "███", "无数据大数字 = 破折号");
    }
}
