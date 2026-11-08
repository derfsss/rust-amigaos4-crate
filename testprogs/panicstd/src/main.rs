//! panicstd: a std (clib4) program that panics, with
//! `amigaos4::track::install_std_hook`: the report goes to the serial
//! output and T:panicstd.crash; with unwinding (this profile) the
//! destructors release the lock it held, which `panictest check` confirms.

use amigaos4::dos::fs::{File, Lock, LockMode};

fn main() {
    amigaos4::track::install_std_hook();
    let _ = File::write_new("RAM:panictest.lock", b"x");
    let l = Lock::new("RAM:panictest.lock", LockMode::Exclusive);
    println!("panicstd: holding the lock: {}", l.is_ok());
    let v: Vec<u32> = Vec::new();
    let n = std::hint::black_box(3usize);
    println!("{}", v[n]);
}
