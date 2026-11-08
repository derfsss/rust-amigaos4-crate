//! IPotgo global(s) and convenience wrappers.
//!
//! Hand-written binding for potgo.resource — Amiga POTGO register
//! (paddle/joystick analog input) arbitration.

use crate::sys::types::*;
use crate::sys::interfaces::potgo::*;

// ---- IPotgo (PotgoIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static IPotgo: *mut PotgoIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut IPotgo: *mut PotgoIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn potgo_alloc_pot_bits(bits: u16) -> u16 {
    ((*IPotgo).AllocPotBits)(IPotgo, bits)
}

#[inline]
pub unsafe fn potgo_free_pot_bits(bits: u16) {
    ((*IPotgo).FreePotBits)(IPotgo, bits)
}

#[inline]
pub unsafe fn potgo_write_potgo(value: u16, mask: u16) {
    ((*IPotgo).WritePotgo)(IPotgo, value, mask)
}
