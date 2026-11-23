//! Operations on a pinned directory. File names were checked by the caller.

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[allow(unsafe_code)]
mod platform {
    use std::ffi::CString;
    use std::fs::{File, OpenOptions};
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::path::Path;

    pub(in crate::project_files) struct Directory(File);

    fn name(value: &str) -> CString {
        CString::new(value).expect("validated file name")
    }

    fn status(value: libc::c_int) -> io::Result<()> {
        if value == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    impl Directory {
        pub(in crate::project_files) fn open(root: &Path) -> io::Result<Self> {
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
                .open(root)
                .map(Self)
        }

        fn open_at(&self, value: &str, flags: libc::c_int) -> io::Result<File> {
            let value = name(value);
            // SAFETY: The directory fd remains owned by self and value is NUL-terminated.
            let fd = unsafe {
                libc::openat(
                    self.0.as_raw_fd(),
                    value.as_ptr(),
                    flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    0o600,
                )
            };
            status(fd)?;
            // SAFETY: openat returned a new owned, valid fd.
            Ok(unsafe { File::from_raw_fd(fd) })
        }

        pub(in crate::project_files) fn open_file(&self, value: &str) -> io::Result<File> {
            let file = self.open_at(value, libc::O_RDONLY | libc::O_NONBLOCK)?;
            let meta = file.metadata()?;
            if !meta.is_file() || meta.nlink() != 1 {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            Ok(file)
        }

        pub(in crate::project_files) fn create_stage(&self, value: &str) -> io::Result<File> {
            self.open_at(value, libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL)
        }

        pub(in crate::project_files) fn publish(
            &self,
            stage: &str,
            target: &str,
            new_file: bool,
        ) -> io::Result<()> {
            let stage = name(stage);
            let target = name(target);
            let fd = self.0.as_raw_fd();
            // SAFETY: Both names are NUL-terminated, with the same live directory fd.
            let result = unsafe {
                if new_file {
                    rename_new(fd, &stage, &target)
                } else {
                    libc::renameat(fd, stage.as_ptr(), fd, target.as_ptr())
                }
            };
            status(result)?;
            self.0.sync_all()
        }

        pub(in crate::project_files) fn discard_stage(&self, stage: &str) {
            let stage = name(stage);
            // SAFETY: The live directory fd and owned NUL-terminated name are valid.
            let _ = unsafe { libc::unlinkat(self.0.as_raw_fd(), stage.as_ptr(), 0) };
        }
    }
    #[cfg(target_os = "linux")]
    unsafe fn rename_new(
        fd: libc::c_int,
        stage: &std::ffi::CStr,
        target: &std::ffi::CStr,
    ) -> libc::c_int {
        // SAFETY: Both strings and the shared directory fd are borrowed by the caller.
        unsafe {
            libc::syscall(
                libc::SYS_renameat2,
                fd,
                stage.as_ptr(),
                fd,
                target.as_ptr(),
                libc::RENAME_NOREPLACE,
            ) as libc::c_int
        }
    }

    #[cfg(target_os = "macos")]
    unsafe fn rename_new(
        fd: libc::c_int,
        stage: &std::ffi::CStr,
        target: &std::ffi::CStr,
    ) -> libc::c_int {
        // SAFETY: Both strings and the shared directory fd are borrowed by the caller.
        unsafe { libc::renameatx_np(fd, stage.as_ptr(), fd, target.as_ptr(), libc::RENAME_EXCL) }
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod platform {
    use std::ffi::OsString;
    use std::fs::{self, File, OpenOptions};
    use std::io;
    use std::mem::MaybeUninit;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::os::windows::io::AsRawHandle;
    use std::path::{Path, PathBuf};
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, GetFinalPathNameByHandleW, MoveFileExW,
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE, MOVEFILE_REPLACE_EXISTING,
        MOVEFILE_WRITE_THROUGH,
    };

    pub(in crate::project_files) struct Directory {
        path: PathBuf,
        _handle: File,
        _parent: File,
    }

    impl Directory {
        pub(in crate::project_files) fn open(root: &Path) -> io::Result<Self> {
            let parent = root.parent().ok_or(io::ErrorKind::InvalidInput)?;
            let parent = pin_dir(parent)?;
            let root_handle = pin_dir(root)?;
            Ok(Self {
                path: final_path(&root_handle)?,
                _handle: root_handle,
                _parent: parent,
            })
        }

        pub(in crate::project_files) fn open_file(&self, value: &str) -> io::Result<File> {
            let file = OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                .open(self.path.join(value))?;
            let meta = file.metadata()?;
            if !meta.is_file() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
            // SAFETY: The handle belongs to file and info points to writable storage.
            if unsafe { GetFileInformationByHandle(file.as_raw_handle().cast(), info.as_mut_ptr()) }
                == 0
            {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: The successful system call initialized the structure.
            if unsafe { info.assume_init() }.nNumberOfLinks != 1 {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            let actual = final_path(&file)?;
            // Short-name aliases can look harmless while naming protected data.
            if actual.parent() != Some(self.path.as_path())
                || !actual
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case(value))
            {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            Ok(file)
        }

        pub(in crate::project_files) fn create_stage(&self, value: &str) -> io::Result<File> {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.path.join(value))
        }

        pub(in crate::project_files) fn publish(
            &self,
            stage: &str,
            target: &str,
            new_file: bool,
        ) -> io::Result<()> {
            let wide = |name: &str| {
                self.path
                    .join(name)
                    .as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect::<Vec<_>>()
            };
            let stage = wide(stage);
            let target = wide(target);
            let flags = MOVEFILE_WRITE_THROUGH
                | if new_file {
                    0
                } else {
                    MOVEFILE_REPLACE_EXISTING
                };
            // SAFETY: Both NUL-terminated UTF-16 buffers outlive the call.
            if unsafe { MoveFileExW(stage.as_ptr(), target.as_ptr(), flags) } != 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }

        pub(in crate::project_files) fn discard_stage(&self, stage: &str) {
            let _ = fs::remove_file(self.path.join(stage));
        }
    }

    fn pin_dir(path: &Path) -> io::Result<File> {
        // Withhold FILE_SHARE_DELETE while using relative names beneath this directory.
        let handle = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = handle.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(handle)
    }

    fn final_path(handle: &File) -> io::Result<PathBuf> {
        let raw = handle.as_raw_handle().cast();
        // SAFETY: A zero length query does not write to the null buffer pointer.
        let size = unsafe { GetFinalPathNameByHandleW(raw, std::ptr::null_mut(), 0, 0) };
        if size == 0 || size > 32_768 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let mut path = vec![0u16; size as usize];
        // SAFETY: The output buffer owns size UTF-16 code units.
        let used = unsafe { GetFinalPathNameByHandleW(raw, path.as_mut_ptr(), size, 0) };
        if used == 0 || used >= size {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(PathBuf::from(OsString::from_wide(&path[..used as usize])))
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod platform {
    use std::fs::File;
    use std::{io, path::Path};

    pub(in crate::project_files) struct Directory;

    impl Directory {
        pub(in crate::project_files) fn open(_: &Path) -> io::Result<Self> {
            Err(io::ErrorKind::Unsupported.into())
        }
        pub(in crate::project_files) fn open_file(&self, _: &str) -> io::Result<File> {
            Err(io::ErrorKind::Unsupported.into())
        }
        pub(in crate::project_files) fn create_stage(&self, _: &str) -> io::Result<File> {
            Err(io::ErrorKind::Unsupported.into())
        }
        pub(in crate::project_files) fn publish(
            &self,
            _: &str,
            _: &str,
            _: bool,
        ) -> io::Result<()> {
            Err(io::ErrorKind::Unsupported.into())
        }
        pub(in crate::project_files) fn discard_stage(&self, _: &str) {}
    }
}

pub(super) use platform::Directory;
