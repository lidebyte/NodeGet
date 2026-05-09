#[cfg(target_os = "windows")]
pub fn count_processes() -> u32 {
    use windows_sys::Win32::Foundation::FALSE;
    use windows_sys::Win32::System::ProcessStatus::EnumProcesses;

    // 初始缓冲区容量
    let mut cap: usize = 1024;

    loop {
        let mut bytes_returned: u32 = 0;
        let mut process_ids: Vec<u32> = vec![0; cap];

        let buf_bytes_u32: u32 = match u32::try_from(cap.saturating_mul(size_of::<u32>())) {
            Ok(v) => v,
            Err(_) => return 0, // cap 太大溢出 u32，放弃
        };

        let ok = unsafe {
            EnumProcesses(
                process_ids.as_mut_ptr(),
                buf_bytes_u32,
                &raw mut bytes_returned,
            )
        };

        if ok == FALSE {
            // EnumProcesses 在 buffer 小于实际需要时并**不会**返回
            // ERROR_INSUFFICIENT_BUFFER：它会成功地返回 `bytes_returned == buf_bytes_u32`，
            // 提示可能截断。因此这里的失败分支只处理真正的 API 错误，不再扩容。
            return 0;
        }

        // 当 bytes_returned 等于提供的 buffer 大小时，结果可能被截断，
        // 按 MSDN 文档加倍重试直到不再饱和。
        if bytes_returned == buf_bytes_u32 {
            cap = cap.saturating_mul(2);
            continue;
        }

        return bytes_returned / size_of::<u32>() as u32;
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
