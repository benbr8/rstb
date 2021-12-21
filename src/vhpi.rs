use std::ffi::CStr;

use crate::sim_if::SimCallback;
use crate::trigger;
use crate::trigger::EdgeKind;
use crate::{sim_if, sim_if::SIM_IF, sim_if::ObjectKind, vhpi_user, RstbErr, RstbResult};

pub struct Vhpi {
    // precision: i32,
}

impl sim_if::SimIf for Vhpi {
    fn set_value_int(&self, handle: usize, value: i32) -> RstbResult<()> {
        // TODO: Currently there is an issue with writing values via VHPI. Questa doesnt support
        // waveform creation with VHPI enabled so it's hard to check whats wrong...
        let mut val = new_vhpi_value();
        val.set_int_val(value as u32);
        // self.log(&format!("setting value = {:?}", val.format));
        unsafe {
            match vhpi_user::vhpi_put_value(
                handle as *mut u32,
                &mut val,
                vhpi_user::vhpiPutValueModeT_vhpiDepositPropagate,
            ) {
                0 => Ok(()),
                _ => Err(RstbErr),
            }
        }
    }
    fn get_value_int(&self, obj: usize) -> RstbResult<i32> {
        unsafe {
            let mut val = new_vhpi_value();
            vhpi_user::vhpi_get_value(obj as *mut u32, &mut val);
            if val.format == vhpi_user::vhpiFormatT_vhpiIntVal {
                Ok(val.value.intg as i32)
            } else {
                Err(RstbErr)
            }
        }
    }
    fn get_handle_by_name(&self, name: &str) -> RstbResult<usize> {
        let mut name_string = name.to_string();
        name_string.push('\0');

        let hdl = unsafe {
            vhpi_user::vhpi_handle_by_name(
                name_string.as_mut_str().as_mut_ptr() as *mut i8,
                std::ptr::null_mut(),
            )
        };
        if hdl.is_null() {
            self.log(&format!("Couldn't get handle from name {}", name));
            Result::Err(RstbErr)
        } else {
            Result::Ok(hdl as usize)
        }
    }
    fn get_sim_time_steps(&self) -> u64 {
        let mut time_obj = vhpi_user::vhpiTimeT {
            ..Default::default()
        };
        unsafe {
            vhpi_user::vhpi_get_time(&mut time_obj, std::ptr::null_mut() as *mut i64);
        }
        ((time_obj.high as u64) << 32) + time_obj.low as u64
    }
    fn log(&self, s: &str) {
        let mut string = format!("{} {}\n\0", self.get_sim_time_steps(), s);
        // eprintln!("{}", string);
        unsafe { vhpi_user::vhpi_printf(string.as_mut_ptr() as *mut i8) };
    }
    fn get_kind(&self, _obj: usize) -> ObjectKind {
        // TODO !!
        ObjectKind::BitVector(1)
    }
    fn get_full_name(&self, obj: usize) -> RstbResult<String> {
        unsafe {
            let ptr =
                vhpi_user::vhpi_get_str(vhpi_user::vhpiStrPropertyT_vhpiFullNameP, obj as *mut u32);
            let s = CStr::from_ptr(check_null(ptr)?)
                .to_owned()
                .into_string()
                .unwrap();
            Ok(s)
        }
    }
    fn get_sim_precision(&self) -> i8 {
        let precision = unsafe {
            vhpi_user::vhpi_get_phys(
                vhpi_user::vhpiPhysPropertyT_vhpiResolutionLimitP,
                std::ptr::null_mut(),
            )
        };
        let mut precision_int = ((precision.high as i64) << 32) + (precision.low as i64);
        if precision_int > 2 {
            precision_int = 2;
        }
        if precision_int < -15 {
            precision_int = -15;
        }
        precision_int as i8
    }
    fn get_root_handle(&self) -> RstbResult<usize> {
        let root = unsafe {
            vhpi_user::vhpi_handle(vhpi_user::vhpiOneToOneT_vhpiRootInst, std::ptr::null_mut())
        };
        Ok(root as usize)
    }
    fn register_callback(&self, cb: SimCallback) -> RstbResult<usize> {
        #[allow(unused_assignments)]  // time is actually read by simulator
        let mut time = vhpi_user::vhpiTimeT{..Default::default()};
        let mut cb_data = match cb {
            SimCallback::ReadOnly => vhpi_user::vhpiCbDataT {
                reason: vhpi_user::vhpiCbEndOfTimeStep as i32,
                cb_rtn: Some(react_vhpi),
                ..Default::default()
            },
            SimCallback::ReadWrite => vhpi_user::vhpiCbDataT {
                reason: vhpi_user::vhpiCbLastKnownDeltaCycle as i32,
                cb_rtn: Some(react_vhpi),
                ..Default::default()
            },
            SimCallback::Time(t) => {
                // self.log(&format!("Registering timer: t={}", t));
                time = vhpi_user::vhpiTimeT {
                    high: (t >> 32) as u32,
                    low: (t & 0xFFFF_FFFF) as u32,
                };
                // self.log(&format!("time={:?}", time));

                vhpi_user::vhpiCbDataT {
                    reason: vhpi_user::vhpiCbAfterDelay as i32,
                    time: &mut time,
                    cb_rtn: Some(react_vhpi),
                    ..Default::default()
                }
            },
            SimCallback::Edge(hdl) => {

                SIM_IF.log("registering edge cb");
                vhpi_user::vhpiCbDataT {
                reason: vhpi_user::vhpiCbValueChange as i32,
                obj: hdl as *mut u32,
                cb_rtn: Some(react_vhpi),
                ..Default::default()
            }},
        };
        let cb_hdl =
            unsafe { vhpi_user::vhpi_register_cb(&mut cb_data, vhpi_user::vhpiReturnCb as i32) };

        // SIM_IF.log(&format!("cb_hdl = {}", cb_hdl as usize));
        Ok(cb_hdl as usize)
    }
    fn cancel_callback(&self, cb_hdl: usize) -> RstbResult<()> {
        match unsafe { vhpi_user::vhpi_remove_cb(cb_hdl as *mut u32) } {
            0 => Ok(()),
            _ => Err(RstbErr),
        }
    }
}

#[no_mangle]
extern "C" fn react_vhpi(cb_data: *const vhpi_user::vhpiCbDataS) {
    let cb = unsafe {
        if (*cb_data).reason == vhpi_user::vhpiCbValueChange as i32 {
            SIM_IF.log("reacting to value change");
        };
        (*cb_data)
            .to_sim_callback()
            .expect("Invalid callback data received.")
    };
    // vpi::log(&format!("Converted CB: {:?}", cb));

    match cb {
        SimCallback::Edge(hdl) => {
            // if signal is not a scalar, schedule all callbacks, since there is no "rising" on a multi bit signal
            let mut edge = EdgeKind::Any;
            let mut is_scalar = true;
            if let sim_if::ObjectKind::BitVector(size) = sim_if::SIM_IF.get_kind(hdl) {
                if size != 1 {
                    is_scalar = false;
                }
            }
            if is_scalar {
                unsafe {
                    if !(*cb_data).value.is_null() {
                        // this actually happens under some conditions?
                        edge = match (*(*cb_data).value).value.intg {
                            0 => EdgeKind::Falling,
                            _ => EdgeKind::Rising,
                        };
                    }
                }
            }
            trigger::react(cb, Some(edge));
        }
        _ => trigger::react(cb, None),
    }
}

impl vhpi_user::vhpiCbDataS {
    pub fn to_sim_callback(&self) -> Option<SimCallback> {
        const CB_AFTER_DELAY: i32 = vhpi_user::vhpiCbAfterDelay as i32;
        const CB_VALUE_CHANGE: i32 = vhpi_user::vhpiCbValueChange as i32;
        const CB_READ_WRITE: i32 = vhpi_user::vhpiCbLastKnownDeltaCycle as i32;
        const CB_READ_ONLY: i32 = vhpi_user::vhpiCbEndOfTimeStep as i32;
        match self.reason {
            CB_AFTER_DELAY => Some(SimCallback::Time(unsafe {
                // SIM_IF.log("to_sim_callback() after delay:");
                // SIM_IF.log(&format!("*self.time={:?}", *self.time));

                (u64::from((*self.time).high) << 32) + u64::from((*self.time).low)
            })),
            CB_VALUE_CHANGE => Some(SimCallback::Edge(self.obj as usize)),
            CB_READ_WRITE => Some(SimCallback::ReadWrite),
            CB_READ_ONLY => Some(SimCallback::ReadOnly),
            _ => None,
        }
    }
}

// fn is_array(handle: usize) -> bool {
//     // let val = unsafe { vpi_user::vpi_get(vpi_user::vpiArray as i32, handle as *mut u32) };
//     let val = unsafe { vpi_user::vpi_get(17, handle as *mut u32) };
//     let r = matches!(val, 0);
//     r
// }

// pub fn print_vpi_cb_data(cb_data: *mut vpi_user::t_cb_data) {
//     unsafe {
//         eprintln!("##################################");
//         eprintln!("print_cb_data");
//         eprintln!("addr: {:p}", cb_data);
//         eprintln!("cb_data: {:?}", *cb_data);
//         if !(*cb_data).time.is_null() {
//             eprintln!("time: {:?}", *(*cb_data).time);
//         }
//         eprintln!("cb_rtn: {:?}", (*cb_data).cb_rtn);
//         eprintln!("##################################");
//     }
// }

// fn discover_nets(handle: usize) -> Vec<usize> {
//     unsafe {
//         let iterator = vpi_user::vpi_iterate(vpi_user::vpiNet as i32, handle as *mut u32);
//         let mut list = Vec::new();
//         loop {
//             let obj = vpi_user::vpi_scan(iterator);
//             match obj.is_null() {
//                 true => break,
//                 _ => list.push(obj as usize),
//             };
//         }
//         list
//     }
// }

fn new_vhpi_value() -> vhpi_user::vhpiValueT {
    vhpi_user::vhpiValueT {
        format: vhpi_user::vhpiFormatT_vhpiIntVal,
        bufSize: 4,
        numElems: 0,
        unit: vhpi_user::vhpiPhysT { high: 0, low: 0 },
        value: vhpi_user::vhpiValueS__bindgen_ty_1 { intg: 0 },
    }
}

impl vhpi_user::vhpiValueT {
    fn set_int_val(&mut self, val: u32) {
        self.value = vhpi_user::vhpiValueS__bindgen_ty_1 { intg: val };
        self.format = vhpi_user::vhpiFormatT_vhpiIntVal;
    }
}

#[inline]
fn check_null<T>(ptr: *const T) -> RstbResult<*const T> {
    if ptr.is_null() {
        Err(RstbErr)
    } else {
        Ok(ptr)
    }
}


/*
 *  VHPI
 */

#[cfg(feature = "vhpi")]
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
extern "C" fn vhpi_start_of_simulation(_cb_data: *const vhpi_user::vhpiCbDataT) {
    eprintln!("vhpi_start_of_simulation");
    start_of_simulation();
}

#[cfg(feature = "vhpi")]
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
extern "C" fn vhpi_end_of_simulation(_cb_data: *const vhpi_user::vhpiCbDataT) {
    end_of_simulation();
}

#[cfg(feature = "vhpi")]
pub fn vhpi_init() {
    unsafe {
        let mut cb_data = vhpi_user::vhpiCbDataT {
            reason: vhpi_user::vhpiCbStartOfSimulation as i32,
            cb_rtn: Some(vhpi_start_of_simulation),
            ..Default::default()
        };
        vhpi_user::vhpi_register_cb(&mut cb_data, 0);
        let mut cb_data = vhpi_user::vhpiCbDataT {
            reason: vhpi_user::vhpiCbEndOfSimulation as i32,
            cb_rtn: Some(vhpi_end_of_simulation),
            ..Default::default()
        };
        vhpi_user::vhpi_register_cb(&mut cb_data, 0);
    };
}

#[cfg(feature = "vhpi")]
#[no_mangle]
pub extern "C" fn vhpi_entry_point() {
    vhpi_init();
}
