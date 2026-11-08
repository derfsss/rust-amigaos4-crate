//! Running a closure on a stack of its own (NewStackRun, V52.6): for deep
//! recursion or big locals without making the whole task's stack larger.
//! The stack is allocated for the call and freed when it returns, in the
//! calling task.

use crate::error::{AmigaError, Result};
use crate::sys::{self, TagItem, APTR, TAG_DONE};

struct Packet<F, R> {
    f: Option<F>,
    r: Option<R>,
}

extern "C" fn trampoline<F: FnOnce() -> R, R>(packet: u32) -> i32 {
    let p = packet as *mut Packet<F, R>;
    // SAFETY: the packet on the caller's stack, alive across NewStackRun,
    // which calls this once in the same task.
    unsafe {
        if let Some(f) = (*p).f.take() {
            (*p).r = Some(f());
        }
    }
    0
}

/// Run `f` on a new stack of `size` bytes (not less: NSR_MinStackSize is
/// the same), and return its result. `Err(AllocationFailed)` when exec
/// cannot provide the stack.
pub fn run_with_stack<F: FnOnce() -> R, R>(size: u32, f: F) -> Result<R> {
    let mut packet = Packet { f: Some(f), r: None };
    let entry: extern "C" fn(u32) -> i32 = trampoline::<F, R>;
    let tags = [
        TagItem { ti_Tag: sys::NSR_STACKSIZE, ti_Data: size },
        TagItem { ti_Tag: sys::NSR_MINSTACKSIZE, ti_Data: size },
        TagItem { ti_Tag: sys::NSR_ARG1, ti_Data: &mut packet as *mut Packet<F, R> as u32 },
        TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: a native entry taking the packet's address, which outlives
    // the call; a TAG_DONE-ended tag list.
    let rc = unsafe { sys::exec_new_stack_run(entry as usize as APTR, tags.as_ptr()) };
    match packet.r.take() {
        Some(r) if rc == 0 => Ok(r),
        _ => Err(AmigaError::AllocationFailed),
    }
}
