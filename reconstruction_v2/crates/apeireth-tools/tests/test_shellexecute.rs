#[cfg(target_os = "windows")]
use winapi::um::shellapi::ShellExecuteW;
#[cfg(target_os = "windows")]
use winapi::um::winuser::SW_SHOWNORMAL;

#[test]
fn test_shell_execute_url() {
    #[cfg(target_os = "windows")]
    unsafe {
        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let url: Vec<u16> = "https://search.bilibili.com/all?keyword=live2d\0".encode_utf16().collect();
        let ret = ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        let ret_code = ret as usize;
        println!("ShellExecuteW return code: {} (Success if > 32)", ret_code);
        assert!(ret_code > 32, "ShellExecute failed with error code {}", ret_code);
    }
}
