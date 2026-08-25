//! Test-only child helper for proving real OS process enforcement.
//!
//! This is **not** a product tool. It parses no user input as a shell command;
//! it exposes fixed, deterministic modes used by the process integration tests:
//!
//! ```text
//! sandbox-test-child sleep <seconds>
//! sandbox-test-child print stdout|stderr <bytes>
//! sandbox-test-child print-both <stdout-bytes> <stderr-bytes>
//! sandbox-test-child echo-args [args...]
//! sandbox-test-child print-cwd
//! sandbox-test-child print-env <name>
//! sandbox-test-child print-pid
//! sandbox-test-child platform-security-info
//! sandbox-test-child token-info            # alias for platform-security-info
//! sandbox-test-child print-no-new-privs
//! sandbox-test-child write-file <path> [content]
//! sandbox-test-child exit-code <n>
//! sandbox-test-child allocate <mebibytes>
//! sandbox-test-child spawn-child sleep <seconds>
//! ```

use std::io::{self, Write};
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(mode) = args.first().map(String::as_str) else {
        eprintln!("missing mode");
        std::process::exit(2);
    };

    match mode {
        "sleep" => {
            let seconds = parse::<u64>(args.get(1)).unwrap_or(1);
            thread::sleep(Duration::from_secs(seconds));
        }
        "print" => {
            let stream = args.get(1).map(String::as_str).unwrap_or("stdout");
            let bytes = parse::<usize>(args.get(2)).unwrap_or(0);
            let bytes = bytes.min(10_000_000);
            let mut buffer = vec![b'x'; bytes];
            // Make the last byte a newline so captured output is easy to assert
            // even when truncated.
            if !buffer.is_empty() {
                buffer[bytes - 1] = b'\n';
            }
            match stream {
                "stdout" => write_stdout(&buffer),
                "stderr" => write_stderr(&buffer),
                other => {
                    eprintln!("unknown stream: {other}");
                    std::process::exit(2);
                }
            }
        }
        "print-both" => {
            let stdout_bytes = parse::<usize>(args.get(1)).unwrap_or(0).min(10_000_000);
            let stderr_bytes = parse::<usize>(args.get(2)).unwrap_or(0).min(10_000_000);
            let stdout_buffer = vec![b'x'; stdout_bytes];
            let stderr_buffer = vec![b'y'; stderr_bytes];
            write_stdout(&stdout_buffer);
            write_stderr(&stderr_buffer);
        }
        "echo-args" => {
            for arg in args.iter().skip(1) {
                println!("ARG:{arg}");
            }
        }
        "print-cwd" => match std::env::current_dir() {
            Ok(cwd) => println!("CWD:{}", cwd.display()),
            Err(e) => {
                eprintln!("cwd error: {e}");
                std::process::exit(2);
            }
        },
        "print-env" => {
            let name = args.get(1).map(String::as_str).unwrap_or("");
            match std::env::var(name) {
                Ok(value) => println!("ENV:{value}"),
                Err(std::env::VarError::NotPresent) => println!("ENV_MISSING"),
                Err(std::env::VarError::NotUnicode(_)) => println!("ENV_NOT_UNICODE"),
            }
        }
        "print-pid" => {
            println!("PID:{}", std::process::id());
        }
        "platform-security-info" | "token-info" => {
            print_platform_security_info();
        }
        "print-no-new-privs" => {
            #[cfg(target_os = "linux")]
            {
                match std::fs::read_to_string("/proc/self/status") {
                    Ok(status) => {
                        for line in status.lines() {
                            if let Some(value) = line.strip_prefix("NoNewPrivs:") {
                                println!("NO_NEW_PRIVS:{}", value.trim());
                                return;
                            }
                        }
                        println!("NO_NEW_PRIVS_MISSING");
                    }
                    Err(e) => {
                        eprintln!("status read error: {e}");
                        std::process::exit(2);
                    }
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                println!("NOT_LINUX");
            }
        }
        "write-file" => {
            let Some(path) = args.get(1) else {
                eprintln!("missing path");
                std::process::exit(2);
            };
            let content = args.get(2).map(String::as_str).unwrap_or("MARK");
            match std::fs::write(path, content) {
                Ok(()) => println!("WRITE_OK"),
                Err(e) => {
                    eprintln!("write error: {e}");
                    std::process::exit(2);
                }
            }
        }
        "exit-code" => {
            let code = parse::<i32>(args.get(1)).unwrap_or(0);
            std::process::exit(code);
        }
        "allocate" => {
            let mebibytes = parse::<usize>(args.get(1)).unwrap_or(0);
            let bytes = mebibytes.saturating_mul(1024 * 1024);
            let mut vec: Vec<u8> = Vec::new();
            match vec.try_reserve_exact(bytes) {
                Ok(()) => {
                    // Touch the reservation only if it succeeded, then release.
                    println!("ALLOC_OK");
                }
                Err(_) => {
                    println!("ALLOC_FAIL");
                }
            }
        }
        "check-job" => {
            #[cfg(windows)]
            {
                use windows_sys::Win32::Foundation::BOOL;
                use windows_sys::Win32::System::JobObjects::IsProcessInJob;
                use windows_sys::Win32::System::Threading::GetCurrentProcess;

                let mut in_job: BOOL = 0;
                let ret = unsafe {
                    IsProcessInJob(GetCurrentProcess(), std::ptr::null_mut(), &mut in_job)
                };
                if ret != 0 && in_job != 0 {
                    println!("IN_JOB");
                } else {
                    println!("NOT_IN_JOB");
                }
            }
            #[cfg(not(windows))]
            {
                println!("NOT_WINDOWS");
            }
        }
        "spawn-child" => {
            let child_mode = args.get(1).map(String::as_str).unwrap_or("sleep");
            let child_arg = args.get(2).map(String::as_str).unwrap_or("1");
            let exe = match std::env::current_exe() {
                Ok(exe) => exe,
                Err(e) => {
                    eprintln!("current_exe error: {e}");
                    std::process::exit(2);
                }
            };

            let spawn_result = Command::new(&exe)
                .arg(child_mode)
                .arg(child_arg)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();

            match spawn_result {
                Ok(mut child) => {
                    println!("SPAWN_OK {}", child.id());
                    let status = child.wait();
                    match status {
                        Ok(status) => println!("CHILD_EXIT {:?}", status.code()),
                        Err(e) => println!("CHILD_WAIT_ERR {e}"),
                    }
                }
                Err(e) => {
                    println!("SPAWN_FAIL {e}");
                }
            }
        }
        other => {
            eprintln!("unknown mode: {other}");
            std::process::exit(2);
        }
    }
}

fn print_platform_security_info() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::BOOL;
        use windows_sys::Win32::Security::IsTokenRestricted;
        use windows_sys::Win32::System::JobObjects::IsProcessInJob;
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        let mut in_job: BOOL = 0;
        let ret = unsafe { IsProcessInJob(GetCurrentProcess(), std::ptr::null_mut(), &mut in_job) };
        if ret != 0 && in_job != 0 {
            println!("IN_JOB");
        } else {
            println!("NOT_IN_JOB");
        }

        let restricted = unsafe { IsTokenRestricted(GetCurrentProcess()) };
        if restricted != 0 {
            println!("TOKEN_RESTRICTED");
        } else {
            println!("TOKEN_NOT_RESTRICTED");
        }
    }

    #[cfg(target_os = "linux")]
    {
        println!("PGRP:{}", unsafe { libc::getpgrp() });
        println!("PID:{}", std::process::id());
        match std::fs::read_to_string("/proc/self/status") {
            Ok(status) => {
                for line in status.lines() {
                    if let Some(value) = line.strip_prefix("NoNewPrivs:") {
                        println!("NO_NEW_PRIVS:{}", value.trim());
                        return;
                    }
                }
                println!("NO_NEW_PRIVS_MISSING");
            }
            Err(_) => println!("NO_NEW_PRIVS_UNAVAILABLE"),
        }
    }

    #[cfg(target_os = "macos")]
    {
        println!("PGRP:{}", unsafe { libc::getpgrp() });
        println!("PID:{}", std::process::id());
        println!("NO_NEW_PRIVS_UNSUPPORTED");
    }

    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        println!("PLATFORM_OTHER");
    }
}

fn write_stdout(buffer: &[u8]) {
    if let Err(e) = io::stdout()
        .write_all(buffer)
        .and_then(|()| io::stdout().flush())
    {
        if e.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("stdout write error: {e}");
            std::process::exit(2);
        }
    }
}

fn write_stderr(buffer: &[u8]) {
    if let Err(e) = io::stderr()
        .write_all(buffer)
        .and_then(|()| io::stderr().flush())
    {
        if e.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("stderr write error: {e}");
            std::process::exit(2);
        }
    }
}

fn parse<T: std::str::FromStr>(value: Option<&String>) -> Option<T> {
    value.and_then(|v| v.parse::<T>().ok())
}
