//! Hardware interrupt servers.
//!
//! [`IntServer`] allocates an interrupt node with
//! `AllocSysObject(ASOT_INTERRUPT)` and adds it to a server chain with
//! `AddIntServer`; dropping it removes it (`RemIntServer`) and frees the
//! node. For a PCI device, the chain number comes from
//! `PCIDevice->MapInterrupt()`.
//!
//! From the exec autodocs (SDK 54.25):
//!
//! - the V50 server is `ULONG handler(struct ExceptionContext *ctx,
//!   struct ExecBase *SysBase, APTR userData)` ([`ServerFn`]); `userData` is
//!   the node's `is_Data`;
//! - servers run in priority order "until the chain ends or one of the
//!   servers returns with a non-zero return value". On a shared PCI line,
//!   return 0 unless your device raised the interrupt;
//! - a server runs as an interrupt: no waiting, no allocation, nothing that
//!   can block. `Signal` "is safe to call from interrupts", which is how a
//!   server wakes the task that does the real work;
//! - a PCI INTx line is level-triggered: before returning 1 the server must
//!   make its device drop the line (usually by reading or acknowledging its
//!   status register), or the chain is entered again at once;
//! - everything the server reads must be in place before [`IntServer::add`]:
//!   it can run the instant the node is on the chain;
//! - end the server with [`arch::clear_reservation`](crate::arch::clear_reservation)
//!   if it uses atomics.

use core::ffi::c_void;

use crate::cstr::require_nul;
use crate::error::{AmigaError, Result};
use crate::sys::{self, TagItem};

/// A V50 interrupt server: `(context, ExecBase, is_Data) -> handled`.
pub type ServerFn = unsafe extern "C" fn(ctx: *mut c_void, sysbase: *mut c_void, data: *mut c_void) -> u32;

/// Remove and free a registered interrupt server (the panic path; see
/// `crate::track`): it must not run once the program's code is gone.
unsafe fn release_int_server(int_num: usize, node: usize) {
    // SAFETY: the registry calls this once, for a node still on its chain.
    unsafe {
        sys::exec_rem_int_server(int_num as u32, node as *mut sys::Interrupt);
        sys::exec_free_sys_object(sys::ASOT_INTERRUPT, node as sys::APTR);
    }
}

/// Remove and free a registered reset callback.
unsafe fn release_reset_callback(node: usize, _: usize) {
    // SAFETY: as release_int_server, for a node on the reset list.
    unsafe {
        sys::exec_rem_reset_callback(node as *mut sys::Interrupt);
        sys::exec_free_sys_object(sys::ASOT_INTERRUPT, node as sys::APTR);
    }
}

/// Remove and free a registered low-memory handler.
unsafe fn release_mem_handler(node: usize, _: usize) {
    // SAFETY: as release_int_server, for a node on the handler list.
    unsafe {
        sys::exec_rem_mem_handler(node as *mut sys::Interrupt);
        sys::exec_free_sys_object(sys::ASOT_INTERRUPT, node as sys::APTR);
    }
}

/// An interrupt server on one chain. Removed and freed on drop.
pub struct IntServer {
    _t: crate::track::Tracked,
    int_num: u32,
    node: *mut sys::Interrupt,
}

impl IntServer {
    /// Allocate a node for `code` with `data` as its `is_Data`, and add it
    /// to chain `int_num` at priority `pri`. `name` must end in `\0` (use
    /// [`amstr!`](crate::amstr)); by convention it says who owns the server.
    ///
    /// # Errors
    ///
    /// `NotNulTerminated` for a bad name, `AllocationFailed` if the node
    /// cannot be allocated, and `NullPointer` if `AddIntServer` refuses the
    /// chain (the autodoc: "FALSE might indicate that the appropriate
    /// interrupt does not exist").
    ///
    /// # Safety
    ///
    /// `code` runs in interrupt context with `data` until this server is
    /// dropped: it must follow the rules above, and `data` must stay valid
    /// for that long.
    #[track_caller]
    pub unsafe fn add(
        int_num: u32,
        pri: i8,
        name: &'static [u8],
        code: ServerFn,
        data: *mut c_void,
    ) -> Result<Self> {
        let node = alloc_node(pri, name, code as usize, data)?;
        // SAFETY: a fresh node; `code` and `data` meet the caller's contract.
        if unsafe { sys::exec_add_int_server(int_num, node) } == 0 {
            // SAFETY: the node was refused, so nothing else holds it.
            unsafe { sys::exec_free_sys_object(sys::ASOT_INTERRUPT, node as sys::APTR) };
            return Err(AmigaError::NullPointer);
        }
        Ok(Self { int_num, node, _t: crate::track::register("interrupt server", int_num as usize, node as usize, release_int_server) })
    }

    /// The chain this server is on.
    #[inline]
    pub fn int_num(&self) -> u32 {
        self.int_num
    }

    /// The interrupt node.
    #[inline]
    pub fn as_ptr(&self) -> *mut sys::Interrupt {
        self.node
    }
}

impl Drop for IntServer {
    fn drop(&mut self) {
        // SAFETY: the node was added to this chain in add().
        unsafe {
            sys::exec_rem_int_server(self.int_num, self.node);
            sys::exec_free_sys_object(sys::ASOT_INTERRUPT, self.node as sys::APTR);
        }
    }
}

/// Allocate an interrupt node for the function at `code`, for a server, a
/// reset callback or a low-memory handler. `ASOINTR_CopyName`: exec keeps
/// its own copy of the name, so the node never holds a pointer into caller
/// memory that a chain walk would read. Allocating runs nothing: `code` and
/// `data` are only stored, so this is safe; adding the node to a list is
/// what the callers' contracts cover.
fn alloc_node(pri: i8, name: &'static [u8], code: usize, data: *mut c_void) -> Result<*mut sys::Interrupt> {
    let name_ptr = require_nul(name)?;
    let tags = [
        TagItem { ti_Tag: sys::ASOINTR_CODE, ti_Data: code as u32 },
        TagItem { ti_Tag: sys::ASOINTR_DATA, ti_Data: data as usize as u32 },
        TagItem { ti_Tag: sys::ASOINTR_PRI, ti_Data: pri as i32 as u32 },
        TagItem { ti_Tag: sys::ASOINTR_NAME, ti_Data: name_ptr as usize as u32 },
        TagItem { ti_Tag: sys::ASOINTR_COPY_NAME, ti_Data: 1 },
        TagItem { ti_Tag: sys::TAG_DONE, ti_Data: 0 },
    ];
    // SAFETY: a TAG_DONE-ended tag list; the name is NUL-terminated and
    // copied by exec (ASOINTR_CopyName).
    let node = unsafe { sys::exec_alloc_sys_object(sys::ASOT_INTERRUPT, tags.as_ptr()) } as *mut sys::Interrupt;
    if node.is_null() {
        Err(AmigaError::AllocationFailed)
    } else {
        Ok(node)
    }
}

/// A reset callback: `code` runs when the machine reboots.
///
/// From exec.doc AddResetCallback (V50): on `ColdReboot()` exec walks the
/// callbacks in priority order and calls each as
/// `is_Code(ctx, ExecBase, is_Data)` with `ctx` 0, "in user mode ... and
/// during a Forbid()"; the return value is ignored. Its job is to stop the
/// hardware, typically a device doing DMA, so the next boot does not find it
/// writing into memory that has been handed out again.
///
/// The same list is also walked by a power-off, in supervisor mode, so the
/// callback must not depend on the mode; and it may be entered twice, so
/// guard it. It takes no lock and does only what stops the hardware.
///
/// Dropping a `ResetCallback` removes it (`RemResetCallback`). Code that is
/// never unloaded (a kickstart module) keeps it for its whole life with
/// [`core::mem::forget`]; code that can be unloaded must drop it first, or
/// exec calls freed memory at the next reboot.
pub struct ResetCallback {
    _t: crate::track::Tracked,
    node: *mut sys::Interrupt,
}

impl ResetCallback {
    /// Allocate a node for `code` and add it to the reset list at `pri`
    /// (127 runs first). `name` must end in `\0`.
    ///
    /// # Errors
    ///
    /// `NotNulTerminated`, `AllocationFailed`, or `NullPointer` if
    /// `AddResetCallback` refuses it.
    ///
    /// # Safety
    ///
    /// `code` and `data` must stay valid until this is dropped (or for the
    /// rest of the machine's life if it is forgotten).
    #[track_caller]
    pub unsafe fn add(pri: i8, name: &'static [u8], code: ServerFn, data: *mut c_void) -> Result<Self> {
        let node = alloc_node(pri, name, code as usize, data)?;
        // SAFETY: a fresh node; `code` and `data` meet the caller's contract.
        if unsafe { sys::exec_add_reset_callback(node) } == 0 {
            // SAFETY: the node was refused, so nothing else holds it.
            unsafe { sys::exec_free_sys_object(sys::ASOT_INTERRUPT, node as sys::APTR) };
            return Err(AmigaError::NullPointer);
        }
        Ok(Self { node, _t: crate::track::register("reset callback", node as usize, 0, release_reset_callback) })
    }
}

impl Drop for ResetCallback {
    fn drop(&mut self) {
        // SAFETY: the node was added in add().
        unsafe {
            sys::exec_rem_reset_callback(self.node);
            sys::exec_free_sys_object(sys::ASOT_INTERRUPT, self.node as sys::APTR);
        }
    }
}

/// A low-memory handler: `(ExecBase, request, is_Data) -> action`, the
/// action being [`MEM_DID_NOTHING`](sys::MEM_DID_NOTHING),
/// [`MEM_ALL_DONE`](sys::MEM_ALL_DONE) (memory was freed: retry the
/// allocation, then go on to the next handler) or
/// [`MEM_TRY_AGAIN`](sys::MEM_TRY_AGAIN) (retry, and call this handler
/// again).
pub type MemHandlerFn =
    unsafe extern "C" fn(sysbase: *mut c_void, request: *const sys::MemHandlerData, data: *mut c_void) -> i32;

/// A low-memory handler (AddMemHandler, V39), removed (RemMemHandler) and
/// freed on drop.
///
/// From exec.doc AddMemHandler and RemMemHandler:
///
/// - exec calls the handlers, highest priority first, when an allocation
///   fails, so the handler can free memory it can spare (caches, pools)
///   and let the allocation be tried again. Positive priorities run before
///   the libraries are expunged (RAMLIB is a handler at priority 0),
///   negative ones after;
/// - it runs in the task whose allocation failed, under Forbid ("We must
///   not break forbid!!!"): it must not Wait, take a lock, or allocate (an
///   allocation failing is why it was called);
/// - it may be called before [`MemHandler::add`] returns, and until the
///   drop's RemMemHandler returns: what it reads must be in place first and
///   outlive the handler.
pub struct MemHandler {
    _t: crate::track::Tracked,
    node: *mut sys::Interrupt,
}

impl MemHandler {
    /// Allocate a node for `code` and add it to exec's low-memory handlers
    /// at `pri`. `name` must end in `\0`; by convention it says who owns it.
    ///
    /// # Errors
    ///
    /// `NotNulTerminated` for a bad name, `AllocationFailed` if the node
    /// cannot be allocated.
    ///
    /// # Safety
    ///
    /// `code` follows the rules above, and `data` stays valid for it until
    /// this is dropped.
    #[track_caller]
    pub unsafe fn add(pri: i8, name: &'static [u8], code: MemHandlerFn, data: *mut c_void) -> Result<Self> {
        let node = alloc_node(pri, name, code as usize, data)?;
        // SAFETY: a fresh, complete node; `code` and `data` meet the
        // caller's contract, so it may run at once.
        unsafe { sys::exec_add_mem_handler(node) };
        Ok(Self { node, _t: crate::track::register("low-memory handler", node as usize, 0, release_mem_handler) })
    }
}

impl Drop for MemHandler {
    fn drop(&mut self) {
        // SAFETY: the node was added in add(). RemMemHandler returns once
        // the handler can no longer be entered, so the node goes after it.
        unsafe {
            sys::exec_rem_mem_handler(self.node);
            sys::exec_free_sys_object(sys::ASOT_INTERRUPT, self.node as sys::APTR);
        }
    }
}

/// A software interrupt handler (exec.doc, Cause: `handler(unused, sysbase,
/// is_Data)`). It runs in interrupt context: no Wait, no locks, no
/// allocation; Signal and PutMsg are allowed.
pub type SoftIntFn = unsafe extern "C" fn(unused: i32, sysbase: *mut c_void, data: *mut c_void);

/// A software interrupt (ASOT_INTERRUPT with ASOINTR_SoftInt), run by
/// [`SoftInt::cause`]. Freed on drop.
pub struct SoftInt {
    node: *mut sys::Interrupt,
}

// SAFETY: the node is MEMF_PUBLIC exec memory; Cause may be called from any
// task.
unsafe impl Send for SoftInt {}
// SAFETY: as above: cause takes &self and only passes the node to exec.
unsafe impl Sync for SoftInt {}

impl SoftInt {
    /// A soft interrupt at `pri` (exec has -32, -16, 0, 16 and 32 only) that
    /// calls `code` with `data`. `name` must end in `\0`.
    ///
    /// # Safety
    ///
    /// `code` follows the interrupt rules above, and `data` stays valid for
    /// it until this is dropped.
    pub unsafe fn new(pri: i8, name: &'static [u8], code: SoftIntFn, data: *mut c_void) -> Result<Self> {
        if ![-32, -16, 0, 16, 32].contains(&pri) {
            return Err(AmigaError::Device("SoftInt priority must be -32, -16, 0, 16 or 32"));
        }
        let name_ptr = require_nul(name)?;
        let tags = [
            TagItem { ti_Tag: sys::ASOINTR_CODE, ti_Data: code as usize as u32 },
            TagItem { ti_Tag: sys::ASOINTR_DATA, ti_Data: data as usize as u32 },
            TagItem { ti_Tag: sys::ASOINTR_SOFT_INT, ti_Data: 1 },
            TagItem { ti_Tag: sys::ASOINTR_PRI, ti_Data: pri as i32 as u32 },
            TagItem { ti_Tag: sys::ASOINTR_NAME, ti_Data: name_ptr as usize as u32 },
            TagItem { ti_Tag: sys::ASOINTR_COPY_NAME, ti_Data: 1 },
            TagItem { ti_Tag: sys::TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a TAG_DONE-ended tag list; the name is copied.
        let node = unsafe { sys::exec_alloc_sys_object(sys::ASOT_INTERRUPT, tags.as_ptr()) } as *mut sys::Interrupt;
        if node.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(Self { node }) }
    }

    /// Run the handler as a software interrupt (Cause). From a task it runs
    /// before Cause returns.
    pub fn cause(&self) {
        // SAFETY: our initialised soft interrupt node.
        unsafe { sys::exec_cause(self.node) }
    }
}

impl Drop for SoftInt {
    fn drop(&mut self) {
        // SAFETY: our node, freed once. A Cause from a task has run its
        // handler before returning, so nothing is pending from this value.
        unsafe { sys::exec_free_sys_object(sys::ASOT_INTERRUPT, self.node as sys::APTR) }
    }
}
