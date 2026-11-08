# Networking: sockets over bsdsocket.library

Two modules give a program TCP/IP networking:

- `net`: owned, typed sockets (`TcpStream`, `TcpListener`, `UdpSocket`),
  socket options, waiting on sockets and signals together, and name lookup.
- `bsdsocket`: the library itself (`SocketLib`), raw socket calls, the
  network interfaces, the stack's statistics, and the constants.

On AmigaOS the TCP/IP stack is reached through **bsdsocket.library**. Roadshow
is one stack that provides it; the crate works with any stack that offers
bsdsocket.library's `main` interface.

## When to use it

Use `net` for clients and servers: fetching from a web server, a small echo
or chat server, sending UDP datagrams. Use `bsdsocket` directly when you need
a call `net` does not wrap, or to show interface and traffic information.

Things `net` can do that `std::net` cannot: wait on sockets *and* exec
signals (such as Ctrl-C) in one call, with a time limit.

## Cargo features

```toml
[dependencies]
amigaos4 = { path = "...", features = ["bsdsocket"] }
```

| Feature | Gives | Pulls in |
|---|---|---|
| `bsdsocket` | both modules: `bsdsocket` and `net` | `utility` |

There is no separate `net` feature.

## Build modes

| Mode | Status |
|---|---|
| no-C-runtime program (`#![no_std]`) | Tested by the crate's test programs: name lookup, UDP over loopback, WaitSelect timeouts and readiness, a wake-up by an exec signal, non-blocking reads, read timeouts, a 20000-byte TCP echo to a child process, and sockets closed on drop. |
| std program (`fn main` on clib4) | Compiles; not exercised by the crate's test programs. |

## The one rule: sockets belong to the task that opened the library

Each task that opens bsdsocket.library gets its **own library base** and its
own socket table. A socket number only means something to the base that made
it. So:

- `SocketLib`, `Socket` and the typed sockets are not `Send`. You cannot
  move them to another task or thread.
- Each task or child process that does networking opens its own `SocketLib`.
- A socket borrows its `SocketLib`, so the library outlives every socket made
  from it.

A std program that also uses `std::net` gets those sockets from the C
library, which opens its own base. Do not pass socket numbers between the two.

## Opening the library

```rust
use amigaos4::bsdsocket::SocketLib;

let lib = SocketLib::open(4)?;     // bsdsocket.library, version 4 or later
let (version, revision) = lib.version();
let name = lib.hostname()?;
```

`SocketLib::open_named(name, version)` opens a library with bsdsocket's
interface under another name or path. Dropping the `SocketLib` closes it.

## Addresses

`SockAddrIn` is an IPv4 address and port (`struct sockaddr_in`).

```rust
use amigaos4::bsdsocket::SockAddrIn;

let addr = SockAddrIn::v4([192, 168, 1, 10], 80);
let ip: [u8; 4] = addr.ip();
let port: u16 = addr.sin_port;
```

The CPU is big-endian like the network, so `sin_port` is the plain port
number. Use `[0, 0, 0, 0]` to listen on every interface and port 0 to let the
stack choose a port (read it back with `local_addr`).

`lib.resolve(host)` looks up a host name and returns its IPv4 addresses as
`Vec<[u8; 4]>`. A dotted address comes back as itself. No address gives
`Err(AmigaError::HostNotFound)`. The lookup may wait for DNS with no time
limit.

## Worked example: a TCP client

This fetches a web page over HTTP/1.0 and returns the whole reply.

```rust
use amigaos4::bsdsocket::{SockAddrIn, SocketLib};
use amigaos4::error::Result;
use core::time::Duration;

fn fetch(host: &str) -> Result<Vec<u8>> {
    let lib = SocketLib::open(4)?;

    // 1. Look up the host and connect to port 80.
    let ip = lib.resolve(host)?[0];
    let mut s = lib.tcp_connect(&SockAddrIn::v4(ip, 80))?;

    // 2. Do not wait more than 10 seconds for any one read.
    s.sock.set_read_timeout(Some(Duration::from_secs(10)))?;

    // 3. Send the request. write_all loops until every byte is taken.
    let req = format!("GET / HTTP/1.0\r\nHost: {}\r\n\r\n", host);
    s.write_all(req.as_bytes())?;

    // 4. Read until the server closes the connection (read returns 0).
    let mut reply = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = s.read(&mut buf)?;
        if n == 0 {
            break;
        }
        reply.extend_from_slice(&buf[..n]);
    }
    Ok(reply)
    // s is dropped here (CloseSocket), then lib.
}
```

In a no-C-runtime program, bring `format!` and `Vec` in from `alloc`.

## Worked example: a small echo server

This listens on port 7000 and sends back whatever a client sends, one client
at a time. It waits on the listening socket and Ctrl-C together, so the user
can stop it from the Shell.

```rust
use amigaos4::bsdsocket::{SockAddrIn, SocketLib};
use amigaos4::error::Result;
use amigaos4::net;
use amigaos4::signal::Cancel;
use core::time::Duration;

fn serve() -> Result<()> {
    let lib = SocketLib::open(4)?;
    let listener = lib.tcp_listen(&SockAddrIn::v4([0, 0, 0, 0], 7000), 4)?;

    loop {
        // Wait for a connection or Ctrl-C, with no time limit.
        let ready = lib.wait(&[&listener.sock], &[], Cancel::MASK, None)?;
        if ready.signals & Cancel::MASK != 0 {
            break;
        }
        if !ready.readable[0] {
            continue;
        }

        let (mut conn, peer) = listener.accept()?;
        let _who = peer.ip();
        conn.sock.set_read_timeout(Some(Duration::from_secs(30)))?;

        let mut buf = [0u8; 1024];
        loop {
            match conn.read(&mut buf) {
                Ok(0) => break,                                   // client closed
                Ok(n) => conn.write_all(&buf[..n])?,
                Err(e) if net::interrupted(&e) => return Ok(()), // Ctrl-C
                Err(_) => break,                                  // timeout or reset
            }
        }
        // conn is dropped here: the connection is closed.
    }
    Ok(())
}
```

`tcp_listen` turns address reuse on, so the server can be restarted at once
on the same port.

To serve several clients at the same time, either put every connection in
one `lib.wait` call, or start a child process per client with
`dos::process::spawn`. A child must open its own `SocketLib`; it cannot use
the parent's sockets.

## `net`: the socket types

### `Socket`

`Socket` owns one socket and closes it (CloseSocket) on drop. The typed
sockets hold one in their public `sock` field, so the options are reached as
`stream.sock.set_nodelay(true)`.

| Method | What it does |
|---|---|
| `Socket::new(lib, domain, kind, protocol)` | A new socket (`socket()`). |
| `fd()` | The socket number, for raw calls. |
| `local_addr()`, `peer_addr()` | The local and remote address. |
| `set_nonblocking(on)` | Calls that would wait fail with EWOULDBLOCK instead. |
| `bytes_available()` | Bytes that can be read without waiting. |
| `set_reuse_addr`, `set_keepalive`, `set_broadcast`, `set_nodelay` | On/off options. `set_nodelay` is TCP_NODELAY. |
| `set_read_timeout(Option<Duration>)`, `set_write_timeout(...)` | A receive or send waits at most this long; `None` for ever. |
| `set_linger(Option<Duration>)` | `Some(d)`: closing waits up to `d` for unsent data. |
| `take_error()` | The pending socket error, if any (reading clears it). |
| `shutdown(Shutdown::Read / Write / Both)` | Shuts one or both directions. |

### `TcpStream`, `TcpListener`, `UdpSocket`

| Made by | Type | Main methods |
|---|---|---|
| `lib.tcp_connect(&addr)` | `TcpStream` | `read` (0 when the peer has closed), `write`, `write_all`, `read_exact` |
| `lib.tcp_listen(&addr, backlog)` | `TcpListener` | `accept() -> (TcpStream, SockAddrIn)`, `local_addr` |
| `lib.udp_bind(&addr)` | `UdpSocket` | `send_to`, `recv_from`, `connect` (a default peer), `send`, `recv`, `local_addr` |

`TcpStream` also implements the crate's `io::Read` and `io::Write` traits.

A UDP example:

```rust
use amigaos4::bsdsocket::{SockAddrIn, SocketLib};
use amigaos4::net;

let lib = SocketLib::open(4)?;
let sock = lib.udp_bind(&SockAddrIn::v4([0, 0, 0, 0], 0))?;
sock.sock.set_broadcast(true)?;
sock.send_to(b"hello", &SockAddrIn::v4([192, 168, 1, 255], 9999))?;

sock.sock.set_nonblocking(true)?;
let mut buf = [0u8; 512];
match sock.recv_from(&mut buf) {
    Ok((n, from)) => { /* n bytes from `from` */ }
    Err(e) if net::would_block(&e) => { /* nothing waiting */ }
    Err(e) => return Err(e),
}
```

### Waiting: `SocketLib::wait`

```rust
pub fn wait(&self, reads: &[&Socket<'_>], writes: &[&Socket<'_>],
            signals: u32, timeout: Option<Duration>) -> Result<Ready>
```

This is WaitSelect. It returns when a socket in `reads` can be read, one in
`writes` can be written, one of the exec `signals` arrives, or `timeout`
passes (`None`: no limit). The `Ready` result has:

| Field | Meaning |
|---|---|
| `readable: Vec<bool>` | One flag per socket in `reads`, in order. |
| `writable: Vec<bool>` | One flag per socket in `writes`. |
| `signals: u32` | The signals (of those asked for) that arrived. |
| `timed_out: bool` | Nothing was ready and no signal came. |

An **exec signal** is one of a task's 32 wake-up bits; Ctrl-C from the Shell
is `signal::Cancel::MASK`. Only socket numbers below `FD_SETSIZE` (256) can
be watched.

### The break mask

Some signals *break* a waiting socket call: the call ends with EINTR. By
default this is Ctrl-C. `lib.set_break_mask(mask)` changes the set. Test an
error with `net::interrupted(&e)`.

### Errors

A failed call gives `AmigaError::IoError(errno)`, read from the library
straight after the call. `net::would_block(&e)` is true for EWOULDBLOCK
(a non-blocking socket had nothing), and `net::interrupted(&e)` for EINTR.
`net` also exports `EPIPE`, `EADDRINUSE` and `ECONNRESET`; `bsdsocket`
exports `EINTR`, `EAGAIN`, `EWOULDBLOCK`, `EINPROGRESS`, `ETIMEDOUT` and
`ECONNREFUSED`.

## `bsdsocket`: the library

### Raw socket calls

`SocketLib` has thin wrappers that take a socket number: `socket`, `connect`,
`bind`, `listen`, `accept`, `send`, `recv`, `shutdown`, `set_opt`, `get_opt`,
`close` and `local_addr`. Prefer the `net` types, which close their socket
for you. `set_opt` and `get_opt` take a 4-byte value and refuse
SO_RCVTIMEO, SO_SNDTIMEO and SO_LINGER (use the typed setters on `Socket`).

`lib.iface()` returns the raw interface for calls the crate does not wrap,
and `lib.errno()` the base's last error.

### Library settings

`get_base_value(code)` and `set_base_value(code, value)` read and set one
SocketBaseTagList value, for example `SBTC_BREAKMASK`,
`SBTC_HAVE_INTERFACE_API` or `SBTC_HAVE_STATUS_API`.

### Network interfaces

An *interface* is one network connection the stack manages (an Ethernet
card, the loopback). Its name is the one in the stack's configuration.

```rust
use amigaos4::bsdsocket::{self as bs, SocketLib};

let lib = SocketLib::open(4)?;
for name in lib.interfaces()? {
    let (addr, mask) = lib.interface_ipv4(&name)?;
    let mtu = lib.query_u32(&name, bs::IFQ_MTU)?;
    let bytes_in = lib.query_u64(&name, bs::IFQ_GetBytesIn)?;
    let (device, unit) = lib.interface_device(&name)?; // the SANA-II driver
    let up = lib.query_u32(&name, bs::IFQ_State)? as i32 == bs::SM_Up;
}
```

| Method | Returns |
|---|---|
| `interfaces()` | The interface names. |
| `interface_ipv4(name)` | Its IPv4 address and netmask. |
| `query_u32(name, tag)`, `query_u64(name, tag)` | One answer for an `IFQ_*` tag (64-bit for `IFQ_GetBytesIn` and `IFQ_GetBytesOut`). |
| `interface_device(name)` | The SANA-II device name and unit behind it. SANA-II is the AmigaOS network driver standard. |
| `copy_stats(name)` | A `SANA2CopyStats`: how many times each SANA-II copy function was called (not bytes). |
| `unsafe query_interface(name, tags)` | Any QueryInterfaceTagList call; each tag's data must point to storage of the right type. |

### Statistics

```rust
use amigaos4::bsdsocket::{field_index, NETSTATUS_tcp, NETSTATUS_tcp_sockets, TCPSTAT_FIELDS};

let mut tcp = [0u32; 64];
let bytes = lib.network_stats(NETSTATUS_tcp, &mut tcp)?;
// TCPSTAT_FIELDS names each counter in order; field_index finds one by name.

for c in lib.connections(NETSTATUS_tcp_sockets)? {
    let port = c.pcd_local_port;
}
```

- `network_stats(kind, out)` fills 32-bit counters for one `NETSTATUS_*`
  kind and returns the number of bytes copied. `TCPSTAT_FIELDS`,
  `IPSTAT_FIELDS`, `UDPSTAT_FIELDS` and `MBSTAT_FIELDS` name the counters in
  order.
- `connections(NETSTATUS_tcp_sockets)` or `connections(NETSTATUS_udp_sockets)`
  lists every socket's addresses, ports, queue sizes and TCP state as
  `ProtocolConnectionData`.

Not every stack has the interface and status calls; check
`get_base_value(SBTC_HAVE_INTERFACE_API)` and
`get_base_value(SBTC_HAVE_STATUS_API)` first.

### Constants

The constants keep the SDK's names: `AF_INET`, `SOCK_STREAM`, `SOCK_DGRAM`,
`SOL_SOCKET`, `SO_*`, `MSG_PEEK`, `MSG_WAITALL`, `MSG_DONTWAIT`, `IPPROTO_*`,
`INADDR_ANY`, `TCP_NODELAY`, `TCP_MAXSEG`, the `IFQ_*` tags, `NETSTATUS_*`
and `SBTC_*`.

## Limitations

- IPv4 only.
- Sockets cannot move between tasks (see the rule above).
- `resolve` has no time limit.
- `wait` watches only socket numbers below 256.

## See also

- [dos.md](dos.md): `process::spawn` for a child process per client.
