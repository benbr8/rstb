use num_format::{Locale, ToFormattedString};
use std::ffi::CStr;

use crate::sim_if::{ObjectKind, SimCallback, SimIf, SIM_IF};
use crate::trigger;
use crate::trigger::EdgeKind;
use crate::SimpleResult;

pub(crate) struct Verilator {
}

impl Verilator {
}

impl SimIf for Verilator {
    fn set_value_int(&self, obj: usize, value: i32, force: bool) -> SimpleResult<()> {
        todo!()
    }
    fn get_value_int(&self, obj: usize) -> SimpleResult<i32> {
        todo!();
    }
    fn set_value_bin(&self, obj: usize, value: String, force: bool) -> SimpleResult<()> {
        panic!("Setting value as binary string is not implemented for Verilator.");
    }
    fn get_value_bin(&self, obj: usize) -> SimpleResult<String> {
        panic!("Getting value as binary string is not implemented for Verilator.");
    }
    fn release(&self, obj: usize) -> SimpleResult<()> {
        panic!("Forcing/Releasing values is not implemented for Verilator.");
    }
    fn get_handle_by_name(&self, name: &str) -> SimpleResult<usize> {
        todo!()
    }
    fn get_sim_time_steps(&self) -> u64 {
        todo!()
    }
    fn log(&self, msg: &str) {
        todo!()
    }
    fn get_size(&self, obj: usize) -> i32 {
        todo!()
    }
    fn get_kind(&self, obj: usize) -> ObjectKind {
        todo!()
    }
    fn is_signed(&self, obj_handle: usize) -> bool {
        panic!("Signed values do not exist in verilated HDL.");
    }
    fn get_full_name(&self, obj: usize) -> SimpleResult<String> {
        todo!()
    }
    fn get_sim_precision(&self) -> i8 {
        panic!("Verilator does not expose simulation precision")
    }
    fn get_root_handle(&self) -> SimpleResult<usize> {
        todo!()
    }
    fn register_callback_rw(&self) -> SimpleResult<usize> {
        panic!("Verilator does not support RW callbacks");
    }
    fn register_callback_ro(&self) -> SimpleResult<usize> {
        // this should be called after eval
        todo!()
    }
    fn register_callback_time(&self, t: u64) -> SimpleResult<usize> {
        todo!()
    }
    fn register_callback_edge(&self, sig_hdl: usize) -> SimpleResult<usize> {
        todo!()
    }
    fn cancel_callback(&self, cb_hdl: usize) -> SimpleResult<()> {
        panic!("Verilator callbacks only execute once and don't need cancellation.")
    }
}


#[no_mangle]
pub(crate) extern "C" fn react_vl_edge() {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn react_vl_time() {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn react_vl_ro()  {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn react_vl_rw() {
    todo!()
}

fn check_null<T>(ptr: *mut T) -> SimpleResult<*mut T> {
    if ptr.is_null() {
        Err(())
    } else {
        Ok(ptr)
    }
}
