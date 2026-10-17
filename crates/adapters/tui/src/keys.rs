//! 键位表: 终端按键 → 驾驶舱动作 (帮助浮层与键位测试共用一份)。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// 驾驶舱动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 打开命令行 (`/`)。
    OpenCommand,
    /// 帮助浮层 (`?`)。
    ToggleHelp,
    /// 发送 / 执行命令 (Enter)。
    Submit,
    /// Esc: 关浮层 / 退出命令行 / 双击触发时间倒带。
    Escape,
    /// 向上翻页。
    PageUp,
    /// 向下翻页。
    PageDown,
    /// 下一个会话页签。
    NextTab,
    /// 上一个会话页签。
    PrevTab,
    /// 两段式退出的第一/二段 (Ctrl+C)。
    QuitStep,
    /// 打开/关闭任务面板 (Ctrl+T; `/tasks` 打开)。
    ToggleTasks,
    /// 任务面板: 选上一行 (↑ / k)。
    SelectPrev,
    /// 任务面板: 选下一行 (↓ / j)。
    SelectNext,
    /// 任务面板: 人判重试选中的 failed 任务 (r)。
    JudgeRetry,
    /// 退格。
    Backspace,
    /// 命令补全 (Tab)。
    Complete,
    /// 可打印字符。
    Char(char),
    /// 忽略。
    Ignored,
}

/// 按键上下文: 光标在正文、输入/命令行, 还是任务面板 (模态)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMode {
    /// 正文浏览态。
    Normal,
    /// 输入态 (命令行或普通输入)。
    Input,
    /// 任务面板态 (模态: 面板键位独占, 只读 + 人判重试)。
    PanelTasks,
}

/// 一行键位说明 (帮助浮层直接上屏)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBinding {
    /// 按键。
    pub keys: &'static str,
    /// 动作。
    pub action: &'static str,
    /// 补充说明。
    pub note: &'static str,
}

/// 键位表。
pub const KEY_TABLE: &[KeyBinding] = &[
    KeyBinding {
        keys: "/",
        action: "打开命令行",
        note: "带命令补全浮层",
    },
    KeyBinding {
        keys: "?",
        action: "帮助浮层",
        note: "键位表 + 命令表",
    },
    KeyBinding {
        keys: "Enter",
        action: "发送 / 执行命令",
        note: "",
    },
    KeyBinding {
        keys: "Esc",
        action: "关闭浮层 / 退出命令行",
        note: "命令行内清空输入",
    },
    KeyBinding {
        keys: "Esc Esc",
        action: "时间倒带",
        note: "回滚上一回合重新输入 (本批接口桩)",
    },
    KeyBinding {
        keys: "Ctrl+C",
        action: "两段式退出",
        note: "第一段断流, 第二段退出",
    },
    KeyBinding {
        keys: "PgUp / PgDn",
        action: "对话滚动",
        note: "",
    },
    KeyBinding {
        keys: "Tab",
        action: "命令补全",
        note: "命令行内补全命令名",
    },
    KeyBinding {
        keys: "Ctrl+Tab / Ctrl+Shift+Tab",
        action: "切换会话页签",
        note: "",
    },
    KeyBinding {
        keys: "Ctrl+T",
        action: "任务面板",
        note: "/tasks 同口; 只读 + 人判重试",
    },
    KeyBinding {
        keys: "↑ / ↓",
        action: "任务面板选择",
        note: "面板内 j/k 同义",
    },
    KeyBinding {
        keys: "r",
        action: "人判重试",
        note: "任务面板内, 仅 failed (失败不自动重试)",
    },
    KeyBinding {
        keys: "Backspace",
        action: "删除输入",
        note: "",
    },
];

/// 把一次按键事件映射为驾驶舱动作 (只响应按下/重复, 释放忽略)。
pub fn map_key(key: KeyEvent, mode: KeyMode) -> Action {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return Action::Ignored;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if ctrl && key.code == KeyCode::Char('c') {
        return Action::QuitStep;
    }
    if ctrl && key.code == KeyCode::Char('t') {
        return Action::ToggleTasks;
    }
    if mode == KeyMode::PanelTasks {
        // 任务面板 (模态): 只读导航 + 人判重试, 其余键不进输入行 (防越权面)。
        return match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::PageUp => Action::SelectPrev,
            KeyCode::Down | KeyCode::Char('j') | KeyCode::PageDown => Action::SelectNext,
            KeyCode::Char('r') => Action::JudgeRetry,
            KeyCode::Esc => Action::Escape,
            _ => Action::Ignored,
        };
    }
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::ALT) => Action::Ignored,
        KeyCode::Char(' ') if ctrl => Action::Ignored,
        KeyCode::BackTab => Action::PrevTab,
        KeyCode::Tab if ctrl => Action::NextTab,
        KeyCode::Tab => Action::Complete,
        KeyCode::Enter => Action::Submit,
        KeyCode::Esc => Action::Escape,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Backspace => Action::Backspace,
        KeyCode::Char('/') if mode == KeyMode::Normal => Action::OpenCommand,
        KeyCode::Char('?') if mode == KeyMode::Normal => Action::ToggleHelp,
        KeyCode::Char(ch) if !ctrl => Action::Char(ch),
        _ => Action::Ignored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    /// 快捷键表齐备: 驾驶舱六类快捷键都在表里。
    #[test]
    fn key_table_covers_cockpit_actions() {
        let keys: Vec<&str> = KEY_TABLE.iter().map(|row| row.keys).collect();
        for required in [
            "/",
            "?",
            "Enter",
            "Esc",
            "Esc Esc",
            "Ctrl+C",
            "PgUp / PgDn",
            "Tab",
            "Ctrl+Tab / Ctrl+Shift+Tab",
        ] {
            assert!(keys.contains(&required), "键位表缺 {required}");
        }
        for row in KEY_TABLE {
            assert!(!row.action.is_empty());
        }
    }

    /// 按键映射: 命令行 / 帮助 / 补全 / 翻页 / 页签 / 两段退出。
    #[test]
    fn maps_cockpit_keys_to_actions() {
        assert_eq!(
            map_key(
                press(KeyCode::Char('/'), KeyModifiers::NONE),
                KeyMode::Normal
            ),
            Action::OpenCommand
        );
        assert_eq!(
            map_key(
                press(KeyCode::Char('?'), KeyModifiers::NONE),
                KeyMode::Normal
            ),
            Action::ToggleHelp
        );
        assert_eq!(
            map_key(
                press(KeyCode::Char('/'), KeyModifiers::NONE),
                KeyMode::Input
            ),
            Action::Char('/')
        );
        assert_eq!(
            map_key(press(KeyCode::Tab, KeyModifiers::NONE), KeyMode::Input),
            Action::Complete
        );
        assert_eq!(
            map_key(press(KeyCode::Tab, KeyModifiers::CONTROL), KeyMode::Normal),
            Action::NextTab
        );
        assert_eq!(
            map_key(
                press(KeyCode::BackTab, KeyModifiers::SHIFT),
                KeyMode::Normal
            ),
            Action::PrevTab
        );
        assert_eq!(
            map_key(
                press(KeyCode::Char('c'), KeyModifiers::CONTROL),
                KeyMode::Normal
            ),
            Action::QuitStep
        );
        assert_eq!(
            map_key(press(KeyCode::Esc, KeyModifiers::NONE), KeyMode::Input),
            Action::Escape
        );
        assert_eq!(
            map_key(press(KeyCode::PageUp, KeyModifiers::NONE), KeyMode::Normal),
            Action::PageUp
        );
        assert_eq!(
            map_key(
                press(KeyCode::Char('h'), KeyModifiers::NONE),
                KeyMode::Input
            ),
            Action::Char('h')
        );
        assert_eq!(
            map_key(press(KeyCode::Left, KeyModifiers::NONE), KeyMode::Normal),
            Action::Ignored
        );
        assert_eq!(
            map_key(
                KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Release,
                    state: crossterm::event::KeyEventState::empty(),
                },
                KeyMode::Normal
            ),
            Action::Ignored
        );
    }

    /// 任务面板键位: 模态导航 + 人判重试 + Ctrl+T 全局; 面板内其余键不进输入行。
    #[test]
    fn panel_tasks_mode_maps_navigation_and_judge_retry_only() {
        let press_plain = |code| press(code, KeyModifiers::NONE);
        assert_eq!(
            map_key(press_plain(KeyCode::Up), KeyMode::PanelTasks),
            Action::SelectPrev
        );
        assert_eq!(
            map_key(press_plain(KeyCode::Char('k')), KeyMode::PanelTasks),
            Action::SelectPrev
        );
        assert_eq!(
            map_key(press_plain(KeyCode::Down), KeyMode::PanelTasks),
            Action::SelectNext
        );
        assert_eq!(
            map_key(press_plain(KeyCode::Char('j')), KeyMode::PanelTasks),
            Action::SelectNext
        );
        assert_eq!(
            map_key(press_plain(KeyCode::Char('r')), KeyMode::PanelTasks),
            Action::JudgeRetry
        );
        assert_eq!(
            map_key(press_plain(KeyCode::Esc), KeyMode::PanelTasks),
            Action::Escape
        );
        // 面板是模态: 普通字符/回车不进输入行 (面板无写口, 防越权面)。
        assert_eq!(
            map_key(press_plain(KeyCode::Char('t')), KeyMode::PanelTasks),
            Action::Ignored
        );
        assert_eq!(
            map_key(press_plain(KeyCode::Enter), KeyMode::PanelTasks),
            Action::Ignored
        );
        // Ctrl+T 全局 (正文 / 输入 / 面板都认)。
        for mode in [KeyMode::Normal, KeyMode::Input, KeyMode::PanelTasks] {
            assert_eq!(
                map_key(press(KeyCode::Char('t'), KeyModifiers::CONTROL), mode),
                Action::ToggleTasks
            );
        }
        // 键位表: 新键位入表 (帮助浮层即真键位)。
        let keys: Vec<&str> = KEY_TABLE.iter().map(|row| row.keys).collect();
        for required in ["Ctrl+T", "↑ / ↓", "r"] {
            assert!(keys.contains(&required), "键位表缺 {required}");
        }
    }
}
