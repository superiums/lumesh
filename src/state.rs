use signal_hook::{consts::signal::SIGINT, iterator::Signals};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn register_signal_handler() {
    let mut signals = Signals::new([SIGINT]).unwrap();
    std::thread::spawn(move || {
        for sig in signals.forever() {
            println!("\nReceived signal: {:?}", sig);
            set_signal(); // 更新共享状态
            break;
        }
    });
}

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);
pub fn set_signal() {
    SHUTDOWN_FLAG.store(true, Ordering::Relaxed);
}

pub fn read_signal() -> bool {
    let r = SHUTDOWN_FLAG.load(Ordering::Relaxed);
    SHUTDOWN_FLAG.store(false, Ordering::Relaxed);
    r
}
