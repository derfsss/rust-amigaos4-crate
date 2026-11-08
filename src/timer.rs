//! timer.device and its ITimer interface, safe (feature `timer`).
//!
//! [`AmigaTimer`] is an open timer.device unit (built on `devio`):
//! synchronous [`delay`](AmigaTimer::delay), and one request that can be
//! [`start`](AmigaTimer::start)ed and waited for with other signals. A
//! delay runs on a request of its own with its own reply port, so it may
//! be used while a started request is pending without disturbing it.
//!
//! Time comes from the unit's ITimer (GetSysTime, GetUpTime, ReadEClock),
//! no I/O. The free functions [`get_up_time`], [`sys_time`], [`eclock`]
//! and [`micro_delay`] use the program's `ITimer` when it is set, else one
//! the crate opens itself (timer.device opened once, never closed).
//!
//! Units: MicroHz, VBlank and WaitUntil take a [`TimerVal`] (WaitUntil an
//! absolute system time); EClock counts E-clock ticks, so a `TimerVal`
//! given to it is converted with the E-clock frequency (or use
//! [`delay_ticks`](AmigaTimer::delay_ticks)).
//!
//! # Example
//!
//! ```ignore
//! use amigaos4::timer::{AmigaTimer, TimerUnit, TimerVal};
//!
//! let mut timer = AmigaTimer::open(TimerUnit::MicroHz)?;
//! timer.delay(TimerVal::from_millis(500))?;
//! let up = timer.up_time();
//! ```

use core::cmp::Ordering;

use crate::devio::Device;
use crate::error::{AmigaError, Result};
use crate::port::AmigaMsgPort;
use crate::sys::{self, EClockVal, TimeVal, TimerIFace};

// ---------------------------------------------------------------------------
// TimerVal
// ---------------------------------------------------------------------------

/// A time value in seconds + microseconds (always normalised:
/// `micros < 1_000_000`, which timer.device requires).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TimerVal {
    /// Whole seconds.
    pub secs: u32,
    /// Microsecond fractional part (0..999_999).
    pub micros: u32,
}

impl TimerVal {
    /// Zero time value.
    pub const ZERO: TimerVal = TimerVal { secs: 0, micros: 0 };

    /// From seconds and microseconds; microseconds of a second or more
    /// carry into the seconds.
    pub fn new(secs: u32, micros: u32) -> Self {
        Self { secs: secs.saturating_add(micros / 1_000_000), micros: micros % 1_000_000 }
    }

    /// From milliseconds.
    pub fn from_millis(ms: u32) -> Self {
        Self { secs: ms / 1_000, micros: (ms % 1_000) * 1_000 }
    }

    /// From microseconds.
    pub fn from_micros(us: u64) -> Self {
        Self { secs: (us / 1_000_000).min(u32::MAX as u64) as u32, micros: (us % 1_000_000) as u32 }
    }

    /// Total milliseconds.
    pub fn as_millis(&self) -> u64 {
        self.secs as u64 * 1_000 + self.micros as u64 / 1_000
    }

    /// Total microseconds.
    pub fn as_micros(&self) -> u64 {
        self.secs as u64 * 1_000_000 + self.micros as u64
    }

    /// self + o (AddTime), None on overflow.
    pub fn checked_add(&self, o: TimerVal) -> Option<TimerVal> {
        let us = self.micros + o.micros;
        let s = self.secs.checked_add(o.secs)?.checked_add(us / 1_000_000)?;
        Some(TimerVal { secs: s, micros: us % 1_000_000 })
    }

    /// self - o (SubTime), None if o is later.
    pub fn checked_sub(&self, o: TimerVal) -> Option<TimerVal> {
        self.as_micros().checked_sub(o.as_micros()).map(TimerVal::from_micros)
    }
}

impl PartialOrd for TimerVal {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for TimerVal {
    /// Earlier is less (ITimer's CmpTime has the opposite sign).
    fn cmp(&self, o: &Self) -> Ordering {
        (self.secs, self.micros).cmp(&(o.secs, o.micros))
    }
}

/// A ReadEClock reading: the E-clock count and its frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EClock {
    pub ticks: u64,
    /// Ticks per second.
    pub freq: u32,
}

impl EClock {
    /// Microseconds from `earlier` to self.
    pub fn micros_since(&self, earlier: &EClock) -> u64 {
        let f = self.freq.max(1) as u128;
        (self.ticks.wrapping_sub(earlier.ticks) as u128 * 1_000_000 / f) as u64
    }
}

// ---------------------------------------------------------------------------
// Units, constants
// ---------------------------------------------------------------------------

/// Timer device unit for [`AmigaTimer::open`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerUnit {
    /// Microsecond precision. May drift under load.
    MicroHz = 0,
    /// VBlank precision (~1/60 s). Stable, low overhead.
    VBlank = 1,
    /// E-clock ticks (system-dependent rate).
    EClock = 2,
    /// Wait until an absolute system time.
    WaitUntil = 3,
}

const TIMER_NAME: &[u8] = b"timer.device\0";

/// devices/timer.h commands.
const TR_ADDREQUEST: u16 = 9;

/// struct TimeRequest: IORequest (32) + TimeVal or EClockVal (8).
const TIME_REQUEST_SIZE: u32 = 40;

// ---------------------------------------------------------------------------
// ITimer
// ---------------------------------------------------------------------------

/// ITimer: the program's global, else timer.device opened by the crate.
fn itimer() -> Option<*mut TimerIFace> {
    static SLOT: crate::iface::DeviceSlot = crate::iface::DeviceSlot::new(TIMER_NAME, TimerUnit::VBlank as u32);
    // SAFETY: reading the program's interface global (set once at
    // start-up, or NULL).
    let global = unsafe { sys::ITimer } as *mut sys::Interface;
    SLOT.get(global).map(|i| i as *mut TimerIFace)
}

fn sys_time_with(i: *mut TimerIFace) -> TimerVal {
    let mut tv = TimeVal { Seconds: 0, Microseconds: 0 };
    // SAFETY: an open ITimer and a TimeVal to fill.
    unsafe { ((*i).GetSysTime)(i, &mut tv) };
    TimerVal { secs: tv.Seconds, micros: tv.Microseconds }
}

fn up_time_with(i: *mut TimerIFace) -> TimerVal {
    let mut tv = TimeVal { Seconds: 0, Microseconds: 0 };
    // SAFETY: as above.
    unsafe { ((*i).GetUpTime)(i, &mut tv) };
    TimerVal { secs: tv.Seconds, micros: tv.Microseconds }
}

fn eclock_with(i: *mut TimerIFace) -> EClock {
    let mut ev = EClockVal { ev_hi: 0, ev_lo: 0 };
    // SAFETY: an open ITimer and an EClockVal to fill.
    let freq = unsafe { ((*i).ReadEClock)(i, &mut ev) };
    EClock { ticks: ((ev.ev_hi as u64) << 32) | ev.ev_lo as u64, freq }
}

/// GetUpTime: time since the system started; never set, so monotonic.
/// ZERO if timer.device cannot be reached (it is in the Kickstart).
pub fn get_up_time() -> TimerVal {
    itimer().map(up_time_with).unwrap_or(TimerVal::ZERO)
}

/// GetSysTime: the system time (starts at power-on, may be set from the
/// battery clock or by TR_SETSYSTIME: not monotonic; use
/// [`get_up_time`] to measure).
pub fn sys_time() -> TimerVal {
    itimer().map(sys_time_with).unwrap_or(TimerVal::ZERO)
}

/// ReadEClock.
pub fn eclock() -> Option<EClock> {
    itimer().map(eclock_with)
}

/// MicroDelay: busy-wait `microseconds` (short waits only: the task
/// spins, using the CPU).
pub fn micro_delay(microseconds: u32) {
    if let Some(i) = itimer() {
        // SAFETY: an open ITimer; MicroDelay only spins.
        unsafe { ((*i).MicroDelay)(i, microseconds) };
    }
}

// ---------------------------------------------------------------------------
// AmigaTimer
// ---------------------------------------------------------------------------

/// An open timer.device unit; everything is released on drop (a started
/// request is aborted first).
pub struct AmigaTimer {
    // Field order is drop order: the interface before the device.
    iface: TimerInterface,
    // The synchronous request's own reply port.
    sync_port: AmigaMsgPort,
    dev: Device,
    unit: TimerUnit,
}

/// ITimer from our own open device; DropInterface on drop.
struct TimerInterface(*mut TimerIFace);

impl Drop for TimerInterface {
    fn drop(&mut self) {
        // SAFETY: from GetInterface on our device, dropped once, before
        // the device closes (field order).
        unsafe { sys::exec_drop_interface(self.0 as *mut sys::Interface) };
    }
}

impl AmigaTimer {
    /// Open timer.device's `unit`.
    pub fn open(unit: TimerUnit) -> Result<Self> {
        let dev = Device::open(TIMER_NAME, unit as u32, 0, TIME_REQUEST_SIZE)?;
        // SAFETY: the open device's base; GetInterface with a NUL-terminated
        // name.
        let i = unsafe { sys::exec_get_interface(dev.base(), c"main".as_ptr() as *const u8, 1, core::ptr::null()) } as *mut TimerIFace;
        if i.is_null() {
            return Err(AmigaError::NullPointer);
        }
        let iface = TimerInterface(i);
        let sync_port = AmigaMsgPort::new()?;
        Ok(Self { iface, sync_port, dev, unit })
    }

    /// The 8 bytes after the IORequest for `d` on this unit.
    fn time_bytes(&self, d: TimerVal) -> [u8; 8] {
        let (a, b) = match self.unit {
            TimerUnit::EClock => {
                let e = eclock_with(self.iface.0);
                let t = (d.as_micros() as u128 * e.freq as u128 / 1_000_000) as u64;
                ((t >> 32) as u32, t as u32)
            }
            _ => (d.secs, d.micros),
        };
        let mut v = [0u8; 8];
        v[..4].copy_from_slice(&a.to_be_bytes());
        v[4..].copy_from_slice(&b.to_be_bytes());
        v
    }

    fn sync(&mut self, bytes: [u8; 8]) -> Result<()> {
        let mut r = self.dev.new_request_on(&self.sync_port)?;
        r.set_command(TR_ADDREQUEST);
        r.extra_mut()[..8].copy_from_slice(&bytes);
        r.do_io().map_err(|e| match e {
            AmigaError::IoErr(c) => AmigaError::IoError(c as i32),
            other => other,
        })
    }

    /// Wait `duration` (TR_ADDREQUEST + DoIO; for WaitUntil: until that
    /// system time). A started request is left running.
    pub fn delay(&mut self, duration: TimerVal) -> Result<()> {
        let b = self.time_bytes(TimerVal::new(duration.secs, duration.micros));
        self.sync(b)
    }

    /// Wait `ticks` E-clock ticks (an EClock unit; other units refuse).
    pub fn delay_ticks(&mut self, ticks: u64) -> Result<()> {
        if self.unit != TimerUnit::EClock {
            return Err(AmigaError::InvalidArgument("delay_ticks needs the EClock unit"));
        }
        let mut b = [0u8; 8];
        b[..4].copy_from_slice(&((ticks >> 32) as u32).to_be_bytes());
        b[4..].copy_from_slice(&(ticks as u32).to_be_bytes());
        self.sync(b)
    }

    /// The system time (ITimer GetSysTime, no I/O).
    pub fn get_sys_time(&mut self) -> TimerVal {
        sys_time_with(self.iface.0)
    }

    /// Time since start-up (GetUpTime): monotonic.
    pub fn up_time(&self) -> TimerVal {
        up_time_with(self.iface.0)
    }

    /// ReadEClock.
    pub fn eclock(&self) -> EClock {
        eclock_with(self.iface.0)
    }

    /// Start a TR_ADDREQUEST for `duration` without waiting (SendIO). When
    /// it completes the timer's port signals the task that opened it: wait
    /// for [`signal_mask`](Self::signal_mask) with other signals, then
    /// [`is_done`](Self::is_done). Collect it with
    /// [`finish`](Self::finish) or cancel it with [`abort`](Self::abort);
    /// starting again cancels the one pending, and dropping the timer
    /// aborts it.
    pub fn start(&mut self, duration: TimerVal) {
        let b = self.time_bytes(TimerVal::new(duration.secs, duration.micros));
        let r = self.dev.request();
        r.set_command(TR_ADDREQUEST);
        r.extra_mut()[..8].copy_from_slice(&b);
        r.start();
    }

    /// The signal a started request raises when it completes.
    #[inline]
    pub fn signal_mask(&self) -> u32 {
        self.dev.signal_mask()
    }

    /// True once a started request has completed (CheckIO); also when
    /// none is pending.
    pub fn is_done(&self) -> bool {
        self.dev.request_ref().check()
    }

    /// Collect a started request (WaitIO; at once if it is done).
    pub fn finish(&mut self) {
        let _ = self.dev.request().finish();
    }

    /// Cancel a started request (AbortIO then WaitIO), and clear its
    /// signal: the aborted request's reply sets it, and WaitIO takes the
    /// message but leaves the signal, so the next Wait on it would return
    /// at once. A loop that aborts and restarts the timer on every other
    /// wakeup then never sleeps (a network stack's process spun at 100%
    /// CPU that way). The request's port is the timer's own, so
    /// no other reply's signal is lost.
    pub fn abort(&mut self) {
        self.dev.request().cancel_pending();
        crate::signal::take(self.signal_mask());
    }
}

// ---------------------------------------------------------------------------
// Waiting with a time limit
// ---------------------------------------------------------------------------

/// How a [`Waiter::wait`] ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waited {
    /// These of the asked-for signals came (and are cleared).
    Signals(u32),
    /// The time ran out first.
    TimedOut,
}

/// Waits on signals with a time limit, ended by Ctrl-C: a timer request of
/// its own, kept for repeated waits.
pub struct Waiter {
    timer: AmigaTimer,
}

impl Waiter {
    pub fn new() -> Result<Self> {
        Ok(Self { timer: AmigaTimer::open(TimerUnit::MicroHz)? })
    }

    /// Wait for any of `mask`, at most `timeout` (None: no limit). Signals
    /// already set count at once. Ctrl-C ends the wait with
    /// `Err(Cancelled)` (and is cleared), whether or not `mask` names it.
    pub fn wait(&mut self, mask: u32, timeout: Option<TimerVal>) -> Result<Waited> {
        let brk = crate::signal::Cancel::MASK;
        let now = crate::signal::take(mask | brk);
        if now & brk != 0 {
            return Err(AmigaError::Cancelled);
        }
        if now & mask != 0 {
            return Ok(Waited::Signals(now & mask));
        }
        if let Some(t) = timeout {
            if t.secs == 0 && t.micros == 0 {
                return Ok(Waited::TimedOut);
            }
            self.timer.start(t);
        }
        let tmask = if timeout.is_some() { self.timer.signal_mask() } else { 0 };
        loop {
            let got = crate::signal::wait(mask | brk | tmask);
            if got & (mask | brk) != 0 {
                if timeout.is_some() {
                    self.timer.abort();
                }
                return if got & brk != 0 { Err(AmigaError::Cancelled) } else { Ok(Waited::Signals(got & mask)) };
            }
            if timeout.is_some() && self.timer.is_done() {
                self.timer.finish();
                return Ok(Waited::TimedOut);
            }
        }
    }

    /// As [`wait`](Self::wait) with the limit in milliseconds.
    pub fn wait_ms(&mut self, mask: u32, ms: u32) -> Result<Waited> {
        self.wait(mask, Some(TimerVal::from_millis(ms)))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_unit_values() {
        assert_eq!(TimerUnit::MicroHz as u32, 0);
        assert_eq!(TimerUnit::VBlank as u32, 1);
        assert_eq!(TimerUnit::EClock as u32, 2);
        assert_eq!(TimerUnit::WaitUntil as u32, 3);
    }

    #[test]
    fn timer_name_null_terminated() {
        assert_eq!(TIMER_NAME.last(), Some(&0u8));
    }

    #[test]
    fn timer_val_normalises() {
        assert_eq!(TimerVal::new(1, 2_500_000), TimerVal { secs: 3, micros: 500_000 });
        assert_eq!(TimerVal::from_millis(1500), TimerVal { secs: 1, micros: 500_000 });
        assert_eq!(TimerVal::new(2, 500_000).as_millis(), 2500);
        assert_eq!(TimerVal::from_micros(3_000_001), TimerVal { secs: 3, micros: 1 });
    }

    #[test]
    fn timer_val_arithmetic() {
        let a = TimerVal::new(1, 999_999);
        assert_eq!(a.checked_add(TimerVal::new(0, 1)), Some(TimerVal::new(2, 0)));
        assert_eq!(TimerVal::new(2, 0).checked_sub(TimerVal::new(0, 1)), Some(TimerVal::new(1, 999_999)));
        assert_eq!(TimerVal::new(0, 1).checked_sub(TimerVal::new(0, 2)), None);
        assert_eq!(TimerVal::new(u32::MAX, 0).checked_add(TimerVal::new(1, 0)), None);
        assert!(TimerVal::new(1, 0) > TimerVal::new(0, 999_999));
    }

    #[test]
    fn eclock_elapsed() {
        let a = EClock { ticks: 1_000, freq: 1_000_000 };
        let b = EClock { ticks: 251_000, freq: 1_000_000 };
        assert_eq!(b.micros_since(&a), 250_000);
    }
}
