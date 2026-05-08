#[cfg(target_os = "windows")]
pub fn count_processes() -> u32 {
    use windows_sys::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
    use windows_sys::Win32::Foundation::{FALSE, GetLastError};
    use windows_sys::Win32::System::ProcessStatus::EnumProcesses;
    // 初始缓冲区容量
    let mut mut_process_ids_capacity = 1024;

    loop {
        let mut bytes_returned: u32 = 0;
        let mut process_ids: Vec<u32> = Vec::with_capacity(mut_process_ids_capacity);

        let result = unsafe {
            EnumProcesses(
                process_ids.as_mut_ptr().cast(),
                (mut_process_ids_capacity * size_of::<u32>()) as u32,
                &raw mut bytes_returned,
            )
        };

        if result == FALSE {
            let error_code = unsafe { GetLastError() };

            if error_code == ERROR_INSUFFICIENT_BUFFER {
                mut_process_ids_capacity += 1024; // 扩充缓冲区容量
                continue;
            }
            return 0; // 其他错误
        }

        let num_processes = bytes_returned / size_of::<u32>() as u32;
        return num_processes;
    }
}

#[cfg(target_os = "linux")]
pub fn count_processes() -> u32 {
    use std::fs;

    // Count entries in /proc whose name is a numeric PID, excluding kernel
    // threads. `/proc/<pid>/cmdline` is always an empty file for kernel
    // threads (kthreadd / ksoftirqd / ...) and non-empty for real user-space
    // processes. This is the same heuristic `ps` / `procps` use and is
    // cheaper than parsing `/proc/<pid>/status`.
    let Ok(entries) = fs::read_dir("/proc") else {
        return 0;
    };

    let mut count: u32 = 0;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if name.parse::<u32>().is_err() {
            continue;
        }

        // Reading cmdline races with process exit; a missing file or
        // read error just means "not a running user process right now".
        match fs::read(format!("/proc/{name}/cmdline")) {
            Ok(bytes) if !bytes.is_empty() => count += 1,
            _ => {}
        }
    }
    count
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub const fn count_processes() -> u32 {
    0 // TODO: MacOS Support
}
