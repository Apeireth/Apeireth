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
    /// 退格。
    Backspace,
    /// 命令补全 (Tab)。
    Complete,
    /// 可打印字符。
    Char(char),
    /// 忽略。
    Ignored,
}

/// 按键上下文: 光标在正文还是在输入/命令行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMode {
    /// 正文浏览态。
    Normal,
    /// 输入态 (命令行或普通输入)。
    Input,
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
    match key.code {
        KeyCode::Char('c') if ctrl => Action::QuitStep,
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
}
