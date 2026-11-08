//! Owned sockets over [`SocketLib`] (feature `bsdsocket`), safe.
//!
//! [`Socket`] owns a socket of a `SocketLib` and closes it (CloseSocket)
//! on drop; [`TcpStream`], [`TcpListener`] and [`UdpSocket`] are typed
//! sockets on top. A socket borrows its library, and neither is `Send`:
//! Roadshow gives each opener its own base and socket table, so a socket
//! belongs to the task that opened the library.
//!
//! [`SocketLib::wait`] is WaitSelect: sockets ready to read or write and
//! exec signals in one call (what `std::net` cannot do). Errors are
//! `AmigaError::IoError(errno)` read from the library straight after the
//! call; [`would_block`] and [`interrupted`] classify them.
//!
//! Options with their own types: the timeouts take a `Duration` (a
//! `timeval`), linger an `Option<Duration>` (a `struct linger`):
//! [`SocketLib::set_opt`] refuses those names, since a 4-byte value is
//! wrong for them.

use core::ffi::c_void;
use core::marker::PhantomData;
use core::time::Duration;

use alloc::string::String;
use alloc::vec::Vec;

use crate::bsdsocket::{self as bs, SockAddrIn, SocketLib};
use crate::error::{AmigaError, Result};
use crate::sys::{self, APTR};

/// sys/filio.h: _IOW('f', 126, long), _IOR('f', 127, long).
pub const FIONBIO: u32 = 0x8004_667E;
pub const FIONREAD: u32 = 0x4004_667F;
/// sys/socket.h
pub const SO_KEEPALIVE: i32 = 0x0008;
pub const SO_BROADCAST: i32 = 0x0020;
pub const SO_LINGER: i32 = 0x0080;
pub const SO_ERROR: i32 = 0x1007;
/// FD_SETSIZE (sys/socket.h): the highest socket WaitSelect can watch, + 1.
pub const FD_SETSIZE: i32 = 256;
/// sys/errno.h
pub const EPIPE: i32 = 32;
pub const EADDRINUSE: i32 = 48;
pub const ECONNRESET: i32 = 54;

/// Whether `e` is EWOULDBLOCK/EAGAIN (a non-blocking socket had nothing).
pub fn would_block(e: &AmigaError) -> bool {
    matches!(e, AmigaError::IoError(c) if *c == bs::EWOULDBLOCK)
}

/// Whether `e` is EINTR (a break signal ended the call).
pub fn interrupted(e: &AmigaError) -> bool {
    matches!(e, AmigaError::IoError(c) if *c == bs::EINTR)
}

/// Roadshow's `struct timeval` (two longwords).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TimeVal {
    secs: u32,
    micros: u32,
}

impl TimeVal {
    fn from(d: Duration) -> Self {
        TimeVal { secs: d.as_secs().min(u32::MAX as u64) as u32, micros: d.subsec_micros() }
    }
}

/// `struct linger`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Linger {
    onoff: i32,
    linger: i32,
}

/// Which way to shut a connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shutdown {
    Read = 0,
    Write = 1,
    Both = 2,
}

/// A socket of `'l`'s library; CloseSocket on drop.
pub struct Socket<'l> {
    lib: &'l SocketLib,
    fd: i32,
    _p: PhantomData<*const ()>,
}

impl<'l> Socket<'l> {
    /// A new socket (`socket(domain, kind, protocol)`).
    pub fn new(lib: &'l SocketLib, domain: i32, kind: i32, protocol: i32) -> Result<Self> {
        let fd = lib.socket(domain, kind, protocol)?;
        Ok(Self { lib, fd, _p: PhantomData })
    }

    /// The socket number (for the library's raw calls).
    pub fn fd(&self) -> i32 {
        self.fd
    }

    fn i(&self) -> *mut sys::SocketIFace {
        self.lib.iface()
    }

    fn check(&self, r: i32) -> Result<i32> {
        if r < 0 { Err(AmigaError::IoError(self.lib.errno())) } else { Ok(r) }
    }

    pub fn local_addr(&self) -> Result<SockAddrIn> {
        self.lib.local_addr(self.fd)
    }

    /// getpeername.
    pub fn peer_addr(&self) -> Result<SockAddrIn> {
        let mut a = SockAddrIn::default();
        let mut len: u32 = 16;
        // SAFETY: our socket and a 16-byte sockaddr_in to fill.
        let r = unsafe { ((*self.i()).getpeername)(self.i(), self.fd, &mut a as *mut _ as *mut c_void, &mut len) };
        self.check(r).map(|_| a)
    }

    fn ioctl_long(&self, req: u32, v: &mut i32) -> Result<()> {
        // SAFETY: our socket; FIONBIO and FIONREAD take a pointer to a long.
        let r = unsafe { ((*self.i()).IoctlSocket)(self.i(), self.fd, req, v as *mut i32 as APTR) };
        self.check(r).map(|_| ())
    }

    /// FIONBIO: calls that would wait fail with EWOULDBLOCK instead.
    pub fn set_nonblocking(&self, on: bool) -> Result<()> {
        let mut v = on as i32;
        self.ioctl_long(FIONBIO, &mut v)
    }

    /// FIONREAD: bytes that can be read without waiting.
    pub fn bytes_available(&self) -> Result<usize> {
        let mut v = 0i32;
        self.ioctl_long(FIONREAD, &mut v).map(|_| v.max(0) as usize)
    }

    fn set_opt_bytes<T>(&self, level: i32, name: i32, v: &T) -> Result<()> {
        // SAFETY: our socket and a value of the option's own type and size.
        let r = unsafe {
            ((*self.i()).setsockopt)(self.i(), self.fd, level, name, v as *const T as APTR, core::mem::size_of::<T>() as u32)
        };
        self.check(r).map(|_| ())
    }

    fn set_flag(&self, level: i32, name: i32, on: bool) -> Result<()> {
        self.set_opt_bytes(level, name, &(on as i32))
    }

    pub fn set_reuse_addr(&self, on: bool) -> Result<()> {
        self.set_flag(bs::SOL_SOCKET, bs::SO_REUSEADDR, on)
    }

    pub fn set_keepalive(&self, on: bool) -> Result<()> {
        self.set_flag(bs::SOL_SOCKET, SO_KEEPALIVE, on)
    }

    pub fn set_broadcast(&self, on: bool) -> Result<()> {
        self.set_flag(bs::SOL_SOCKET, SO_BROADCAST, on)
    }

    /// TCP_NODELAY.
    pub fn set_nodelay(&self, on: bool) -> Result<()> {
        self.set_flag(bs::IPPROTO_TCP, bs::TCP_NODELAY, on)
    }

    /// SO_RCVTIMEO: a receive waits at most `d` (None: for ever).
    pub fn set_read_timeout(&self, d: Option<Duration>) -> Result<()> {
        self.set_opt_bytes(bs::SOL_SOCKET, bs::SO_RCVTIMEO, &d.map(TimeVal::from).unwrap_or_default())
    }

    /// SO_SNDTIMEO.
    pub fn set_write_timeout(&self, d: Option<Duration>) -> Result<()> {
        self.set_opt_bytes(bs::SOL_SOCKET, bs::SO_SNDTIMEO, &d.map(TimeVal::from).unwrap_or_default())
    }

    /// SO_LINGER: Some(d) lets a close wait up to `d` for unsent data.
    pub fn set_linger(&self, d: Option<Duration>) -> Result<()> {
        let l = Linger { onoff: d.is_some() as i32, linger: d.map(|d| d.as_secs().min(i32::MAX as u64) as i32).unwrap_or(0) };
        self.set_opt_bytes(bs::SOL_SOCKET, SO_LINGER, &l)
    }

    /// SO_ERROR: the pending error (cleared by reading it), if any.
    pub fn take_error(&self) -> Result<Option<i32>> {
        self.lib.get_opt(self.fd, bs::SOL_SOCKET, SO_ERROR).map(|e| (e != 0).then_some(e))
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.lib.shutdown(self.fd, how as i32)
    }
}

impl Drop for Socket<'_> {
    fn drop(&mut self) {
        let _ = self.lib.close(self.fd);
    }
}

/// A TCP connection.
pub struct TcpStream<'l> {
    pub sock: Socket<'l>,
}

impl<'l> TcpStream<'l> {
    /// Read some bytes; 0 when the peer has closed.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.sock.lib.recv(self.sock.fd, buf, 0)
    }

    /// Send some bytes; how many were taken.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.sock.lib.send(self.sock.fd, buf, 0)
    }

    pub fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.write(buf)? {
                0 => return Err(AmigaError::IoError(EPIPE)),
                n => buf = &buf[n..],
            }
        }
        Ok(())
    }

    pub fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.read(buf)? {
                0 => return Err(AmigaError::UnexpectedEof),
                n => buf = &mut buf[n..],
            }
        }
        Ok(())
    }
}

impl crate::io::Read for TcpStream<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        TcpStream::read(self, buf)
    }
}

impl crate::io::Write for TcpStream<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        TcpStream::write(self, buf)
    }
}

/// A listening TCP socket.
pub struct TcpListener<'l> {
    pub sock: Socket<'l>,
}

impl<'l> TcpListener<'l> {
    /// The next connection and its peer.
    pub fn accept(&self) -> Result<(TcpStream<'l>, SockAddrIn)> {
        let (fd, a) = self.sock.lib.accept(self.sock.fd)?;
        Ok((TcpStream { sock: Socket { lib: self.sock.lib, fd, _p: PhantomData } }, a))
    }

    pub fn local_addr(&self) -> Result<SockAddrIn> {
        self.sock.local_addr()
    }
}

/// A UDP socket.
pub struct UdpSocket<'l> {
    pub sock: Socket<'l>,
}

impl<'l> UdpSocket<'l> {
    /// sendto: the bytes sent.
    pub fn send_to(&self, buf: &[u8], to: &SockAddrIn) -> Result<usize> {
        let i = self.sock.i();
        // SAFETY: our socket, `buf` readable for its length, a 16-byte
        // sockaddr_in.
        let r = unsafe { ((*i).sendto)(i, self.sock.fd, buf.as_ptr() as APTR, buf.len() as i32, 0, to as *const _ as *mut c_void, 16) };
        self.sock.check(r).map(|n| n as usize)
    }

    /// recvfrom: the bytes received and the sender.
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SockAddrIn)> {
        let i = self.sock.i();
        let mut a = SockAddrIn::default();
        let mut len: u32 = 16;
        // SAFETY: our socket, `buf` writable for its length, a sockaddr_in
        // to fill.
        let r = unsafe {
            ((*i).recvfrom)(i, self.sock.fd, buf.as_mut_ptr() as APTR, buf.len() as i32, 0, &mut a as *mut _ as *mut c_void, &mut len)
        };
        self.sock.check(r).map(|n| (n as usize, a))
    }

    /// connect: a default peer for send/recv.
    pub fn connect(&self, to: &SockAddrIn) -> Result<()> {
        self.sock.lib.connect(self.sock.fd, to)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        self.sock.lib.send(self.sock.fd, buf, 0)
    }

    pub fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        self.sock.lib.recv(self.sock.fd, buf, 0)
    }

    pub fn local_addr(&self) -> Result<SockAddrIn> {
        self.sock.local_addr()
    }
}

/// What WaitSelect reported.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ready {
    /// One flag per socket in `reads`, in order.
    pub readable: Vec<bool>,
    /// One flag per socket in `writes`.
    pub writable: Vec<bool>,
    /// The exec signals (of those asked for) that arrived.
    pub signals: u32,
    /// Nothing was ready and no signal came before the timeout.
    pub timed_out: bool,
}

/// An fd_set: FD_SETSIZE bits in longwords.
#[derive(Clone, Copy)]
struct FdSet([u32; (FD_SETSIZE / 32) as usize]);

impl FdSet {
    fn of(socks: &[&Socket<'_>]) -> Result<Self> {
        let mut s = FdSet([0; (FD_SETSIZE / 32) as usize]);
        for k in socks {
            if !(0..FD_SETSIZE).contains(&k.fd) {
                return Err(AmigaError::InvalidArgument("socket number beyond FD_SETSIZE"));
            }
            s.0[(k.fd / 32) as usize] |= 1 << (k.fd % 32);
        }
        Ok(s)
    }

    fn has(&self, fd: i32) -> bool {
        self.0[(fd / 32) as usize] & (1 << (fd % 32)) != 0
    }
}

impl SocketLib {
    /// A TCP connection to `to`.
    pub fn tcp_connect(&self, to: &SockAddrIn) -> Result<TcpStream<'_>> {
        let sock = Socket::new(self, bs::AF_INET, bs::SOCK_STREAM, 0)?;
        self.connect(sock.fd, to)?;
        Ok(TcpStream { sock })
    }

    /// A TCP socket listening on `at` (address reuse on).
    pub fn tcp_listen(&self, at: &SockAddrIn, backlog: i32) -> Result<TcpListener<'_>> {
        let sock = Socket::new(self, bs::AF_INET, bs::SOCK_STREAM, 0)?;
        sock.set_reuse_addr(true)?;
        self.bind(sock.fd, at)?;
        self.listen(sock.fd, backlog)?;
        Ok(TcpListener { sock })
    }

    /// A UDP socket bound to `at`.
    pub fn udp_bind(&self, at: &SockAddrIn) -> Result<UdpSocket<'_>> {
        let sock = Socket::new(self, bs::AF_INET, bs::SOCK_DGRAM, 0)?;
        self.bind(sock.fd, at)?;
        Ok(UdpSocket { sock })
    }

    /// The IPv4 addresses of `host` (gethostbyname; a dotted address
    /// comes back as itself). Copied at once: the library reuses its
    /// result. `Err(HostNotFound)` when there are none. It may wait for
    /// DNS without a time limit.
    pub fn resolve(&self, host: &str) -> Result<Vec<[u8; 4]>> {
        let h = crate::cstr::c_copy(host.as_bytes())?;
        let i = self.iface();
        let mut v = Vec::new();
        // SAFETY: an open interface and a NUL-terminated name; the
        // struct hostent (h_addrtype at 8, h_length at 12, h_addr_list at
        // 16, a NULL-ended list of address pointers) belongs to this base
        // and is read before any other call on it.
        unsafe {
            let he = ((*i).gethostbyname)(i, h.as_ptr() as *mut u8) as *const u8;
            if he.is_null() {
                return Err(AmigaError::HostNotFound);
            }
            let kind = core::ptr::read(he.add(8) as *const i32);
            let len = core::ptr::read(he.add(12) as *const i32);
            let list = core::ptr::read(he.add(16) as *const *const *const u8);
            if kind == bs::AF_INET && len == 4 && !list.is_null() {
                let mut k = 0;
                loop {
                    let a = core::ptr::read(list.add(k));
                    if a.is_null() || k >= 64 {
                        break;
                    }
                    v.push([*a, *a.add(1), *a.add(2), *a.add(3)]);
                    k += 1;
                }
            }
        }
        if v.is_empty() { Err(AmigaError::HostNotFound) } else { Ok(v) }
    }

    /// gethostname.
    pub fn hostname(&self) -> Result<String> {
        let i = self.iface();
        let mut buf = [0u8; 256];
        // SAFETY: an open interface and a buffer of the size given.
        let r = unsafe { ((*i).gethostname)(i, buf.as_mut_ptr(), buf.len() as i32) };
        if r < 0 {
            return Err(AmigaError::IoError(self.errno()));
        }
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        Ok(String::from_utf8_lossy(&buf[..end]).into_owned())
    }

    /// The signals that break a waiting call with EINTR (SBTC_BREAKMASK;
    /// CTRL-C by default).
    pub fn set_break_mask(&self, mask: u32) -> Result<()> {
        self.set_base_value(bs::SBTC_BREAKMASK, mask)
    }

    /// WaitSelect: wait until a socket of `reads` can be read, one of
    /// `writes` written, one of `signals` arrives, or `timeout` passes
    /// (None: no limit). A break signal gives `Err(IoError(EINTR))`.
    pub fn wait(&self, reads: &[&Socket<'_>], writes: &[&Socket<'_>], signals: u32, timeout: Option<Duration>) -> Result<Ready> {
        let mut r = FdSet::of(reads)?;
        let mut w = FdSet::of(writes)?;
        let nfds = reads.iter().chain(writes.iter()).map(|s| s.fd + 1).max().unwrap_or(0);
        let mut tv = timeout.map(TimeVal::from);
        let mut sig = signals;
        let i = self.iface();
        // SAFETY: an open interface; two fd_sets, an optional timeval and a
        // signal mask on this stack; NULL for the exception set.
        let n = unsafe {
            ((*i).WaitSelect)(
                i,
                nfds,
                &mut r as *mut FdSet as APTR,
                &mut w as *mut FdSet as APTR,
                core::ptr::null_mut(),
                tv.as_mut().map_or(core::ptr::null_mut(), |t| t as *mut TimeVal as *mut c_void),
                &mut sig,
            )
        };
        if n < 0 {
            return Err(AmigaError::IoError(self.errno()));
        }
        let readable: Vec<bool> = reads.iter().map(|s| r.has(s.fd)).collect();
        let writable: Vec<bool> = writes.iter().map(|s| w.has(s.fd)).collect();
        let got = sig & signals;
        Ok(Ready { timed_out: n == 0 && got == 0, readable, writable, signals: got })
    }
}
