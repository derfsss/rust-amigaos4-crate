use core::fmt;

/// Errors returned by safe AmigaOS wrappers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmigaError {
    /// An OS call returned a null pointer unexpectedly.
    NullPointer,
    /// A memory or resource allocation failed.
    AllocationFailed,
    /// A clib4 POSIX call set errno to this value.
    IoError(i32),
    /// An AmigaOS DOS error code (from IoErr()).
    DosError(i32),
    /// A read or write operation hit EOF before completing.
    UnexpectedEof,
    /// A byte-string argument was missing its trailing `\0`.
    ///
    /// OS-string arguments must be null-terminated; build them with
    /// [`amstr!`](crate::amstr) to get the terminator at compile time.
    NotNulTerminated,
    /// A hostname could not be resolved to any address.
    HostNotFound,
    /// `StartDMA` refused the region (it returned 0 entries).
    DmaFailed,
    /// A device did not behave as its interface says; the text names how.
    Device(&'static str),
    /// The lock asked for would wait for the calling task itself (it
    /// already holds it exclusively).
    WouldDeadlock,
    /// The call must run on another task (the one named in its
    /// documentation, such as the task that spawned a child).
    WrongTask,
    /// An exec I/O request ended with this io_Error (or OpenDevice refused
    /// with it): exec/errors.h and the device's own codes.
    IoErr(i8),
    /// The name is taken (named memory, a public object).
    Exists,
    /// An argument the OS cannot take as it is; the text says which.
    InvalidArgument(&'static str),
    /// The installed library is too old for the call; the text names it.
    Unsupported(&'static str),
    /// Someone else holds it (a PCI device locked by another driver).
    InUse,
    /// The user asked to stop (Ctrl-C, SIGBREAKF_CTRL_C) while it waited.
    Cancelled,
}

impl fmt::Display for AmigaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AmigaError::NullPointer => write!(f, "null pointer"),
            AmigaError::AllocationFailed => write!(f, "allocation failed"),
            AmigaError::IoError(e) => write!(f, "I/O error (errno={})", e),
            AmigaError::DosError(e) => write!(f, "DOS error ({})", e),
            AmigaError::UnexpectedEof => write!(f, "unexpected end of file"),
            AmigaError::NotNulTerminated => {
                write!(f, "byte string is not null-terminated (use amstr!)")
            }
            AmigaError::HostNotFound => write!(f, "host not found"),
            AmigaError::DmaFailed => write!(f, "StartDMA failed"),
            AmigaError::Device(what) => write!(f, "device: {}", what),
            AmigaError::WouldDeadlock => write!(f, "the calling task already holds the lock"),
            AmigaError::WrongTask => write!(f, "called from the wrong task"),
            AmigaError::IoErr(e) => write!(f, "I/O error {}", e),
            AmigaError::Exists => write!(f, "the name exists already"),
            AmigaError::InvalidArgument(what) => write!(f, "invalid argument: {}", what),
            AmigaError::InUse => write!(f, "held by another user"),
            AmigaError::Cancelled => write!(f, "cancelled (Ctrl-C)"),
            AmigaError::Unsupported(what) => write!(f, "not supported by the installed library: {}", what),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AmigaError {}

/// Convenience alias used throughout this crate.
pub type Result<T> = core::result::Result<T, AmigaError>;

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::format;

    #[test]
    fn display_null_pointer() {
        let e = AmigaError::NullPointer;
        assert_eq!(format!("{}", e), "null pointer");
    }

    #[test]
    fn display_allocation_failed() {
        let e = AmigaError::AllocationFailed;
        assert_eq!(format!("{}", e), "allocation failed");
    }

    #[test]
    fn display_io_error() {
        let e = AmigaError::IoError(5);
        assert_eq!(format!("{}", e), "I/O error (errno=5)");
    }

    #[test]
    fn display_io_error_zero() {
        let e = AmigaError::IoError(0);
        assert_eq!(format!("{}", e), "I/O error (errno=0)");
    }

    #[test]
    fn display_io_error_negative() {
        let e = AmigaError::IoError(-1);
        assert_eq!(format!("{}", e), "I/O error (errno=-1)");
    }

    #[test]
    fn display_dos_error() {
        let e = AmigaError::DosError(205);
        assert_eq!(format!("{}", e), "DOS error (205)");
    }

    #[test]
    fn display_dos_error_negative() {
        let e = AmigaError::DosError(-1);
        assert_eq!(format!("{}", e), "DOS error (-1)");
    }

    #[test]
    fn display_unexpected_eof() {
        let e = AmigaError::UnexpectedEof;
        assert_eq!(format!("{}", e), "unexpected end of file");
    }

    #[test]
    fn debug_formatting() {
        let e = AmigaError::IoError(42);
        assert_eq!(format!("{:?}", e), "IoError(42)");
    }

    #[test]
    fn equality() {
        assert_eq!(AmigaError::NullPointer, AmigaError::NullPointer);
        assert_eq!(AmigaError::IoError(5), AmigaError::IoError(5));
        assert_ne!(AmigaError::IoError(5), AmigaError::IoError(6));
        assert_ne!(AmigaError::NullPointer, AmigaError::AllocationFailed);
    }

    #[test]
    fn copy_clone() {
        let e = AmigaError::DosError(100);
        let e2 = e; // Copy
        let e3 = e.clone(); // Clone
        assert_eq!(e, e2);
        assert_eq!(e, e3);
    }

    #[test]
    fn result_ok() {
        let r: Result<u32> = Ok(42);
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn result_err() {
        let r: Result<u32> = Err(AmigaError::NullPointer);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), AmigaError::NullPointer);
    }

    #[test]
    fn io_error_max_errno() {
        let e = AmigaError::IoError(i32::MAX);
        let s = format!("{}", e);
        assert!(s.contains(&format!("{}", i32::MAX)));
    }

    #[test]
    fn dos_error_zero() {
        let e = AmigaError::DosError(0);
        assert_eq!(format!("{}", e), "DOS error (0)");
    }

    #[test]
    fn display_not_nul_terminated() {
        let e = AmigaError::NotNulTerminated;
        assert_eq!(
            format!("{}", e),
            "byte string is not null-terminated (use amstr!)"
        );
    }

    #[test]
    fn display_host_not_found() {
        let e = AmigaError::HostNotFound;
        assert_eq!(format!("{}", e), "host not found");
    }

    #[test]
    fn all_variants_distinct() {
        let variants: &[AmigaError] = &[
            AmigaError::NullPointer,
            AmigaError::AllocationFailed,
            AmigaError::IoError(0),
            AmigaError::DosError(0),
            AmigaError::UnexpectedEof,
            AmigaError::NotNulTerminated,
            AmigaError::HostNotFound,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j], "variants {} and {} should differ", i, j);
            }
        }
    }
}
