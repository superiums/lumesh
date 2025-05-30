use super::terminal::{TerminalOps, get_terminal_impl};
use crate::{Environment, RuntimeError};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

// 使用 RAII 守卫确保终端模式恢复
struct TerminalGuard {
    terminal: Box<dyn TerminalOps>,
}

impl TerminalGuard {
    fn new(terminal: Box<dyn TerminalOps>) -> Result<Self, RuntimeError> {
        terminal.enable_raw_mode()?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.terminal.disable_raw_mode();
    }
}

pub fn exec_in_pty(
    cmdstr: &String,
    args: Option<Vec<String>>,
    env: &mut Environment,
    input: Option<Vec<u8>>,
) -> Result<Option<Vec<u8>>, RuntimeError> {
    let terminal = get_terminal_impl();

    // 设置信号处理
    #[cfg(unix)]
    terminal.setup_signal_handlers()?;
    let (w, h) = terminal.get_terminal_size();

    // 输入处理线程
    let running = Arc::new(AtomicBool::new(false));
    let running_clone = Arc::clone(&running);
    let running_clone2 = Arc::clone(&running);
    // 设置 Ctrl+C 处理
    #[cfg(windows)]
    terminal.handle_ctrl_c(Arc::clone(&running))?;
    // Guard
    let _terminal_guard = TerminalGuard::new(terminal)?;

    let is_vi = cmdstr == "vi";
    let is_shell = ["bash", "sh", "fish", "zsh"].contains(&cmdstr.as_str());
    // pty
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: h,
            cols: w,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

    // Unix 特定的终端设置
    #[cfg(unix)]
    {
        if let Some(master_fd) = pair.master.as_raw_fd() {
            unsafe {
                let mut termios = std::mem::zeroed();
                if libc::tcgetattr(master_fd, &mut termios) == 0 {
                    // 配置终端属性...
                    // 输入控制
                    // termios.c_cc[libc::VEOF] = 4; // Ctrl+D
                    // termios.c_cc[libc::VEOL] = libc::_POSIX_VDISABLE; // 无 EOL
                    // termios.c_cc[libc::VEOL2] = libc::_POSIX_VDISABLE;
                    // termios.c_cc[libc::VERASE] = 0x7f; // ASCII DEL (Backspace)
                    // termios.c_cc[libc::VWERASE] = 0x17; // Ctrl+W
                    // termios.c_cc[libc::VKILL] = 0x15; // Ctrl+U
                    // termios.c_cc[libc::VREPRINT] = 0x12; // Ctrl+R
                    // termios.c_cc[libc::VINTR] = 0x03; // Ctrl+C
                    // termios.c_cc[libc::VQUIT] = 0x1c; // Ctrl+\
                    // termios.c_cc[libc::VSUSP] = 0x1a; // Ctrl+Z
                    // termios.c_cc[libc::VSTART] = 0x11; // Ctrl+Q
                    // termios.c_cc[libc::VSTOP] = 0x13; // Ctrl+S
                    // termios.c_cc[libc::VLNEXT] = 0x16; // Ctrl+V
                    // termios.c_cc[libc::VDISCARD] = 0x0f; // Ctrl+O
                    // termios.c_cc[libc::VMIN] = 1;
                    // termios.c_cc[libc::VTIME] = 0;

                    termios.c_lflag |= libc::ECHO | libc::ICANON;
                    termios.c_lflag |= libc::ISIG;
                    // termios.c_lflag &= !libc::ISIG;
                    termios.c_oflag |= libc::OPOST;
                    if is_vi {
                        termios.c_iflag &= !libc::IXON; // 禁用流控制
                    }
                    libc::tcsetattr(master_fd, libc::TCSANOW, &termios);
                }
            }
        }
    }

    let mut cmd = CommandBuilder::new(cmdstr);
    if let Some(ag) = args {
        cmd.args(ag);
    }

    for (k, v) in env.get_bindings_string() {
        cmd.env(k, v);
    }

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

    let mut master_reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;
    let mut master_writer = pair
        .master
        .take_writer()
        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

    // 输出转发线程

    let _output_thread = if is_shell || is_vi {
        // drop(_terminal_guard);
        thread::spawn(move || {
            loop {
                if running_clone2.load(Ordering::SeqCst) {
                    break;
                }

                // 将读取的数据输出到标准输出
                let mut buffer = [0u8; 1024];
                match master_reader.read(&mut buffer) {
                    Ok(_) => io::stdout().write_all(&buffer).unwrap(),
                    Err(_) => break,
                }
                thread::sleep(Duration::from_millis(20));
                let _ = io::stdout().flush();
            }
        })
    } else {
        thread::spawn(move || {
            let _ = io::copy(&mut master_reader, &mut io::stdout());
        })
    };

    let input_thread = thread::spawn(move || {
        if let Some(last_input) = input {
            if is_vi {
                let _ = master_writer.write_all("i".as_bytes());
            }
            if let Err(e) = master_writer.write_all(&last_input) {
                eprintln!("Failed to write to master: {}", e);
            }
            if let Err(e) = master_writer.flush() {
                eprintln!("Failed to flush master: {}", e);
            }
            if is_vi {
                let _ = master_writer.write_all(b"\n");
                let esc_char = [27u8];
                let _ = master_writer.write_all(&esc_char);
                thread::sleep(Duration::from_millis(50));
                let _ = master_writer.flush();
            }
        }

        let mut input_buffer = [0u8; 1];
        loop {
            if running_clone.load(Ordering::SeqCst) {
                break;
            }

            match io::stdin().read_exact(&mut input_buffer) {
                Ok(_) => {
                    let _ = master_writer.write_all(&input_buffer);
                }
                Err(_) => break,
            }
            thread::sleep(Duration::from_millis(80));
        }
    });

    child.wait()?;
    running.store(true, Ordering::SeqCst);
    let _ = input_thread.join();
    if is_shell || is_vi {
        let _ = _output_thread.join();
    }

    Ok(None)
}
