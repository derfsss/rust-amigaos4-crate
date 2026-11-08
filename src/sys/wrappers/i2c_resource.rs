//! II2CResource global(s) and convenience wrappers.
//!
//! Hand-written binding for i2c.resource — multi-bus arbiter that
//! returns the bus-level I2CIFace for a given bus number or name.
//! Lives in i2c.h alongside I2CIFace (the bus-level interface).

use crate::sys::types::*;
use crate::sys::interfaces::i2c_resource::*;

// ---- II2CResource (I2CResourceIFace) ----

#[cfg(target_os = "amigaos")]
extern "C" {
    pub static II2CResource: *mut I2CResourceIFace;
}

#[cfg(not(target_os = "amigaos"))]
#[allow(non_upper_case_globals)]
pub static mut II2CResource: *mut I2CResourceIFace = core::ptr::null_mut();

#[inline]
pub unsafe fn i2c_resource_get_bus_by_number(num: u32) -> *mut I2CIFace {
    ((*II2CResource).GetBusByNumber)(II2CResource, num)
}

#[inline]
pub unsafe fn i2c_resource_get_bus_by_name(name: CONST_STRPTR) -> *mut I2CIFace {
    ((*II2CResource).GetBusByName)(II2CResource, name)
}
