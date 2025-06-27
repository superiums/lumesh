use crate::RuntimeErrorKind;
// use std::sync::{Arc, atomic::AtomicBool};

// 定义跨平台终端操作 trait
pub trait TerminalOps {
    fn enable_raw_mode(&self) -> Result<(), RuntimeErrorKind>;
    fn disable_raw_mode(&self) -> Result<(), RuntimeErrorKind>;
    #[cfg(unix)]
    fn setup_signal_handlers(&self) -> Result<(), RuntimeErrorKind>;
    #[cfg(windows)]
    fn handle_ctrl_c(&self, running: Arc<AtomicBool>) -> Result<(), RuntimeErrorKind>;
    fn get_terminal_size(&self) -> (u16, u16);
}

// Unix 平台实现
#[cfg(unix)]
pub struct UnixTerminal;

#[cfg(unix)]
impl TerminalOps for UnixTerminal {
    fn enable_raw_mode(&self) -> Result<(), RuntimeErrorKind> {
        crossterm::terminal::enable_raw_mode()
            .map_err(|e| RuntimeErrorKind::CustomError(e.to_string().into()))
    }

    fn disable_raw_mode(&self) -> Result<(), RuntimeErrorKind> {
        crossterm::terminal::disable_raw_mode()
            .map_err(|e| RuntimeErrorKind::CustomError(e.to_string().into()))
    }

    fn setup_signal_handlers(&self) -> Result<(), RuntimeErrorKind> {
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
            .map_err(|e| RuntimeErrorKind::CustomError(e.to_string().into()))?;

            signal::sigaction(
                signal::Signal::SIGINT,
                &signal::SigAction::new(
                    signal::SigHandler::SigDfl,
                    signal::SaFlags::SA_RESTART,
                    signal::SigSet::empty(),
                ),
            )
            .map_err(|e| RuntimeErrorKind::CustomError(e.to_string().into()))?;
        }
        Ok(())
    }

    // fn handle_ctrl_c(&self, _running: Arc<AtomicBool>) -> Result<(), RuntimeErrorKind> {
    //     // Unix 上已经通过信号处理
    //     Ok(())
    // }

    fn get_terminal_size(&self) -> (u16, u16) {
        crossterm::terminal::size().unwrap_or((24, 80))

        // use terminal_size::{Height, Width, terminal_size};
        // match terminal_size() {
        //     Some((Width(w), Height(h))) => (w, h),
        //     _ => (24, 80),
        // }
    }
}

// Windows 平台实现
#[cfg(windows)]
pub struct WindowsTerminal;

#[cfg(windows)]
use std::io::{stdin, stdout};
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::{Arc, Mutex};
#[cfg(windows)]
use winapi::shared::minwindef::BOOL;
#[cfg(windows)]
use winapi::um::consoleapi::{GetConsoleMode, SetConsoleCtrlHandler, SetConsoleMode};
#[cfg(windows)]
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
#[cfg(windows)]
use winapi::um::wincon::{
    CONSOLE_SCREEN_BUFFER_INFO, CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT,
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, GetConsoleScreenBufferInfo,
};

#[cfg(windows)]
impl TerminalOps for WindowsTerminal {
    fn enable_raw_mode(&self) -> Result<(), RuntimeErrorKind> {
        unsafe {
            let handle = stdin().as_raw_handle();
            if handle == INVALID_HANDLE_VALUE {
                return Err(RuntimeErrorKind::CustomError(
                    "Failed to get stdin handle".to_string(),
                ));
            }

            let mut mode: u32 = 0;
            if GetConsoleMode(handle, &mut mode) == 0 {
                return Err(RuntimeErrorKind::CustomError(
                    "Failed to get console mode".to_string(),
                ));
            }

            mode &= !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT);

            if SetConsoleMode(handle, mode) == 0 {
                return Err(RuntimeErrorKind::CustomError(
                    "Failed to set raw mode".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn disable_raw_mode(&self) -> Result<(), RuntimeErrorKind> {
        unsafe {
            let handle = stdin().as_raw_handle();
            if handle == INVALID_HANDLE_VALUE {
                return Err(RuntimeErrorKind::CustomError(
                    "Failed to get stdin handle".to_string(),
                ));
            }

            let mut mode: u32 = 0;
            if GetConsoleMode(handle, &mut mode) == 0 {
                return Err(RuntimeErrorKind::CustomError(
                    "Failed to get console mode".to_string(),
                ));
            }

            mode |= ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT;

            if SetConsoleMode(handle, mode) == 0 {
                return Err(RuntimeErrorKind::CustomError(
                    "Failed to restore console mode".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn handle_ctrl_c(&self, running: Arc<AtomicBool>) -> Result<(), RuntimeErrorKind> {
        lazy_static::lazy_static! {
            static ref RUNNING_FLAG: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
        }

        *RUNNING_FLAG.lock().unwrap() = Some(running);

        unsafe extern "system" fn handler(ctrl_type: u32) -> BOOL {
            match ctrl_type {
                CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT => {
                    if let Some(flag) = RUNNING_FLAG.lock().unwrap().as_ref() {
                        flag.store(false, Ordering::SeqCst);
                    }
                    1 // TRUE - signal handled
                }
                _ => 0, // FALSE - pass to next handler
            }
        }

        unsafe {
            if SetConsoleCtrlHandler(Some(handler), 1) == 0 {
                return Err(RuntimeErrorKind::CustomError(
                    "Failed to set Ctrl+C handler".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn get_terminal_size(&self) -> (u16, u16) {
        unsafe {
            let handle = stdout().as_raw_handle();
            if handle == INVALID_HANDLE_VALUE {
                return (80, 24); // Default size
            }

            let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
            if GetConsoleScreenBufferInfo(handle, &mut csbi) == 0 {
                return (80, 24); // Default size
            }

            let width = csbi.srWindow.Right - csbi.srWindow.Left + 1;
            let height = csbi.srWindow.Bottom - csbi.srWindow.Top + 1;

            (width as u16, height as u16)
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
