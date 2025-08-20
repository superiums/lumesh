use std::sync::{Arc, LazyLock, Mutex};

static CURRENT_CHILD_PID: LazyLock<Arc<Mutex<Option<u32>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// 设置当前运行的子进程ID
pub fn set_child(pid: u32) {
    if let Ok(mut current_pid) = CURRENT_CHILD_PID.lock() {
        *current_pid = Some(pid);
    }
}

/// 终止当前运行的子进程
pub fn kill_child() -> bool {
    if let Ok(current_pid) = CURRENT_CHILD_PID.lock() {
        if let Some(pid) = *current_pid {
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM) == 0
            }
            #[cfg(windows)]
            {
                // Windows实现需要使用WinAPI
                use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
                use winapi::um::winnt::PROCESS_TERMINATE;
                unsafe {
                    let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
                    if !handle.is_null() {
                        let result = TerminateProcess(handle, 1);
                        winapi::um::handleapi::CloseHandle(handle);
                        result != 0
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        }
    } else {
        false
    }
}

/// 清除当前子进程ID
pub fn clear_child() {
    if let Ok(mut current_pid) = CURRENT_CHILD_PID.lock() {
        *current_pid = None;
    }
}

// /// 检查是否有活跃的子进程
// pub fn has_active_child() -> bool {
//     if let Ok(current_pid) = CURRENT_CHILD_PID.lock() {
//         current_pid.is_some()
//     } else {
//         false
//     }
// }
