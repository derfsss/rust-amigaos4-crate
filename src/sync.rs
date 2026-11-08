//! Locks for data shared between tasks, without Forbid.
//!
//! - [`Mutex<T>`]: an exec mutex (AllocSysObject ASOT_MUTEX, V52), created
//!   NOT recursive. exec's default is recursive, which would let one task
//!   hold two `&mut T`. MutexRelease "makes no ownership test", so the only
//!   release is the guard's drop.
//! - [`Semaphore<T>`]: a SignalSemaphore (ASOT_SEMAPHORE) as a reader-writer
//!   lock: [`write`](Semaphore::write) exclusive, [`read`](Semaphore::read)
//!   shared. SignalSemaphores always nest for their exclusive owner (and,
//!   from V39, grant the owner a shared lock too), which would alias the
//!   `&mut T`. Both calls therefore refuse with
//!   [`AmigaError::WouldDeadlock`] when the calling task holds the
//!   exclusive lock.
//!
//! [`Mutex`] tracks its owner itself: a second lock by the holding task
//! returns [`AmigaError::WouldDeadlock`] (`try_lock` answers `None`).
//! exec cannot be relied on there: MutexObtain on a non-recursive mutex the
//! caller holds deadlocks, and MutexAttemptWithSignal GRANTS it again (it
//! returned 0 with one of its signals already set: exec 54.57 on the QEMU
//! AmigaOne, testprogs/exectest), which would be two `&mut T`.
//!
//! Not caught (documented by exec.doc, undetectable from the semaphore):
//! a task that holds a shared lock and asks for the exclusive one waits
//! forever.
//!
//! A [`Semaphore`] can also be asked for without blocking (Procure, V39):
//! [`bid_write`](Semaphore::bid_write) and [`bid_read`](Semaphore::bid_read)
//! post a bid and return at once. The bid has a message port of its own,
//! whose [`signal_mask`](WriteBid::signal_mask) can join a Wait on other
//! signals; when exec grants the lock it replies the bid, and
//! [`take`](WriteBid::take) or [`wait`](WriteBid::wait) turn it into the
//! same guard `write`/`read` give. Dropping a bid that was not taken
//! cancels it (Vacate): a pending one is withdrawn, a granted one released.
//!
//! Both objects are MEMF_SHARED (exec's choice for them) and untracked
//! (ASO_NoTrack, exec's default), so the task that made one may end while
//! others use it. They are for tasks: never lock from an interrupt.

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::error::{AmigaError, Result};
use crate::port::AmigaMsgPort;
use crate::signal::current_task;
use crate::sys::{self, SemaphoreMessage, SignalSemaphore, Task, TagItem, APTR, TAG_DONE};

// ---------------------------------------------------------------------------
// Mutex
// ---------------------------------------------------------------------------

/// Data behind an exec mutex.
pub struct Mutex<T: ?Sized> {
    mx: APTR,
    /// The task holding it, or null: set after the lock is taken, cleared
    /// before it is released, so only the holder can see itself here.
    owner: AtomicPtr<Task>,
    data: UnsafeCell<T>,
}

// SAFETY: the mutex owns its T; moving it moves the T (hence T: Send).
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
// SAFETY: the mutex hands `&mut T` to one task at a time, so sharing it
// moves access to T between tasks (T: Send), never shares it.
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// A new, unlocked, non-recursive mutex around `value`.
    pub fn new(value: T) -> Result<Self> {
        let tags = [
            TagItem { ti_Tag: sys::ASOMUTEX_RECURSIVE, ti_Data: 0 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a valid tag list.
        let mx = unsafe { sys::exec_alloc_sys_object(sys::ASOT_MUTEX, tags.as_ptr()) };
        if mx.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self { mx, owner: AtomicPtr::new(null_mut()), data: UnsafeCell::new(value) })
    }

    /// The value, taking the mutex apart.
    pub fn into_inner(self) -> T {
        let me = core::mem::ManuallyDrop::new(self);
        // SAFETY: `me` is never used or dropped again; the mutex is freed here.
        unsafe {
            sys::exec_free_sys_object(sys::ASOT_MUTEX, me.mx);
            core::ptr::read(me.data.get())
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Refuse a lock by the task that holds it (exec would deadlock it, or,
    /// for MutexAttemptWithSignal, grant it: two `&mut T`).
    fn check_not_owner(&self) -> Result<()> {
        if self.owner.load(Ordering::Acquire) == current_task() {
            Err(AmigaError::WouldDeadlock)
        } else {
            Ok(())
        }
    }

    /// The lock is taken: record the owner, make the guard.
    fn guard(&self) -> MutexGuard<'_, T> {
        self.owner.store(current_task(), Ordering::Release);
        MutexGuard { m: self, _not_send: PhantomData }
    }

    /// Lock, waiting as long as it takes. `WouldDeadlock` when the calling
    /// task holds it already.
    pub fn lock(&self) -> Result<MutexGuard<'_, T>> {
        self.check_not_owner()?;
        // SAFETY: `mx` is our mutex, alive while `self` is.
        unsafe { sys::exec_mutex_obtain(self.mx) };
        Ok(self.guard())
    }

    /// Lock if it is free now (None when anyone holds it, this task too).
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        self.check_not_owner().ok()?;
        // SAFETY: as in lock. MutexAttempt returns a BOOL.
        if unsafe { sys::exec_mutex_attempt(self.mx) } != 0 {
            Some(self.guard())
        } else {
            None
        }
    }

    /// Lock, or give up when one of `signals` arrives: `Ok(Err(signals
    /// that arrived))`. `WouldDeadlock` when the calling task holds it.
    pub fn lock_or_signal(&self, signals: u32) -> Result<core::result::Result<MutexGuard<'_, T>, u32>> {
        self.check_not_owner()?;
        // SAFETY: as in lock. Returns the preempting mask, 0 = locked.
        let r = unsafe { sys::exec_mutex_attempt_with_signal(self.mx, signals) };
        Ok(if r == 0 { Ok(self.guard()) } else { Err(r) })
    }

    /// The value through `&mut self`: no locking needed.
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

impl<T: ?Sized> Drop for Mutex<T> {
    fn drop(&mut self) {
        // SAFETY: no guard can outlive `self`, so nobody holds it.
        unsafe { sys::exec_free_sys_object(sys::ASOT_MUTEX, self.mx) }
    }
}

/// A held [`Mutex`]; released on drop, by the task that locked it.
pub struct MutexGuard<'a, T: ?Sized> {
    m: &'a Mutex<T>,
    _not_send: PhantomData<*mut ()>,
}

// SAFETY: sharing a guard shares `&T` only, which needs T: Sync.
unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: the lock is held.
        unsafe { &*self.m.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the lock is held, and it is not recursive.
        unsafe { &mut *self.m.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.m.owner.store(null_mut(), Ordering::Release);
        // SAFETY: this guard is the one lock on it.
        unsafe { sys::exec_mutex_release(self.m.mx) }
    }
}

// ---------------------------------------------------------------------------
// Semaphore
// ---------------------------------------------------------------------------

/// Data behind a SignalSemaphore, read shared or written exclusive.
pub struct Semaphore<T: ?Sized> {
    ss: *mut SignalSemaphore,
    data: UnsafeCell<T>,
}

// SAFETY: the semaphore owns its T; moving it moves the T.
unsafe impl<T: ?Sized + Send> Send for Semaphore<T> {}
// SAFETY: readers on several tasks share `&T` (T: Sync); a writer gets
// `&mut T` alone, moving access between tasks (T: Send).
unsafe impl<T: ?Sized + Send + Sync> Sync for Semaphore<T> {}

impl<T> Semaphore<T> {
    /// A new, private (unnamed) semaphore around `value`.
    pub fn new(value: T) -> Result<Self> {
        let tags = [TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: a valid tag list; exec initialises it (InitSemaphore).
        let ss = unsafe { sys::exec_alloc_sys_object(sys::ASOT_SEMAPHORE, tags.as_ptr()) };
        if ss.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        Ok(Self { ss: ss as *mut SignalSemaphore, data: UnsafeCell::new(value) })
    }

    pub fn into_inner(self) -> T {
        let me = core::mem::ManuallyDrop::new(self);
        // SAFETY: `me` is never used or dropped again.
        unsafe {
            sys::exec_free_sys_object(sys::ASOT_SEMAPHORE, me.ss as APTR);
            core::ptr::read(me.data.get())
        }
    }
}

impl<T: ?Sized> Semaphore<T> {
    /// ss_Owner. Only its equality with the caller is meaningful: only the
    /// caller can make it equal to itself.
    fn owner(&self) -> *mut Task {
        // SAFETY: ss_Owner is a pointer at SS_OWNER_OFFSET (checked against
        // the SDK); a volatile read, since other tasks change it.
        unsafe { core::ptr::read_volatile((self.ss as *const u8).add(sys::SS_OWNER_OFFSET) as *const *mut Task) }
    }

    fn check_not_owner(&self) -> Result<()> {
        if self.owner() == current_task() {
            Err(AmigaError::WouldDeadlock)
        } else {
            Ok(())
        }
    }

    /// Exclusive lock, waiting as long as it takes.
    pub fn write(&self) -> Result<WriteGuard<'_, T>> {
        self.check_not_owner()?;
        // SAFETY: our semaphore, alive while `self` is.
        unsafe { sys::exec_obtain_semaphore(self.ss) };
        Ok(WriteGuard { s: self, _not_send: PhantomData })
    }

    /// Shared lock, waiting as long as it takes. Do not ask for
    /// [`write`](Self::write) while holding one: that waits forever.
    pub fn read(&self) -> Result<ReadGuard<'_, T>> {
        self.check_not_owner()?;
        // SAFETY: as in write.
        unsafe { sys::exec_obtain_semaphore_shared(self.ss) };
        Ok(ReadGuard { s: self, _not_send: PhantomData })
    }

    /// Exclusive lock if free now.
    pub fn try_write(&self) -> Result<Option<WriteGuard<'_, T>>> {
        self.check_not_owner()?;
        // SAFETY: as in write. AttemptSemaphore returns a BOOL.
        Ok((unsafe { sys::exec_attempt_semaphore(self.ss) } != 0)
            .then(|| WriteGuard { s: self, _not_send: PhantomData }))
    }

    /// Shared lock if no one holds it exclusively now.
    pub fn try_read(&self) -> Result<Option<ReadGuard<'_, T>>> {
        self.check_not_owner()?;
        // SAFETY: as in write. AttemptSemaphoreShared returns a BOOL.
        Ok((unsafe { sys::exec_attempt_semaphore_shared(self.ss) } != 0)
            .then(|| ReadGuard { s: self, _not_send: PhantomData }))
    }

    /// Exclusive lock, or `Ok(Err(signals that arrived))` when one of
    /// `signals` comes first (V53.77).
    pub fn write_or_signal(&self, signals: u32) -> Result<core::result::Result<WriteGuard<'_, T>, u32>> {
        self.check_not_owner()?;
        // SAFETY: as in write. Returns the preempting mask, 0 = locked.
        let r = unsafe { sys::exec_attempt_semaphore_with_signal(self.ss, signals) };
        Ok(if r == 0 { Ok(WriteGuard { s: self, _not_send: PhantomData }) } else { Err(r) })
    }

    /// Shared lock, or `Ok(Err(signals that arrived))` (V53.77).
    pub fn read_or_signal(&self, signals: u32) -> Result<core::result::Result<ReadGuard<'_, T>, u32>> {
        self.check_not_owner()?;
        // SAFETY: as in write.
        let r = unsafe { sys::exec_attempt_semaphore_shared_with_signal(self.ss, signals) };
        Ok(if r == 0 { Ok(ReadGuard { s: self, _not_send: PhantomData }) } else { Err(r) })
    }

    /// Bid for the exclusive lock without waiting (Procure). `WouldDeadlock`
    /// when the calling task holds the exclusive lock (exec would nest it).
    pub fn bid_write(&self) -> Result<WriteBid<'_, T>> {
        self.check_not_owner()?;
        Ok(WriteBid { s: self, core: BidCore::post(self.ss, sys::SM_EXCLUSIVE)? })
    }

    /// Bid for a shared lock without waiting (Procure). `WouldDeadlock` when
    /// the calling task holds the exclusive lock (exec would grant it).
    pub fn bid_read(&self) -> Result<ReadBid<'_, T>> {
        self.check_not_owner()?;
        Ok(ReadBid { s: self, core: BidCore::post(self.ss, sys::SM_SHARED)? })
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

/// A posted Procure: the bid message and the port it is replied to.
struct BidCore {
    ss: *mut SignalSemaphore,
    port: AmigaMsgPort,
    msg: *mut SemaphoreMessage,
    /// The reply has been taken off the port: the lock is ours.
    granted: bool,
    /// Turned into a guard: the guard releases the lock, not Vacate.
    handed_over: bool,
}

impl BidCore {
    fn post(ss: *mut SignalSemaphore, kind: u32) -> Result<Self> {
        let port = AmigaMsgPort::new()?;
        let tags = [
            TagItem { ti_Tag: sys::ASOMSG_SIZE, ti_Data: sys::SEMAPHORE_MESSAGE_SIZE as u32 },
            TagItem { ti_Tag: sys::ASOMSG_REPLYPORT, ti_Data: port.as_ptr() as u32 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: a valid tag list; the reply port lives as long as the bid.
        let msg = unsafe { sys::exec_alloc_sys_object(sys::ASOT_MESSAGE, tags.as_ptr()) } as *mut SemaphoreMessage;
        if msg.is_null() {
            return Err(AmigaError::AllocationFailed);
        }
        // SAFETY: a fresh 24-byte message. The kind is ln_Name's value (a
        // number, not a pointer: semaphores.h, SM_SHARED/SM_EXCLUSIVE) at
        // offset 10 (tests/layout/exec.c); ssm_Semaphore is exec's to set.
        // Procure gets our semaphore, alive while the bid's borrow is.
        unsafe {
            core::ptr::write_unaligned((msg as *mut u8).add(10) as *mut u32, kind);
            core::ptr::write_unaligned((msg as *mut u8).add(sys::SSM_SEMAPHORE_OFFSET) as *mut u32, 0);
            sys::exec_procure(ss, msg);
        }
        Ok(Self { ss, port, msg, granted: false, handed_over: false })
    }

    /// Take the reply if it has come: true once the lock is ours.
    fn poll(&mut self) -> bool {
        if !self.granted && self.port.get_raw().is_some() {
            // The only message this port ever receives is our bid, and it
            // is replied with ssm_Semaphore set unless Vacate cancelled it,
            // which only drop does.
            self.granted = true;
        }
        self.granted
    }

    fn wait(&mut self) {
        while !self.poll() {
            self.port.wait();
        }
    }
}

impl Drop for BidCore {
    fn drop(&mut self) {
        if !self.handed_over {
            // SAFETY: our semaphore and bid. Vacate replies a pending bid
            // (ssm_Semaphore cleared) or, for one already granted, releases
            // the lock; either way the message reaches our port at most once.
            unsafe { sys::exec_vacate(self.ss, self.msg) };
            if !self.granted {
                // Pending (now replied by Vacate) or granted but not yet
                // taken: exactly one reply is on its way or queued.
                while self.port.get_raw().is_none() {
                    self.port.wait();
                }
            }
        }
        // SAFETY: the reply has been taken off the port (or the bid was
        // granted and taken before), so exec holds no reference to it.
        unsafe { sys::exec_free_sys_object(sys::ASOT_MESSAGE, self.msg as APTR) };
    }
}

/// A pending bid for a [`Semaphore`]'s exclusive lock.
pub struct WriteBid<'a, T: ?Sized> {
    s: &'a Semaphore<T>,
    core: BidCore,
}

/// A pending bid for a shared lock on a [`Semaphore`].
pub struct ReadBid<'a, T: ?Sized> {
    s: &'a Semaphore<T>,
    core: BidCore,
}

macro_rules! bid_methods {
    ($bid:ident, $guard:ident) => {
        impl<'a, T: ?Sized> $bid<'a, T> {
            /// The signal of the bid's port: set when the lock is granted.
            pub fn signal_mask(&self) -> u32 {
                self.core.port.signal_mask()
            }

            /// Whether the lock has been granted (it stays granted).
            pub fn is_granted(&mut self) -> bool {
                self.core.poll()
            }

            /// The guard if the lock has been granted, else the bid back.
            pub fn take(mut self) -> core::result::Result<$guard<'a, T>, Self> {
                if self.core.poll() {
                    self.core.handed_over = true;
                    Ok($guard { s: self.s, _not_send: PhantomData })
                } else {
                    Err(self)
                }
            }

            /// Wait until the lock is granted.
            pub fn wait(mut self) -> $guard<'a, T> {
                self.core.wait();
                self.core.handed_over = true;
                $guard { s: self.s, _not_send: PhantomData }
            }
        }
    };
}
bid_methods!(WriteBid, WriteGuard);
bid_methods!(ReadBid, ReadGuard);

impl<T: ?Sized> Drop for Semaphore<T> {
    fn drop(&mut self) {
        // SAFETY: no guard can outlive `self`.
        unsafe { sys::exec_free_sys_object(sys::ASOT_SEMAPHORE, self.ss as APTR) }
    }
}

/// A held exclusive lock.
pub struct WriteGuard<'a, T: ?Sized> {
    s: &'a Semaphore<T>,
    _not_send: PhantomData<*mut ()>,
}

/// A held shared lock.
pub struct ReadGuard<'a, T: ?Sized> {
    s: &'a Semaphore<T>,
    _not_send: PhantomData<*mut ()>,
}

// SAFETY: a shared guard gives `&T` only, which needs T: Sync.
unsafe impl<T: ?Sized + Sync> Sync for WriteGuard<'_, T> {}
// SAFETY: as above.
unsafe impl<T: ?Sized + Sync> Sync for ReadGuard<'_, T> {}

impl<T: ?Sized> Deref for WriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: held exclusively.
        unsafe { &*self.s.data.get() }
    }
}

impl<T: ?Sized> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: held exclusively, and the owner cannot lock again.
        unsafe { &mut *self.s.data.get() }
    }
}

impl<T: ?Sized> Deref for ReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: held shared: no writer.
        unsafe { &*self.s.data.get() }
    }
}

impl<T: ?Sized> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: one obtain, one release, by the locking task.
        unsafe { sys::exec_release_semaphore(self.s.ss) }
    }
}

impl<T: ?Sized> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: as above.
        unsafe { sys::exec_release_semaphore(self.s.ss) }
    }
}
