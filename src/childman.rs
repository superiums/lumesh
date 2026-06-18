use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

static CURRENT_CHILD_PID: LazyLock<Arc<Mutex<Option<u32>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

/// 当前子进程 PID（原子变量，用于信号处理器中读取）
static CHILD_PID_FOR_SIGNAL: AtomicI32 = AtomicI32::new(-1);

/// 是否收到过 SIGINT 信号（全局标志，由 signal_handler 设置）
static SIGINT_RECEIVED: AtomicBool = AtomicBool::new(false);

/// 设置当前运行的子进程ID
pub fn set_child(pid: u32) {
    if let Ok(mut current_pid) = CURRENT_CHILD_PID.lock() {
        *current_pid = Some(pid);
    }
    CHILD_PID_FOR_SIGNAL.store(pid as i32, Ordering::SeqCst);
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
    CHILD_PID_FOR_SIGNAL.store(-1, Ordering::SeqCst);
}

/// 安装 SIGINT 处理器：
/// - 设置标志位（供 REPL 检测）
/// - 如果当前有活跃的子进程，发送 SIGTERM 终止它（确保 sleep/cat/gui 程序等都能退出）
pub fn install_sigint_handler() {
    #[cfg(unix)]
    {
        use nix::sys::signal::{self, SigAction, SigHandler, SaFlags, SigSet};
        // 使用非局部函数，避免 extern "C" fn 嵌套闭包
        extern "C" fn handle_sigint(_: i32) {
            SIGINT_RECEIVED.store(true, Ordering::SeqCst);
            let pid = CHILD_PID_FOR_SIGNAL.load(Ordering::SeqCst);
            if pid > 0 {
                unsafe {
                    libc::kill(pid, libc::SIGTERM);
                }
            }
        }
        let action = SigAction::new(
            SigHandler::Handler(handle_sigint),
            SaFlags::SA_RESTART,
            SigSet::empty(),
        );
        unsafe {
            let _ = signal::sigaction(signal::Signal::SIGINT, &action);
        }
    }
    #[cfg(windows)]
    {
        extern "system" fn handle_sigint(_: u32) -> i32 {
            SIGINT_RECEIVED.store(true, Ordering::SeqCst);
            let pid = CHILD_PID_FOR_SIGNAL.load(Ordering::SeqCst);
            if pid > 0 {
                unsafe {
                    // 在 Windows 上使用 TerminateProcess
                    use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
                    use winapi::um::winnt::PROCESS_TERMINATE;
                    let handle = OpenProcess(PROCESS_TERMINATE, 0, pid as u32);
                    if !handle.is_null() {
                        TerminateProcess(handle, 1);
                        winapi::um::handleapi::CloseHandle(handle);
                    }
                }
            }
            1
        }
        unsafe {
            winapi::um::consoleapi::SetConsoleCtrlHandler(Some(handle_sigint), 1);
        }
    }
}

/// 检查是否收到过 SIGINT，并清除标志
pub fn check_and_clear_sigint() -> bool {
    SIGINT_RECEIVED.swap(false, Ordering::SeqCst)
}
