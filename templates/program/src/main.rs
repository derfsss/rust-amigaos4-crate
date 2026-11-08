//! my-program: a template for an AmigaOS 4.1 program in Rust.
//!
//!   my-program [NAME] [REQUESTER/S]      (NAME,REQUESTER/S)
//!
//! An ordinary Rust binary with `fn main` and std on clib4: println!,
//! std::env, std::fs and threads work as on any platform, and the amigaos4
//! crate adds the AmigaOS side (here ReadArgs and an Intuition requester).
//! Every interface global the crate reads has a weak NULL definition, so
//! nothing here declares IIntuition and the rest: the crate opens what it
//! uses.
//!
//! Build: tools/build-package.sh templates/program (on Linux or WSL); the
//! program is target/powerpc-unknown-amigaos/release/my-program.

use amigaos4::dos::{self, args::Args};
use amigaos4::error::AmigaError;
use amigaos4::intuition::easy_request;

/// The stack AmigaOS gives the program when it starts (the Shell and
/// Workbench read this cookie).
#[used]
static STACK: [u8; 14] = *b"$STACK:262144\0";

/// The version string the Version command and Workbench read.
#[used]
static VERSION: [u8; 33] = *b"$VER: my-program 0.1 (5.11.2026)\0";

const TEMPLATE: &str = "NAME,REQUESTER/S";

fn main() {
    let args = match Args::from_command_line(TEMPLATE) {
        Ok(a) => a,
        Err(AmigaError::DosError(c)) => {
            eprintln!("my-program: {} (template {})", dos::fault(c), TEMPLATE);
            std::process::exit(10);
        }
        Err(e) => {
            eprintln!("my-program: {}", e);
            std::process::exit(10);
        }
    };
    let name = args.text("NAME").ok().flatten().unwrap_or("Amiga").to_string();
    let greeting = format!("Hello, {}, from Rust.", name);
    println!("{}", greeting);
    println!("std::env::args: {:?}", std::env::args().collect::<Vec<_>>());

    if args.switch("REQUESTER").unwrap_or(false) {
        // No window: the requester opens on the default public screen.
        match easy_request(None, "my-program", &greeting, &["Nice", "Quit"]) {
            Ok(k) => println!("requester: button {}", k),
            Err(e) => eprintln!("my-program: requester: {}", e),
        }
    }
}
