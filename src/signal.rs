//! Signal bits: a task's own bits, allocated and freed by RAII.
//!
//! exec.doc, AllocSignal: "Allocated signals are only valid for use with the
//! task that allocated them", and they "should be deallocated before the
//! task exits". So [`Signal`] is neither `Send` nor `Sync`: it lives and
//! dies on the task that made it. Another task reaches it through a
//! [`SignalTarget`], whose sending is `unsafe` (the receiving task must
//! still exist and still own the bit).
//!
//! Signals must not be allocated or freed from exception code, and `Wait`
//! must never be called from an interrupt.

use core::marker::PhantomData;
use core::ptr::null;

use crate::error::{AmigaError, Result};
use crate::sys::{self, Task};

/// The calling task.
#[inline]
pub fn current_task() -> *mut Task {
    // SAFETY: FindTask(NULL) returns the caller and touches nothing.
    unsafe { sys::exec_find_task(null()) }
}

/// Wait until one of `mask`'s signals arrives; returns those that did (and
/// clears them). `mask` 0 waits forever.
#[inline]
pub fn wait(mask: u32) -> u32 {
    // SAFETY: Wait affects only the caller's own signals.
    unsafe { sys::exec_wait(mask) }
}

/// Which of `mask`'s signals are set now, clearing them (no waiting). A
/// Shell program's CTRL-C check is `take(SIGBREAKF_CTRL_C) != 0`.
#[inline]
pub fn take(mask: u32) -> u32 {
    // SAFETY: SetSignal on the caller's own signals.
    unsafe { sys::exec_set_signal(0, mask) & mask }
}

/// Which of `mask`'s signals are set now, leaving them set.
#[inline]
pub fn peek(mask: u32) -> u32 {
    // SAFETY: SetSignal(0, 0) only reads.
    unsafe { sys::exec_set_signal(0, 0) & mask }
}

/// Ctrl-C for the calling task, as a request to stop: the Shell (and
/// `Break`) send SIGBREAKF_CTRL_C. A long loop calls [`Cancel::check`]
/// now and then; waits that should end on Ctrl-C include [`Cancel::MASK`]
/// (as `timer::Waiter` does).
pub struct Cancel;

impl Cancel {
    /// The signal Ctrl-C sets.
    pub const MASK: u32 = sys::SIGBREAKF_CTRL_C;

    /// Has Ctrl-C come since the last look? Clears it.
    pub fn requested() -> bool {
        take(Self::MASK) != 0
    }

    /// `Err(Cancelled)` if Ctrl-C has come (cleared), else `Ok(())`.
    pub fn check() -> Result<()> {
        if Self::requested() { Err(AmigaError::Cancelled) } else { Ok(()) }
    }
}

/// One signal bit of the calling task, freed on drop.
pub struct Signal {
    bit: i8,
    _t: crate::track::Tracked,
    _not_send: PhantomData<*mut ()>,
}

/// Free a registered signal bit (the panic path; see `crate::track`).
unsafe fn release_signal(bit: usize, _: usize) {
    // SAFETY: the registry calls this once, in the task that allocated it.
    unsafe { sys::exec_free_signal(bit as i8) }
}

impl Signal {
    /// Allocate any free bit (AllocSignal(-1)); the bit starts clear.
    #[track_caller]
    pub fn alloc() -> Result<Self> {
        // SAFETY: allocates from the caller's own pool.
        let bit = unsafe { sys::exec_alloc_signal(-1) };
        if bit < 0 {
            Err(AmigaError::AllocationFailed)
        } else {
            Ok(Self { bit, _t: crate::track::register_bound("signal bit", bit as u8 as usize, 0, release_signal), _not_send: PhantomData })
        }
    }

    /// The bit number, 0..31.
    #[inline]
    pub fn bit(&self) -> u8 {
        self.bit as u8
    }

    /// `1 << bit`.
    #[inline]
    pub fn mask(&self) -> u32 {
        1u32 << self.bit
    }

    /// Wait for this signal alone.
    pub fn wait(&self) {
        wait(self.mask());
    }

    /// Wait for this signal or any of `others`; returns the signals that
    /// arrived (this one's bit included when it did).
    pub fn wait_or(&self, others: u32) -> u32 {
        wait(self.mask() | others)
    }

    /// Is it set? Clears it if so.
    pub fn take(&self) -> bool {
        take(self.mask()) != 0
    }

    /// Clear it without waiting.
    pub fn clear(&self) {
        take(self.mask());
    }

    /// The address another task signals this one through.
    pub fn target(&self) -> SignalTarget {
        SignalTarget { task: current_task(), mask: self.mask() }
    }
}

impl Drop for Signal {
    fn drop(&mut self) {
        // SAFETY: the bit was allocated by this task (Signal is !Send).
        unsafe { sys::exec_free_signal(self.bit) }
    }
}

/// A task and a signal mask, to be signalled from another task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignalTarget {
    task: *mut Task,
    mask: u32,
}

// SAFETY: a task pointer and a mask are plain values; the one use of them
// (send) is unsafe and carries the contract.
unsafe impl Send for SignalTarget {}
// SAFETY: as for Send: nothing is reached through a shared reference.
unsafe impl Sync for SignalTarget {}

impl SignalTarget {
    /// A target from parts (for example a task's CTRL-C:
    /// `SignalTarget::new(task, SIGBREAKF_CTRL_C)`).
    pub const fn new(task: *mut Task, mask: u32) -> Self {
        Self { task, mask }
    }

    pub fn task(&self) -> *mut Task {
        self.task
    }

    pub fn mask(&self) -> u32 {
        self.mask
    }

    /// Signal the task.
    ///
    /// # Safety
    /// The task still exists, and still owns the bits (a freed bit may have
    /// been given to some other use).
    pub unsafe fn send(&self) {
        // SAFETY: the caller guarantees the task exists and owns the bits.
        unsafe { sys::exec_signal(self.task, self.mask) }
    }
}
