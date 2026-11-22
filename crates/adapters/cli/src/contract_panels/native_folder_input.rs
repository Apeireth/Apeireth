//! Bootstrap proof delivered through the paired desktop child stdin pipe.

#![allow(unsafe_code)]

use std::ffi::c_void;
use std::io::{self, Read};
use std::os::windows::io::{AsRawHandle, RawHandle};
use std::time::{Duration, Instant};

use super::NativeFolderGrantToken;

const FILE_TYPE_PIPE: u32 = 3;
const LAUNCHER_FRAME: &[u8] = b"APEIRETH-FOLDER-GRANT-1\n";
const FRAME_BYTES: usize = LAUNCHER_FRAME.len() + 64;
const FRAME_WAIT: Duration = Duration::from_millis(750);

#[link(name = "kernel32")]
extern "system" {
    #[link_name = "GetFileType"]
    fn get_file_type(file: RawHandle) -> u32;
    #[link_name = "PeekNamedPipe"]
    fn peek_named_pipe(
        pipe: RawHandle,
        buffer: *mut c_void,
        buffer_len: u32,
        read: *mut u32,
        available: *mut u32,
        remaining: *mut u32,
    ) -> i32;
}

fn parse_frame(frame: &[u8]) -> Option<NativeFolderGrantToken> {
    let token = std::str::from_utf8(frame.strip_prefix(LAUNCHER_FRAME)?).ok()?;
    NativeFolderGrantToken::parse(token)
}

pub(super) fn read_token() -> Option<NativeFolderGrantToken> {
    let stdin = io::stdin();
    let handle = stdin.as_raw_handle();
    // SAFETY: The live process stdin handle is borrowed. This call does not close it.
    if unsafe { get_file_type(handle) } != FILE_TYPE_PIPE {
        return None;
    }
    // No read is attempted until all bytes are buffered in the inherited pipe.
    // The launcher may write immediately after the child starts.
    let mut input = stdin.lock();
    let deadline = Instant::now() + FRAME_WAIT;
    loop {
        let mut available = 0;
        // SAFETY: The handle is a pipe, and only the valid `available` pointer is writable.
        if unsafe {
            peek_named_pipe(
                handle,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut available,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return None;
        }
        if available as usize == FRAME_BYTES {
            let mut frame = [0; FRAME_BYTES];
            input.read_exact(&mut frame).ok()?;
            return parse_frame(&frame);
        }
        if available as usize > FRAME_BYTES || Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inherited_credential_accepts_only_the_exact_frame() {
        let credential = "ab".repeat(32);
        let frame = [LAUNCHER_FRAME, credential.as_bytes()].concat();
        let adopted = parse_frame(&frame).expect("complete launcher frame");
        assert!(adopted.matches(&credential));
        assert!(parse_frame(credential.as_bytes()).is_none());
        assert!(parse_frame(&frame[..frame.len() - 1]).is_none());
        assert!(parse_frame(&[frame.as_slice(), b"\n"].concat()).is_none());
    }
}
