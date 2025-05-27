// 包装器结构体
use nix::libc::{STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};
use nix::pty::{ForkptyResult, forkpty};
use nix::sys::signal;
use nix::sys::termios::{tcgetattr, tcsetattr};
use nix::unistd::{close, dup2_stderr, dup2_stdin, dup2_stdout, fork, getpid, getppid};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::process::Command;
use std::thread;

use crate::{Environment, RuntimeError};
use nix::pty::{Winsize, openpty};

pub fn exec_in_pty(
    cmdstr: &String,
    args: Option<Vec<String>>,
    env: &mut Environment,
    input: Option<Vec<u8>>, // 前一条命令的输出（None 表示第一个命令）
    pipe_out: bool,
    mode: u8,
) {
    exec_in_pty0(cmdstr, args, env, input, pipe_out, mode);
}
fn exec_in_pty2(
    cmdstr: &String,
    args: Option<Vec<String>>,
    env: &mut Environment,
    input: Option<Vec<u8>>, // 前一条命令的输出（None 表示第一个命令）
    pipe_out: bool,
    mode: u8,
) {
    // 设置伪终端窗口大小
    let winsize = Winsize {
        ws_row: 40,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    // 创建伪终端
    let pty = openpty(Some(&winsize), None).expect("Failed to create pty");

    // 创建子进程
    match unsafe { fork() } {
        Ok(nix::unistd::ForkResult::Parent { child, .. }) => {
            // 父进程逻辑
            println!("Parent PID: {}", getpid());
            println!("Child PID: {}", child);

            // 将父进程的标准输入、输出和错误流重定向到主设备文件描述符
            // dup2_stdin(pty.master);
            // let _ = dup2_stdin(pty.master.try_clone().unwrap());
            // let _ = dup2_stderr(pty.master.try_clone().unwrap());
            // let _ = dup2_stdout(pty.master.try_clone().unwrap());

            // 读取子进程的输出
            let mut buffer = [0u8; 1024];
            let master_clone = pty.master.try_clone().unwrap();
            let master_clone2 = pty.master.try_clone().unwrap();
            let t0_guard = thread::spawn(move || {
                loop {
                    let bytes_read = nix::unistd::read(master_clone.as_fd(), &mut buffer).unwrap();
                    if bytes_read == 0 {
                        break; // 子进程结束
                    }
                    // 将读取的数据输出到标准输出
                    io::stdout().write_all(&buffer[..bytes_read]).unwrap();
                }
            });
            let t1_guard = std::thread::spawn(move || {
                loop {
                    // let bytes_read = nix::unistd::read(master.as_fd(), &mut buffer).unwrap();
                    // if bytes_read == 0 {
                    //     break; // 子进程结束
                    // }
                    // // 将读取的数据输出到标准输出
                    // io::stdout().write_all(&buffer[..bytes_read]).unwrap();

                    // input
                    let mut input_buffer = [0u8; 10];

                    let n = io::stdin()
                        .read(&mut input_buffer)
                        .expect("Failed to read line");
                    // 将输入写入伪终端
                    if n == 0 || n == 1 && input_buffer[0].to_string() == "q" {
                        break;
                    }
                    // let master_clone = Arc::clone(&master_clone);

                    let _ = nix::unistd::write(master_clone2.as_fd(), &input_buffer);
                }
                // std::io::copy(&mut std::io::stdin(), &mut pty.master.).unwrap();
            });
            // 等待子进程结束
            println!("Waiting for child process...");
            match nix::sys::wait::waitpid(child, None) {
                Ok(_) => {
                    println!("Child process exited.");
                    let _ = t0_guard.join();
                    let _ = t1_guard.join();
                }
                Err(e) => {
                    eprintln!("Failed to wait for child process: {}", e);
                    let _ = t0_guard.join();
                    let _ = t1_guard.join();
                }
            }

            // 关闭主设备文件描述符
            close(pty.master.try_clone().unwrap().as_raw_fd()).expect("Failed to close master FD");
        }
        Ok(nix::unistd::ForkResult::Child) => {
            // 子进程逻辑
            println!("Child PID: {}", getpid());

            // 将从设备文件描述符设置为标准输入、输出和错误流
            // let _ = dup2_stdin(pty.slave.try_clone().unwrap());
            // let _ = dup2_stdout(pty.slave.try_clone().unwrap());
            // let _ = dup2_stderr(pty.slave.try_clone().unwrap());

            // 设置环境变量（可选）
            // nix::unistd::setenv("TERM", "xterm-256color", true).unwrap();

            // 启动子进程命令
            let mut command = std::process::Command::new(cmdstr)
                .stdin(pty.slave.try_clone().unwrap())
                .stdout(pty.slave.try_clone().unwrap())
                .stderr(pty.slave.try_clone().unwrap())
                .spawn()
                .expect("Failed to start htop");

            // 等待子进程退出
            match command.wait() {
                Ok(status) => println!("Child process exited with status: {:?}", status),
                Err(e) => eprintln!("Failed to wait for child process: {}", e),
            }
        }
        Err(e) => {
            eprintln!("Failed to fork: {}", e);
        }
    }
}

fn exec_in_pty0(
    cmdstr: &String,
    args: Option<Vec<String>>,
    env: &mut Environment,
    input: Option<Vec<u8>>, // 前一条命令的输出（None 表示第一个命令）
    pipe_out: bool,
    mode: u8,
) -> Result<Option<Vec<u8>>, RuntimeError> {
    // 1. 创建PTY系统（自动选择平台适配的实现）
    let pty_system = native_pty_system();

    // 2. 创建PTY终端
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            // 像素尺寸可忽略（通常为0）
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();

    // 2. 启动子进程（关键配置）
    let cmd = CommandBuilder::new(cmdstr);

    // 3. 通过PTY启动（完全脱离当前终端）
    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

    // 4. 主进程转发逻辑
    // disable_line_buffering();
    let mut master_reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;
    let mut master_writer = pair
        .master
        .take_writer()
        .map_err(|e| RuntimeError::CustomError(e.to_string()))?;

    // 4. 输入转发线程（处理预先输入的input）
    // if let Some(input) = input {
    //     let mut writer = master_writer;
    //     thread::spawn(move || {
    //         writer.write_all(&input).unwrap();
    //     });
    // }

    // 简单转发逻辑
    let guard_1 = std::thread::spawn(move || {
        std::io::copy(&mut master_reader, &mut std::io::stdout()).unwrap();
    });

    // 处理输入（自动支持控制字符）
    let guard_2 = std::thread::spawn(move || {
        std::io::copy(&mut std::io::stdin(), &mut master_writer).unwrap();

        // loop {
        //     // input
        //     let mut input_buffer = [0u8; 10];

        //     let n = io::stdin()
        //         .read(&mut input_buffer)
        //         .expect("Failed to read line");
        //     // 将输入写入伪终端
        //     if n == 0 || n == 1 && input_buffer[0].to_string() == "q" {
        //         break;
        //     }
        //     // let master_clone = Arc::clone(&master_clone);

        //     let _ = master_writer.write(&input_buffer);
        //     // let _ = master_writer.flush();
        //     // let _ = nix::unistd::write(pair.master.take_writer().as_mut(), &input_buffer);
        // }
    });

    println!("当前终端: {:?}", std::env::var("TERM")); // 检查终端类型
    println!("控制终端: {:?}", unsafe { libc::isatty(0) }); // 检查stdin是否终端
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
        .unwrap();
    }
    // 使用通道实现简易退出
    match child.wait() {
        Ok(_) => {
            println!("Child process exited.");
            let _ = guard_1.join();
            let _ = guard_2.join();
        }
        Err(e) => {
            eprintln!("Failed to wait for child process: {}", e);
            let _ = guard_1.join();
            let _ = guard_2.join();
        }
    }
    Ok(None)
}

use nix::sys::termios::{self, LocalFlags, SetArg, Termios};

fn configure_terminal(fd: i32) -> Result<(), String> {
    // 获取当前终端属性
    unsafe {
        let mut termios =
            termios::tcgetattr(BorrowedFd::borrow_raw(fd)).map_err(|e| e.to_string())?;

        // 修改属性：关闭回显和行缓冲
        termios.local_flags &= !(LocalFlags::ECHO | LocalFlags::ICANON);

        // 应用修改后的属性
        tcsetattr(BorrowedFd::borrow_raw(fd), SetArg::TCSAFLUSH, &termios)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn exec_in_pty3(
    cmdstr: &String,
    args: Option<Vec<String>>,
    env: &mut Environment,
    input: Option<Vec<u8>>, // 前一条命令的输出（None 表示第一个命令）
    pipe_out: bool,
    mode: u8,
) {
    // 创建PTY系统
    let pty_system = portable_pty::native_pty_system();

    // 创建PTY终端
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();

    // 获取主终端的文件描述符
    let fd = pair.master.as_raw_fd().unwrap();

    // 配置主终端为原始模式
    if let Err(err) = configure_terminal(fd) {
        eprintln!("Failed to configure terminal: {}", err);
        return;
    }

    // 启动子进程
    let mut child = pair
        .slave
        .spawn_command(CommandBuilder::new(cmdstr))
        .unwrap();

    // 主进程与子进程交互
    let mut master_reader = pair.master.try_clone_reader().unwrap();
    let mut master_writer = pair.master.take_writer().unwrap();

    // 将子进程的输出转发到主进程
    let guard_1 = std::thread::spawn(move || {
        std::io::copy(&mut master_reader, &mut std::io::stdout()).unwrap();
    });
    // 捕获用户输入并转发到子进程
    // std::thread::spawn(move || {
    //     loop {
    //         let mut input = String::new();
    //         match std::io::stdin().read_line(&mut input) {
    //             Ok(0) => break, // EOF
    //             Ok(_) => {
    //                 master_writer.write_all(input.as_bytes()).unwrap();
    //             }
    //             Err(e) => {
    //                 eprintln!("Input error: {}", e);
    //                 break;
    //             }
    //         }
    //     }
    // });
    enable_raw_mode();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen);

    println!("Press any key...");

    let guard_2 = std::thread::spawn(move || {
        loop {
            if event::poll(std::time::Duration::from_millis(500)).unwrap() {
                if let Event::Key(key_event) = event::read().unwrap() {
                    match key_event.code {
                        KeyCode::Char('q') => {
                            println!("\nExiting...");
                            break;
                        }
                        KeyCode::Char(c) => {
                            // println!("You pressed: '{}'", c);
                            master_writer.write_all(c.to_string().as_bytes()).unwrap();
                            master_writer.flush().unwrap();
                        }
                        _ => {
                            println!("You pressed: {:?}", key_event.code);
                        }
                    }
                }
            }
        }
    });
    execute!(stdout, LeaveAlternateScreen);
    disable_raw_mode();

    // 等待子进程结束
    match child.wait() {
        Ok(_) => {
            println!("Child process exited.");
            let _ = guard_1.join();
            let _ = guard_2.join();
        }
        Err(e) => {
            eprintln!("Failed to wait for child process: {}", e);
            let _ = guard_1.join();
            let _ = guard_2.join();
        }
    }
}

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
