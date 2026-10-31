//! 主窗标题栏配色：把「未选中态」那一档中性深灰钉死在前台态。
//!
//! Windows 11 在「设置 › 个性化 › 颜色 › 在标题栏和窗口边框上显示强调色」
//! (ColorPrevalence=1) 打开时，用**系统强调色**画**前台**窗口的标题栏。本机强调色
//! 是浅灰 #A1A1A1，于是主窗一获得焦点，深色界面顶上就横出一条浅灰带，与界面其余
//! 部分（内容区 #0A0A0B、侧栏 #202020）割裂；*未选中*时反而落回 Windows 深色主题
//! 的默认底 #202020，看上去自然。
//!
//! 这里用 DWM 的 caption / text / border 三个属性，把标题栏钉在未选中态那一档深灰：
//! 前台与后台同色，不再跟随强调色。标题栏属非客户区，Web 层的 CSS 改不到它，只能
//! 在 DWM 这一层改。
//!
//! # 系统版本差异
//!
//! * 深色沉浸模式 (DWMWA_USE_IMMERSIVE_DARK_MODE)：Windows 10 1809 (build 17763) 起。
//! * 自定义标题栏三色：Windows 11 (build 22000) 起才认。更早的系统上这三条调用返回
//!   E_INVALIDARG，本文一律忽略——沉浸模式已经把标题栏换成深色一套，不会回到
//!   浅灰强调色。

/// 把 RGB 三个通道编成 Windows 的 COLORREF：**0x00BBGGRR**（红在最低字节）。
///
/// 次序写反了标题栏会整体偏色，所以单测锁住这个编码。
pub const fn colorref(r: u8, g: u8, b: u8) -> u32 {
    ((b as u32) << 16) | ((g as u32) << 8) | (r as u32)
}

/// 一组标题栏配色（底色 / 文字 / 边框），前台与后台共用同一组。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionPlan {
    /// 底色 = 未选中态的 #202020（与界面侧栏同一档灰）。
    pub caption: u32,
    /// 标题文字 #C8C8C8：深底上的浅灰，比纯白柔和。
    pub text: u32,
    /// 边框 #2A2A2A：比底色亮一档，保住窗口轮廓。
    pub border: u32,
}

/// 固定的标题栏配色。数值即观感契约，改这里等于改界面。
pub const CAPTION_PLAN: CaptionPlan = CaptionPlan {
    caption: colorref(0x20, 0x20, 0x20),
    text: colorref(0xC8, 0xC8, 0xC8),
    border: colorref(0x2A, 0x2A, 0x2A),
};

/// 把固定配色套到窗口标题栏上；非 Windows 平台是空实现。
///
/// 取不到原生句柄（窗口还没实体化等）时只打一行日志并保留系统默认配色——配色是观感
/// 优化，不值得让启动失败。
#[cfg(windows)]
pub fn apply_dark_caption(window: &tauri::WebviewWindow) {
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{BOOL, HWND};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
    };

    // tauri 的 hwnd() 走 windows crate 的 HWND；这里是裸指针，直接转给 windows-sys。
    let hwnd: HWND = match window.hwnd() {
        Ok(hwnd) => hwnd.0 as HWND,
        Err(error) => {
            eprintln!("titlebar: no native handle, keeping system caption: {error}");
            return;
        }
    };

    unsafe {
        let dark: BOOL = 1;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &dark as *const BOOL as *const c_void,
            std::mem::size_of::<BOOL>() as u32,
        );

        for (attribute, color) in [
            (DWMWA_CAPTION_COLOR, CAPTION_PLAN.caption),
            (DWMWA_TEXT_COLOR, CAPTION_PLAN.text),
            (DWMWA_BORDER_COLOR, CAPTION_PLAN.border),
        ] {
            // 老系统不认这些属性（返回错误）：忽略即可，深色沉浸模式已经兜住底色。
            let _ = DwmSetWindowAttribute(
                hwnd,
                attribute as u32,
                &color as *const u32 as *const c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
}

/// 非 Windows 平台：标题栏由系统主题决定，这里无事可做。
#[cfg(not(windows))]
pub fn apply_dark_caption(_window: &tauri::WebviewWindow) {}

#[cfg(test)]
mod tests {
    use super::{colorref, CAPTION_PLAN};

    /// COLORREF 是 0x00BBGGRR：红在最低字节。写反了标题栏整体偏色。
    #[test]
    fn colorref_uses_bgr_byte_order() {
        assert_eq!(colorref(0xAA, 0xBB, 0xCC), 0x00CC_BBAA);
        assert_eq!(CAPTION_PLAN.caption, 0x0020_2020);
        assert_eq!(CAPTION_PLAN.text, 0x00C8_C8C8);
        assert_eq!(CAPTION_PLAN.border, 0x002A_2A2A);
    }

    /// 标题文字必须比底色亮得够多，否则深色标题栏上的标题读不清（WCAG ≥ 4.5:1）。
    #[test]
    fn caption_text_contrasts_with_caption_background() {
        // sRGB 单通道线性化（WCAG 2.1 相对亮度定义）。
        fn channel(color: u32, shift: u32) -> f64 {
            let value = ((color >> shift) & 0xFF) as f64 / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }
        fn luminance(color: u32) -> f64 {
            0.2126 * channel(color, 0) + 0.7152 * channel(color, 8) + 0.0722 * channel(color, 16)
        }

        let (caption, text) = (
            luminance(CAPTION_PLAN.caption),
            luminance(CAPTION_PLAN.text),
        );
        let (lighter, darker) = if caption > text {
            (caption, text)
        } else {
            (text, caption)
        };
        let ratio = (lighter + 0.05) / (darker + 0.05);
        assert!(ratio >= 4.5, "标题文字对比度 {ratio:.2}:1，低于 4.5:1");
    }
}
