use crossterm::terminal::{self, ClearType};
use terminal_size::{Height, Width, terminal_size};
// 包装器结构体
use crate::{Environment, RuntimeError};
use crossterm::execute;
use nix::sys::signal;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub fn exec_in_pty(
    cmdstr: &String,
    args: Option<Vec<String>>,
    env: &mut Environment,
    input: Option<Vec<u8>>, // 前一条命令的输出（None 表示第一个命令）
) -> Result<Option<Vec<u8>>, RuntimeError> {
    let _terminal_guard = TerminalGuard::new()?;
    // terminal::enable_raw_mode()?;

    // 设置信号处理，确保在收到信号时能正确清理
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

    execute!(io::stdout(), terminal::Clear(ClearType::All))?;

    // 1. 创建PTY系统（自动选择平台适配的实现）
    let pty_system = native_pty_system();

    // 2. 创建PTY终端
    let size = terminal_size();
    let (w, h) = match size {
        Some((Width(w), Height(h))) => (w, h),
        _ => (24, 80),
    };
    let pair = pty_system
        .openpty(PtySize {
            rows: h,
            cols: w,
            // 像素尺寸可忽略（通常为0）
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();

    // 2. 启动子进程（关键配置）
    let mut cmd = CommandBuilder::new(cmdstr);
    if let Some(ag) = args {
        cmd.args(ag);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    for (k, v) in env.get_bindings_string() {
        cmd.env(k, v);
    }
    // 3. 通过PTY启动（完全脱离当前终端）
    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

    // 4. 主进程转发逻辑
    let master_fd = pair.master.as_raw_fd().unwrap(); // 保存文件描述符
    // 示例：使用 libc 直接配置 PTY
    unsafe {
        let mut termios = std::mem::zeroed();
        libc::tcgetattr(master_fd, &mut termios);

        // 启用关键控制标志
        termios.c_lflag |= libc::ECHO | libc::ICANON;
        termios.c_oflag |= libc::OPOST;
        if cmdstr == "vi" {
            termios.c_iflag &= !libc::IXON; // 禁用流控制
        }
        libc::tcsetattr(master_fd, libc::TCSANOW, &termios);
    }

    // disable_line_buffering();
    let mut master_reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;
    let mut master_writer = pair
        .master
        .take_writer()
        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

    // 简单转发逻辑
    // 读取子进程的输出
    // let mut buffer = [0u8];
    // let guard_1 = thread::spawn(move || {
    //     loop {
    //         // 将读取的数据输出到标准输出
    //         match master_reader.read(&mut buffer) {
    //             Ok(_) => io::stdout().write_all(&buffer).unwrap(),
    //             Err(_) => break,
    //         }
    //     }
    // });
    let _ = std::thread::spawn(move || {
        std::io::copy(&mut master_reader, &mut std::io::stdout()).unwrap();
    });

    // 处理输入（自动支持控制字符）
    let running = Arc::new(AtomicBool::new(false));
    let running_clone = Arc::clone(&running);
    // let cmdstr_clone = cmdstr.clone();
    let guard_2 = thread::spawn(move || {
        //     std::io::copy(&mut std::io::stdin(), &mut master_writer).unwrap();
        // });
        if let Some(last_input) = input {
            // println!("Writing input: {:?}", last_input); // 调试输出
            if let Err(e) = master_writer.write_all(&last_input) {
                eprintln!("Failed to write to master: {}", e);
            }
            if let Err(e) = master_writer.flush() {
                eprintln!("Failed to flush master: {}", e);
            }
            thread::sleep(Duration::from_millis(200));
        }
        loop {
            if running_clone.load(Ordering::SeqCst) {
                drop(master_writer);
                break;
            }
            // input
            let mut input_buffer = [0u8];
            // println!("waiting input");
            // execute!(io::stdout(), cursor::MoveTo(0, 10)).unwrap(); // 移动光标到 (0, 0)
            // println!("{}> ", cmdstr_clone);
            // io::stdout().flush().unwrap();

            match io::stdin().read_exact(&mut input_buffer) {
                Ok(_) => {}
                Err(_) => {
                    drop(master_writer);
                    break;
                }
            }
            // println!("got");
            // 将输入写入伪终端
            if input_buffer.len() == 0
            // || input_buffer.len() == 1 && input_buffer[0].to_string() == "113"
            {
                drop(master_writer);
                break;
            }
            // let master_clone = Arc::clone(&master_clone);

            let _ = master_writer.write_all(&input_buffer);
            // let _ = master_writer.flush();
            // println!(
            //     "writed: {} bytes: {}",
            //     n.unwrap(),
            //     input_buffer[0].to_string()
            // );
            // let _ = nix::unistd::write(pair.master.take_writer().as_mut(), &input_buffer);
            thread::sleep(Duration::from_millis(150));
        }
    });

    // println!("当前终端: {:?}", pair.master.tty_name()); // 检查终端类型
    // println!("终端: {:?}", std::env::var("TERM")); // 检查终端类型
    // println!("控制终端: {:?}", unsafe { libc::isatty(0) }); // 检查stdin是否终端

    // 使用通道实现简易退出
    // match child.wait() {
    //     Ok(_) => {
    //         println!("Child process exited.");
    //     }
    //     Err(e) => {
    //         eprintln!("Failed to wait for child process: {}", e);
    //     }
    // }
    child.wait()?;
    // println!("---closing 1");
    // let _ = guard_1.join();
    // println!("---closing 2");
    running.store(true, Ordering::SeqCst); // 设置停止标志
    let _ = guard_2.join();
    // println!("---closing 3");

    // 子进程退出后关闭
    // 显式释放（非必需但推荐）
    // drop(controller);
    drop(pair.master);
    unsafe {
        libc::close(master_fd);
    }
    println!("\nbye!");

    // // 显式关闭主设备
    // drop(master_reader);
    // drop(master_writer);
    // drop(pair.master);

    // println!(
    //     "tty:{}, terminal: {}",
    //     io::stdin().is_tty(),
    //     io::stdin().is_terminal()
    // );
    // terminal::disable_raw_mode()?;
    // execute!(io::stdout(), terminal::Clear(ClearType::All))?;

    Ok(None)
}

use std::sync::atomic::{AtomicBool, Ordering};

// RAII守卫，确保终端模式总是被恢复
struct TerminalGuard;
impl TerminalGuard {
    fn new() -> Result<Self, RuntimeError> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // 确保在退出时恢复终端模式
        let _ = terminal::disable_raw_mode();
        unsafe {
            libc::kill(0, libc::SIGCONT);
        }
    }
}
