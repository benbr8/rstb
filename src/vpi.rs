use std::ffi::CStr;
use num_format::ToFormattedString;
use num_format::{Locale, ToFormattedStr};

use crate::sim_if::SimCallback;
use crate::trigger;
use crate::trigger::EdgeKind;
use crate::{vpi_user, RstbErr, RstbResult, sim_if, sim_if::ObjectKind};

pub struct Vpi {
    precision: i8,
}

impl Vpi {
    pub fn new() -> Self {
        Vpi{ precision: get_time_precision() }
    }
}

impl sim_if::SimIf for Vpi {
    fn set_value_int(&self, handle: usize, value: i32) -> RstbResult<()> {
        let mut val = vpi_user::t_vpi_value {
            format: vpi_user::vpiIntVal as i32,
            value: vpi_user::t_vpi_value__bindgen_ty_1 { integer: value },
        };
        let mut time = vpi_user::t_vpi_time {
            type_: vpi_user::vpiSimTime as i32,
            ..Default::default()
        };
        unsafe {
            vpi_user::vpi_put_value(
                handle as *mut u32,
                &mut val,
                &mut time,
                vpi_user::vpiInertialDelay as i32,
            );
        };
        // TODO: error??
        Ok(())
    }
    fn get_value_int(&self, obj: usize) -> RstbResult<i32> {
        unsafe {
            let mut val = vpi_user::t_vpi_value {
                format: vpi_user::vpiIntVal as i32,
                value: vpi_user::t_vpi_value__bindgen_ty_1 { integer: 0 }
            };
            vpi_user::vpi_get_value(obj as *mut u32, &mut val);
            if val.format == vpi_user::vpiIntVal as i32 {
                Ok(val.value.integer)
            } else {
                Err(RstbErr)
            }
        }
    }
    fn get_handle_by_name(&self, name: &str) -> RstbResult<usize> {
        let mut name_string = name.to_string();
        name_string.push('\0');

        let hdl = unsafe {
            vpi_user::vpi_handle_by_name(
                name_string.as_mut_str().as_mut_ptr() as *mut i8,
                std::ptr::null_mut(),
            ) as usize
        };
        if hdl == 0 {
            self.log(&format!("Couldn't get handle from name {}", name));
            Result::Err(RstbErr)
        } else {
            Result::Ok(hdl)
        }
    }
    fn get_sim_time_steps(&self) -> u64 {
        let mut time_obj = vpi_user::t_vpi_time {
            type_: vpi_user::vpiSimTime as i32,
            ..Default::default()
        };
        unsafe {
            vpi_user::vpi_get_time(std::ptr::null_mut() as *mut u32, &mut time_obj);
        }
        ((time_obj.high as u64) << 32) + time_obj.low as u64
    }
    fn log(&self, msg: &str) {
        let t = self.get_sim_time("ns");
        let int = t.floor() as u64;
        let mut frac_str = format!("{:.3}", t % 1.0);
        frac_str.remove(0);
        let mut string = format!("{}{}ns {}\n\0", int.to_formatted_string(&Locale::en), frac_str, msg);
        // eprintln!("{}", string);
        unsafe { vpi_user::vpi_printf(string.as_mut_ptr() as *mut i8) };
    }
    fn get_kind(&self, obj: usize) -> ObjectKind {
        let t = get_kind_raw(obj);
        let size = unsafe { vpi_user::vpi_get(vpi_user::vpiSize as i32, obj as *mut u32) } as u32;
        // log(&format!("type: {}", t));
        match t as u32 {
            vpi_user::vpiIntegerVar => ObjectKind::Integer,
            vpi_user::vpiRealVar => ObjectKind::Real,
            vpi_user::vpiNet
            | vpi_user::vpiNetBit
            | vpi_user::vpiReg
            | vpi_user::vpiRegBit
            | vpi_user::vpiMemoryWord => ObjectKind::BitVector(size),
            _ => ObjectKind::Unknown,
        }
    }
    fn get_full_name(&self, obj: usize) -> RstbResult<String> {
        unsafe {
            let ptr = vpi_user::vpi_get_str(vpi_user::vpiFullName as i32, obj as *mut u32);
            let s = CStr::from_ptr(check_null(ptr)?).to_owned().into_string().unwrap();
            Ok(s)
        }
    }
    fn get_sim_precision(&self) -> i8 {
        self.precision
    }
    fn get_root_handle(&self) -> RstbResult<usize> {
        let iterator =
            unsafe { vpi_user::vpi_iterate(vpi_user::vpiModule as i32, std::ptr::null_mut()) };
        if iterator.is_null() {
            return Err(RstbErr);
        }
        let root = unsafe { vpi_user::vpi_scan(iterator) };
        if root.is_null() {
            return Err(RstbErr);
        }
        if !unsafe { vpi_user::vpi_scan(iterator).is_null() } {
            unsafe { vpi_user::vpi_free_object(iterator) };
        }
        Ok(root as usize)
    }
    fn register_callback(&self, cb: SimCallback) -> RstbResult<usize> {
        // reason
        let reason = match cb {
            SimCallback::Time(_) => vpi_user::cbAfterDelay as i32,
            SimCallback::Edge(_) => vpi_user::cbValueChange as i32,
            SimCallback::ReadWrite => vpi_user::cbReadWriteSynch as i32,
            SimCallback::ReadOnly => vpi_user::cbReadOnlySynch as i32,
        };
        // time
        let mut time = match cb {
            SimCallback::Time(t) => vpi_user::t_vpi_time {
                type_: vpi_user::vpiSimTime as i32,
                high: (t >> 32) as u32,
                low: (t & 0xFFFF_FFFF) as u32,
                ..Default::default()
            },
            _ => vpi_user::t_vpi_time {
                type_: vpi_user::vpiSuppressTime as i32,
                ..Default::default()
            },
        };
        // value
        let mut value = match cb {
            SimCallback::Edge(_) => vpi_user::t_vpi_value {
                format: vpi_user::vpiIntVal as i32,
                ..Default::default()
            },
            _ => vpi_user::t_vpi_value {
                format: vpi_user::vpiSuppressVal as i32,
                ..Default::default()
            },
        };
        // handle
        let handle = match cb {
            SimCallback::Edge(handle) => handle as *mut u32,
            _ => std::ptr::null_mut(),
        };
        let mut cb_data = vpi_user::t_cb_data {
            reason,
            cb_rtn: Some(react_vpi),
            obj: handle,
            value: &mut value,
            time: &mut time,
            ..Default::default()
        };
        // vpi::log("Registering callback with simulator.");
        // vpi::print_cb_data(&mut cb_data);
        let ret = unsafe { vpi_user::vpi_register_cb(&mut cb_data) };
        // vpi::log("Registeried callback with simulator.");
        Ok(ret as usize)
    }
    fn cancel_callback(&self, cb_hdl: usize) -> RstbResult<()> {
        match unsafe {vpi_user::vpi_remove_cb(cb_hdl as *mut u32) } {
            1 => Ok(()),
            _ => Err(RstbErr)
        }
    }
}

#[no_mangle]
pub extern "C" fn react_vpi(cb_data: *mut vpi_user::t_cb_data) -> vpi_user::PLI_INT32 {
    let cb = unsafe{(*cb_data)
        .to_sim_callback()
        .expect("Invalid callback data received.")};
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
                unsafe{
                    if !(*cb_data).value.is_null() {  // this actually happens under some conditions?
                        edge = match (*(*cb_data).value).value.integer {
                            0 => EdgeKind::Falling,
                            _ => EdgeKind::Rising,
                        };
                    }
                }
            }
            trigger::react(cb, Some(edge));
        },
        _ => trigger::react(cb, None),
    }

    0
}


fn get_time_precision() -> i8 {
    let mut precision =
        unsafe { vpi_user::vpi_get(vpi_user::vpiTimePrecision as i32, std::ptr::null_mut()) };
    if precision > 2 {
        precision = 2;
    }
    if precision < -15 {
        precision = -15;
    }
    precision as i8
}


impl vpi_user::t_cb_data {
    pub fn to_sim_callback(&self) -> Option<SimCallback> {
        const CB_AFTER_DELAY: i32 = vpi_user::cbAfterDelay as i32;
        const CB_VALUE_CHANGE: i32 = vpi_user::cbValueChange as i32;
        const CB_READ_WRITE: i32 = vpi_user::cbReadWriteSynch as i32;
        const CB_READ_ONLY: i32 = vpi_user::cbReadOnlySynch as i32;
        match self.reason {
            CB_AFTER_DELAY => Some(SimCallback::Time(unsafe {
                (u64::from((*self.time).high) << 32) + u64::from((*self.time).low)
            })),
            CB_VALUE_CHANGE => Some(SimCallback::Edge(self.obj as usize)),
            CB_READ_WRITE => Some(SimCallback::ReadWrite),
            CB_READ_ONLY => Some(SimCallback::ReadOnly),
            _ => None,
        }
    }
}


fn is_array(handle: usize) -> bool {
    // let val = unsafe { vpi_user::vpi_get(vpi_user::vpiArray as i32, handle as *mut u32) };
    let val = unsafe { vpi_user::vpi_get(17, handle as *mut u32) };
    let r = matches!(val, 0);
    r
}

pub fn print_vpi_cb_data(cb_data: *mut vpi_user::t_cb_data) {
    unsafe {
        eprintln!("##################################");
        eprintln!("print_cb_data");
        eprintln!("addr: {:p}", cb_data);
        eprintln!("cb_data: {:?}", *cb_data);
        if !(*cb_data).time.is_null() {
            eprintln!("time: {:?}", *(*cb_data).time);
        }
        eprintln!("cb_rtn: {:?}", (*cb_data).cb_rtn);
        eprintln!("##################################");
    }
}

fn get_kind_raw(obj: usize) -> i32 {
    unsafe { vpi_user::vpi_get(vpi_user::vpiType as i32, obj as *mut u32) }
}

fn get_name(obj: usize) -> Option<String> {
    unsafe {
        let ptr = vpi_user::vpi_get_str(vpi_user::vpiName as i32, obj as *mut u32);
        let s = CStr::from_ptr(check_null(ptr).ok()?).to_owned().into_string().unwrap();
        Some(s)
    }
}

fn discover_nets(handle: usize) -> Vec<usize> {
    unsafe {
        let iterator = vpi_user::vpi_iterate(vpi_user::vpiNet as i32, handle as *mut u32);
        let mut list = Vec::new();
        loop {
            let obj = vpi_user::vpi_scan(iterator);
            match obj.is_null() {
                true => break,
                _ => list.push(obj as usize),
            };
        }
        list
    }
}

fn check_null<T>(ptr: *mut T) -> RstbResult<*mut T> {
    if ptr.is_null() {
        Err(RstbErr)
    } else {
        Ok(ptr)
    }
}
