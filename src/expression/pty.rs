use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::thread;

use libc::winsize;

use crate::repl::{new_editor, read_user_input};
use crate::{Environment, RuntimeError};

// 统一接口
trait Pty {
    fn resize(&self, cols: u16, rows: u16) -> io::Result<()>;
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn spawn_command(&mut self, cmd: &str) -> io::Result<()>;
}

// #[cfg(unix)]
// fn create_pty() -> io::Result<Box<dyn Pty>> {
//     Ok(Box::new(unix_pty::UnixPty::new()?))
// }

#[cfg(windows)]
fn create_pty() -> io::Result<Box<dyn Pty>> {
    Ok(Box::new(win_pty::WinPty::new()?))
}

fn nix_pty(
    cmdstr: &String,
    args: Option<Vec<String>>,
    env: &mut Environment,
    input: Option<Vec<u8>>,
) -> () {
    use super::*;
    use nix::pty::{ForkptyResult, forkpty};
    use nix::sys::termios;
    use nix::sys::wait::{WaitStatus, waitpid};
    use nix::unistd::execvpe;
    use nix::unistd::{close, dup2_stderr, dup2_stdout};
    use nix::{pty::openpty, unistd::dup2_stdin};
    use std::ffi::CStr;
    use std::ffi::CString;
    use std::os::{
        fd::{AsFd, OwnedFd, RawFd},
        unix::io::{AsRawFd, FromRawFd},
    };
    unsafe {
        let ws = winsize {
            ws_row: 40,
            ws_col: 90,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        match forkpty(Some(&ws), None) {
            Ok(ForkptyResult::Parent { child, master }) => {
                // 父进程逻辑
                println!("Parent process PID: {}", nix::unistd::getpid());
                println!("Child process PID: {}", child);

                // 读取子进程的输出
                let mut buffer = [0u8; 1024];
                let master_clone = master.try_clone().unwrap();
                thread::spawn(move || {
                    loop {
                        let bytes_read =
                            nix::unistd::read(master_clone.as_fd(), &mut buffer).unwrap();
                        if bytes_read == 0 {
                            break; // 子进程结束
                        }
                        // 将读取的数据输出到标准输出
                        io::stdout().write_all(&buffer[..bytes_read]).unwrap();
                    }
                });

                // use std::sync::{Arc, Mutex};

                // let master_clone = Arc::new(Mutex::new(master));
                // thread::spawn({
                //     let master_clone = Arc::clone(&master_clone);
                //     move || {
                //         let mut buffer = [0u8; 1024];
                //         loop {
                //             let mut guard = master_clone.lock().unwrap();
                //             let bytes_read = nix::unistd::read(guard.as_fd(), &mut buffer).unwrap();
                //             if bytes_read == 0 {
                //                 break;
                //             }
                //             io::stdout().write_all(&buffer[..bytes_read]).unwrap();
                //         }
                //     }
                // });

                // let _ = dup2_stdin(master.try_clone().unwrap());
                // let _ = dup2_stdout(master.try_clone().unwrap());
                // let _ = dup2_stderr(master.try_clone().unwrap());

                // loop {
                //     // let bytes_read = nix::unistd::read(master.as_fd(), &mut buffer).unwrap();
                //     // if bytes_read == 0 {
                //     //     break; // 子进程结束
                //     // }
                //     // // 将读取的数据输出到标准输出
                //     // io::stdout().write_all(&buffer[..bytes_read]).unwrap();

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

                //     let _ = nix::unistd::write(master.as_fd(), &input_buffer);
                // }
                // 等待子进程退出
                match waitpid(child, None) {
                    Ok(WaitStatus::Exited(_, status)) => {
                        println!("Child process exited with status: {}", status);
                    }
                    Ok(status) => {
                        println!("Child process terminated abnormally: {:?}", status);
                    }
                    Err(e) => {
                        eprintln!("Failed to wait for child process: {}", e);
                    }
                }

                // 关闭主设备文件描述符
                // let master_clone = Arc::clone(&master_clone);

                close(master).expect("Failed to close master FD");
            }
            Ok(ForkptyResult::Child) => {
                // 子进程逻辑
                println!("Child process started.");
                use nix::sys::signal;

                signal::sigaction(
                    signal::Signal::SIGTERM,
                    &signal::SigAction::new(
                        signal::SigHandler::SigDfl,
                        signal::SaFlags::SA_RESTART,
                        signal::SigSet::empty(),
                    ),
                )
                .unwrap();

                // 设置环境变量（可选）
                // let env_vars = vec![
                //     CString::new("TERM=xterm-256color").unwrap(),
                //     CString::new(
                //         "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                //     )
                //     .unwrap(),
                // ];
                let env_vars = env
                    .get_bindings_string()
                    .iter()
                    .map(|(k, v)| CString::new(format!("{}={}", k, v)).unwrap())
                    .collect::<Vec<_>>();
                // 执行子进程命令（例如启动一个新的 shell）
                let cargs = match args {
                    Some(va) => va
                        .iter()
                        .map(|a| CString::new(a.as_str()).unwrap())
                        .collect(),
                    None => vec![],
                };
                let cmd = CString::new(cmdstr.as_str()).unwrap();

                match execvpe(&cmd, &cargs, &env_vars) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Failed to execute command: {}", e);
                        // exit(1);
                    }
                }

                // 子进程不会到达此处
            }
            Err(e) => {
                eprintln!("Failed to fork: {}", e);
            }
        }
    }
}

pub fn spawn_in_pty(
    cmdstr: &String,
    args: Option<Vec<String>>,
    env: &mut Environment,
    input: Option<Vec<u8>>,
) -> io::Result<()> {
    nix_pty(cmdstr, args, env, input);
    // let mut pty = create_pty()?;
    // pty.spawn_command(cmdstr)?;

    // // 设置初始终端大小
    // pty.resize(80, 24)?;

    // // 原始模式开关（Unix）
    // #[cfg(unix)]
    // let _guard = unix_pty::set_raw_mode()?;

    // // 主循环：转发输入输出
    // let mut buf = [0u8; 1024];
    // loop {
    //     // 从PTY读取输出并显示
    //     match pty.read(&mut buf) {
    //         Ok(0) => break, // EOF
    //         Ok(n) => {
    //             io::stdout().write_all(&buf[..n])?;
    //             io::stdout().flush()?;
    //         }
    //         Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
    //         Err(e) => return Err(e),
    //     }

    //     // 从用户输入读取并发送到PTY
    //     match io::stdin().read(&mut buf) {
    //         Ok(0) => break, // EOF
    //         Ok(n) => {
    //             pty.write(&buf[..n])?;
    //         }
    //         Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
    //         Err(e) => return Err(e),
    //     }
    // }

    Ok(())
}

// Unix PTY 实现
// #[cfg(unix)]
// mod unix_pty {
//     use super::*;
//     use nix::pty::{ForkptyResult, forkpty};
//     use nix::sys::termios;
//     use nix::sys::wait::{WaitStatus, waitpid};
//     use nix::unistd::execvpe;
//     use nix::unistd::{close, dup2_stderr, dup2_stdout};
//     use nix::{pty::openpty, unistd::dup2_stdin};
//     use std::ffi::CStr;
//     use std::ffi::CString;
//     use std::os::{
//         fd::{AsFd, OwnedFd, RawFd},
//         unix::io::{AsRawFd, FromRawFd},
//     };

//     pub struct UnixPty {
//         // master: std::fs::File,
//         master: OwnedFd,
//     }

//     impl UnixPty {
//         pub fn new() -> io::Result<UnixPty> {
//             let pty = openpty(None, None).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
//             Ok(Self {
//                 // master: unsafe { std::fs::File::from_raw_fd(pty.master.as_raw_fd()) },
//                 master: pty.master,
//             })
//         }
//     }

//     impl Pty for UnixPty {
//         fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
//             use libc::{TIOCSWINSZ, ioctl};
//             use nix::pty::Winsize;

//             let ws = Winsize {
//                 ws_row: rows,
//                 ws_col: cols,
//                 ws_xpixel: 0,
//                 ws_ypixel: 0,
//             };
//             // 使用 libc 的 ioctl 来设置终端大小
//             let ptr = &ws as *const Winsize as *mut Winsize;
//             let result = unsafe { ioctl(self.master.as_raw_fd(), TIOCSWINSZ, ptr) };

//             if result == -1 {
//                 return Err(io::Error::last_os_error());
//             }
//             Ok(())
//         }

//         fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
//             // self.master.read(buf)
//         }

//         fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
//             // self.master.write(buf)
//         }

//         fn spawn_command(&mut self, cmd: &str) -> io::Result<()> {
//             use nix::unistd::{ForkResult, dup2, fork};

//             match unsafe { fork() } {
//                 Ok(ForkResult::Parent { .. }) => Ok(()),
//                 Ok(ForkResult::Child) => {
//                     // 在子进程中执行命令
//                     // 修正点1: 创建临时的 OwnedFd 用于 dup2
//                     // let mut stdin_fd = unsafe { OwnedFd::from_raw_fd(0) };
//                     // let mut stdout_fd = unsafe { OwnedFd::from_raw_fd(1) };
//                     // let mut stderr_fd = unsafe { OwnedFd::from_raw_fd(2) };

//                     // 修正点2: 正确调用 dup2
//                     dup2_stdin(self.master.as_fd()).unwrap(); // 标准输入
//                     dup2_stdout(self.master.as_fd()).unwrap(); // 标准输出
//                     dup2_stderr(self.master.as_fd()).unwrap(); // 标准错误

//                     let mut command = Command::new("sh");
//                     command.arg("-c").arg(cmd);
//                     command.spawn();

//                     unreachable!();
//                 }
//                 Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
//             }
//         }
//     }

//     // 设置终端原始模式
//     pub fn set_raw_mode() -> io::Result<RawModeGuard> {
//         let fd = io::stdin().as_raw_fd();
//         let mut termios = termios::tcgetattr(io::stdin().as_fd())
//             .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
//         let original = termios.clone();

//         termios::cfmakeraw(&mut termios);
//         termios::tcsetattr(io::stdin().as_fd(), termios::SetArg::TCSANOW, &termios)
//             .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

//         Ok(RawModeGuard { fd, original })
//     }

//     // 自动恢复终端设置
//     pub struct RawModeGuard {
//         fd: RawFd,
//         original: termios::Termios,
//     }

//     impl Drop for RawModeGuard {
//         fn drop(&mut self) {
//             let _ = termios::tcsetattr(
//                 io::stdin().as_fd(),
//                 termios::SetArg::TCSANOW,
//                 &self.original,
//             );
//         }
//     }
// }

// Windows ConPTY 实现
#[cfg(windows)]
mod win_pty {
    use super::*;
    use conpty::Process;
    use winapi::um::wincon::{COORD, SMALL_RECT};

    pub struct WinPty {
        process: Process,
    }

    impl WinPty {
        pub fn new() -> io::Result<Self> {
            Ok(Self {
                process: Process::spawn("cmd /C btop").map_err(to_io_error)?,
            })
        }
    }

    impl Pty for WinPty {
        fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
            let size = COORD {
                X: cols as i16,
                Y: rows as i16,
            };
            let rect = SMALL_RECT {
                Left: 0,
                Top: 0,
                Right: (cols - 1) as i16,
                Bottom: (rows - 1) as i16,
            };
            self.process.resize(size, rect).map_err(to_io_error)
        }

        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.process.output().read(buf)
        }

        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.process.input().write(buf)
        }

        fn spawn_command(&mut self, cmd: &str) -> io::Result<()> {
            self.process = Process::spawn(&format!("cmd /C {}", cmd)).map_err(to_io_error)?;
            Ok(())
        }
    }

    fn to_io_error(e: impl std::fmt::Display) -> io::Error {
        io::Error::new(io::ErrorKind::Other, e.to_string())
    }
}
