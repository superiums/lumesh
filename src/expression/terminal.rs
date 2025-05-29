use crate::RuntimeError;
// use std::sync::{Arc, atomic::AtomicBool};

// 定义跨平台终端操作 trait
pub trait TerminalOps {
    fn enable_raw_mode(&self) -> Result<(), RuntimeError>;
    fn disable_raw_mode(&self) -> Result<(), RuntimeError>;
    fn setup_signal_handlers(&self) -> Result<(), RuntimeError>;
    #[cfg(windows)]
    fn handle_ctrl_c(&self, running: Arc<AtomicBool>) -> Result<(), RuntimeError>;
    fn get_terminal_size(&self) -> Result<(u16, u16), RuntimeError>;
}

// Unix 平台实现
#[cfg(unix)]
pub struct UnixTerminal;

#[cfg(unix)]
impl TerminalOps for UnixTerminal {
    fn enable_raw_mode(&self) -> Result<(), RuntimeError> {
        crossterm::terminal::enable_raw_mode().map_err(|e| RuntimeError::CustomError(e.to_string()))
    }

    fn disable_raw_mode(&self) -> Result<(), RuntimeError> {
        crossterm::terminal::disable_raw_mode()
            .map_err(|e| RuntimeError::CustomError(e.to_string()))
    }

    fn setup_signal_handlers(&self) -> Result<(), RuntimeError> {
        use nix::sys::signal;

        unsafe {
            signal::sigaction(
                signal::Signal::SIGTERM,
                &signal::SigAction::new(
                    signal::SigHandler::SigDfl,
                    signal::SaFlags::SA_RESTART,
                    signal::SigSet::empty(),
                ),
            )
            .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

            signal::sigaction(
                signal::Signal::SIGINT,
                &signal::SigAction::new(
                    signal::SigHandler::SigDfl,
                    signal::SaFlags::SA_RESTART,
                    signal::SigSet::empty(),
                ),
            )
            .map_err(|e| RuntimeError::CustomError(e.to_string()))?;
        }
        Ok(())
    }

    // fn handle_ctrl_c(&self, _running: Arc<AtomicBool>) -> Result<(), RuntimeError> {
    //     // Unix 上已经通过信号处理
    //     Ok(())
    // }

    fn get_terminal_size(&self) -> Result<(u16, u16), RuntimeError> {
        use terminal_size::{Height, Width, terminal_size};

        match terminal_size() {
            Some((Width(w), Height(h))) => Ok((w, h)),
            _ => Ok((24, 80)),
        }
    }
}

// Windows 平台实现
#[cfg(windows)]
pub struct WindowsTerminal;

#[cfg(windows)]
impl TerminalOps for WindowsTerminal {
    fn enable_raw_mode(&self) -> Result<(), RuntimeError> {
        use std::io::stdin;
        use std::os::windows::io::AsRawHandle;
        use winapi::um::wincon::{
            ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, GetConsoleMode,
            SetConsoleMode,
        };

        unsafe {
            let stdin_handle = stdin().as_raw_handle();
            let mut mode: u32 = 0;

            if GetConsoleMode(stdin_handle, &mut mode) == 0 {
                return Err(RuntimeError::CustomError(
                    "Failed to get console mode".to_string(),
                ));
            }

            mode &= !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT);

            if SetConsoleMode(stdin_handle, mode) == 0 {
                return Err(RuntimeError::CustomError(
                    "Failed to set console mode".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn disable_raw_mode(&self) -> Result<(), RuntimeError> {
        use std::io::stdin;
        use std::os::windows::io::AsRawHandle;
        use winapi::um::wincon::{
            ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, GetConsoleMode,
            SetConsoleMode,
        };

        unsafe {
            let stdin_handle = stdin().as_raw_handle();
            let mut mode: u32 = 0;

            if GetConsoleMode(stdin_handle, &mut mode) == 0 {
                return Err(RuntimeError::CustomError(
                    "Failed to get console mode".to_string(),
                ));
            }

            mode |= ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT;

            if SetConsoleMode(stdin_handle, mode) == 0 {
                return Err(RuntimeError::CustomError(
                    "Failed to set console mode".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn setup_signal_handlers(&self) -> Result<(), RuntimeError> {
        // Windows 不需要 Unix 风格的信号处理
        Ok(())
    }

    fn handle_ctrl_c(&self, running: Arc<AtomicBool>) -> Result<(), RuntimeError> {
        use std::ptr;
        use std::sync::Mutex;
        use std::sync::atomic::Ordering;
        use winapi::um::wincon::{CTRL_C_EVENT, SetConsoleCtrlHandler};

        // 我们需要将 running 标志保存在全局可访问的地方
        // 由于原始指针需要，我们使用 Mutex 来安全地管理它
        lazy_static::lazy_static! {
            static ref RUNNING_FLAG: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
        }

        // 将运行标志存储在全局变量中
        *RUNNING_FLAG.lock().unwrap() = Some(running);

        unsafe extern "system" fn ctrl_handler(_: u32) -> i32 {
            use winapi::um::wincon::{CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT};

            match ctrl_type {
                CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT => {
                    // 获取全局的运行标志
                    if let Some(running_flag) = RUNNING_FLAG.lock().unwrap().as_ref() {
                        // 设置应该退出的标志
                        running_flag.store(true, Ordering::SeqCst);
                        // SHOULD_EXIT.store(true, Ordering::SeqCst);
                    }

                    // 返回1表示我们已经处理了这个信号
                    1
                }
                _ => {
                    // 对于其他控制事件，返回0表示我们不处理
                    0
                }
            }
        }

        unsafe {
            if SetConsoleCtrlHandler(Some(ctrl_handler), 1) == 0 {
                return Err(RuntimeError::CustomError(
                    "Failed to set Ctrl+C handler".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn get_terminal_size(&self) -> Result<(u16, u16), RuntimeError> {
        use std::io::stdout;
        use std::os::windows::io::AsRawHandle;
        use winapi::um::wincon::{CONSOLE_SCREEN_BUFFER_INFO, GetConsoleScreenBufferInfo};

        unsafe {
            let stdout_handle = stdout().as_raw_handle();
            let mut console_info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();

            if GetConsoleScreenBufferInfo(stdout_handle, &mut console_info) == 0 {
                return Ok((80, 24)); // 默认值
            }

            let width = console_info.srWindow.Right - console_info.srWindow.Left + 1;
            let height = console_info.srWindow.Bottom - console_info.srWindow.Top + 1;

            Ok((width as u16, height as u16))
        }
    }
}

// 根据平台选择终端实现
pub fn get_terminal_impl() -> Box<dyn TerminalOps> {
    #[cfg(unix)]
    return Box::new(UnixTerminal);

    #[cfg(windows)]
    return Box::new(WindowsTerminal);
}
