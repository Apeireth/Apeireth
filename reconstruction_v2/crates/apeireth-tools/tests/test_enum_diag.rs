use winapi::shared::windef::{HWND, RECT};
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, LPARAM, TRUE};
use winapi::um::winuser::{
    CloseDesktop, CloseWindowStation, EnumDesktopWindows, EnumWindows, GetClassNameW, GetWindowRect,
    GetWindowTextW, IsWindowVisible, OpenDesktopW, OpenWindowStationW, SetProcessWindowStation,
    WINSTA_ALL_ACCESS,
};


fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn test_enum_proc(hwnd: HWND, _: LPARAM) -> BOOL {
    let is_vis = IsWindowVisible(hwnd);
    let mut rect: RECT = std::mem::zeroed();
    let _ = GetWindowRect(hwnd, &mut rect);
    let mut title_buf = [0u16; 256];
    let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 256);
    let title = if len > 0 {
        String::from_utf16_lossy(&title_buf[..len as usize])
    } else {
        String::new()
    };
    let mut class_buf = [0u16; 128];
    let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 128);
    let class_name = if class_len > 0 {
        String::from_utf16_lossy(&class_buf[..class_len as usize])
    } else {
        String::new()
    };

    if !title.trim().is_empty() && is_vis != 0 {
        println!("VISIBLE USER WINDOW -> HWND={:?}, Rect=({},{},{},{}), Title=\"{}\", Class=\"{}\"",
            hwnd, rect.left, rect.top, rect.right, rect.bottom, title, class_name);
    }
    TRUE
}

#[test]
#[ignore = "diagnostic: opens WinSta0 / Default desktop handle"]
fn test_enum_winsta0() {
    unsafe {
        let winsta_name = to_wide("WinSta0");
        let winsta = OpenWindowStationW(winsta_name.as_ptr(), FALSE, WINSTA_ALL_ACCESS);
        println!("OpenWindowStation(WinSta0) = {:?}", winsta);
        if !winsta.is_null() {
            SetProcessWindowStation(winsta);
            let desk_name = to_wide("Default");
            let desk = OpenDesktopW(desk_name.as_ptr(), 0, FALSE, 0x01FF);
            println!("OpenDesktop(Default) = {:?}", desk);

            if !desk.is_null() {
                EnumDesktopWindows(desk, Some(test_enum_proc), 0);
                CloseDesktop(desk);
            }
            CloseWindowStation(winsta);
        } else {
            EnumWindows(Some(test_enum_proc), 0);
        }
    }
}
