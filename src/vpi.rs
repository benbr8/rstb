use num_format::{Locale, ToFormattedString};
use std::ffi::CStr;

use crate::sim_if::{ObjectKind, SimCallback, SimIf, SIM_IF};
use crate::trigger;
use crate::trigger::EdgeKind;
use crate::{sv_vpi_user, vpi_user, SimpleResult};

pub(crate) struct Vpi {
    precision: i8,
}

impl Vpi {
    pub fn new() -> Self {
        Vpi {
            precision: get_time_precision(),
        }
    }
    #[inline]
    unsafe fn _register_callback(
        &self,
        reason: i32,
        mut time: vpi_user::t_vpi_time,
        mut value: vpi_user::t_vpi_value,
        sig_hdl: *mut u32,
        cb_fun: unsafe extern "C" fn(*mut vpi_user::t_cb_data) -> vpi_user::PLI_INT32,
    ) -> usize {
        let mut cb_data = vpi_user::t_cb_data {
            reason,
            cb_rtn: Some(cb_fun),
            obj: sig_hdl,
            value: &mut value,
            time: &mut time,
            ..Default::default()
        };
        // vpi::log("Registering callback with simulator.");
        // vpi::print_cb_data(&mut cb_data);
        // vpi::log("Registeried callback with simulator.");
        vpi_user::vpi_register_cb(&mut cb_data) as usize
    }
}

impl SimIf for Vpi {
    fn set_value_int(&self, obj: usize, value: i32, force: bool) -> SimpleResult<()> {
        let mut val = vpi_user::t_vpi_value {
            format: vpi_user::vpiIntVal as i32,
            value: vpi_user::t_vpi_value__bindgen_ty_1 { integer: value },
        };
        let mut time = vpi_user::t_vpi_time {
            type_: vpi_user::vpiSimTime as i32,
            ..Default::default()
        };
        let mut flag = vpi_user::vpiInertialDelay as i32;
        if force {
            flag = vpi_user::vpiForceFlag as i32;
        }
        unsafe {
            vpi_user::vpi_put_value(obj as *mut u32, &mut val, &mut time, flag);
        };
        // TODO: error??
        Ok(())
    }
    fn get_value_int(&self, obj: usize) -> SimpleResult<i32> {
        unsafe {
            let mut val = vpi_user::t_vpi_value {
                format: vpi_user::vpiIntVal as i32,
                value: vpi_user::t_vpi_value__bindgen_ty_1 { integer: 0 },
            };
            vpi_user::vpi_get_value(obj as *mut u32, &mut val);
            if val.format == vpi_user::vpiIntVal as i32 {
                Ok(val.value.integer)
            } else {
                Err(())
            }
        }
    }
    fn set_value_bin(&self, obj: usize, value: String, force: bool) -> SimpleResult<()> {
        let mut val = value;
        val.push('\0');
        let mut val = vpi_user::t_vpi_value {
            format: vpi_user::vpiBinStrVal as i32,
            value: vpi_user::t_vpi_value__bindgen_ty_1 {
                str_: val.as_mut_ptr() as *mut i8,
            },
        };
        let mut time = vpi_user::t_vpi_time {
            type_: vpi_user::vpiSimTime as i32,
            ..Default::default()
        };
        let mut flag = vpi_user::vpiInertialDelay as i32;
        if force {
            flag = vpi_user::vpiForceFlag as i32;
        }
        unsafe {
            vpi_user::vpi_put_value(obj as *mut u32, &mut val, &mut time, flag);
        };
        // TODO: error??
        Ok(())
    }
    fn get_value_bin(&self, obj: usize) -> SimpleResult<String> {
        unsafe {
            let mut val = vpi_user::t_vpi_value {
                format: vpi_user::vpiBinStrVal as i32,
                value: vpi_user::t_vpi_value__bindgen_ty_1 { integer: 0 },
            };
            vpi_user::vpi_get_value(obj as *mut u32, &mut val);
            if val.format == vpi_user::vpiBinStrVal as i32 {
                let s = CStr::from_ptr(check_null(val.value.str_)?)
                    .to_owned()
                    .into_string()
                    .unwrap();
                Ok(s)
            } else {
                Err(())
            }
        }
    }
    fn release(&self, obj: usize) -> SimpleResult<()> {
        let mut val = vpi_user::t_vpi_value {
            format: vpi_user::vpiIntVal as i32,
            value: vpi_user::t_vpi_value__bindgen_ty_1 { integer: 0 },
        };
        unsafe {
            vpi_user::vpi_put_value(
                obj as *mut u32,
                &mut val,
                std::ptr::null_mut(),
                vpi_user::vpiReleaseFlag as i32,
            );
        };
        Ok(())
    }
    fn get_handle_by_name(&self, name: &str) -> SimpleResult<usize> {
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
            Result::Err(())
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
        let t = self.get_sim_time_f64("ns");
        let int = t.floor() as u64;
        let mut frac_str = format!("{:.3}", t % 1.0);
        frac_str.remove(0);
        let mut string = format!(
            "{}{}ns {}\n\0",
            int.to_formatted_string(&Locale::en),
            frac_str,
            msg
        );
        // eprintln!("{}", string);
        unsafe { vpi_user::vpi_printf(string.as_mut_ptr() as *mut i8) };
    }
    fn get_size(&self, obj: usize) -> i32 {
        unsafe { vpi_user::vpi_get(vpi_user::vpiSize as i32, obj as *mut u32) }
    }
    fn get_kind(&self, obj: usize) -> ObjectKind {
        let t = get_kind_raw(obj);
        match t as u32 {
            vpi_user::vpiRealVar | sv_vpi_user::vpiShortRealVar => ObjectKind::Real,
            vpi_user::vpiNet
            | vpi_user::vpiReg
            | vpi_user::vpiIntegerVar
            | sv_vpi_user::vpiBitVar
            | sv_vpi_user::vpiLongIntVar
            | sv_vpi_user::vpiIntVar => ObjectKind::Bits,
            _ => ObjectKind::Other,
        }
    }
    fn is_signed(&self, obj_handle: usize) -> bool {
        (unsafe { vpi_user::vpi_get(vpi_user::vpiSigned as i32, obj_handle as *mut u32) } != 0)
    }
    fn get_full_name(&self, obj: usize) -> SimpleResult<String> {
        unsafe {
            let ptr = vpi_user::vpi_get_str(vpi_user::vpiFullName as i32, obj as *mut u32);
            let s = CStr::from_ptr(check_null(ptr)?)
                .to_owned()
                .into_string()
                .unwrap();
            Ok(s)
        }
    }
    fn get_sim_precision(&self) -> i8 {
        self.precision
    }
    fn get_root_handle(&self) -> SimpleResult<usize> {
        let mut result: usize;
        let iterator =
            unsafe { vpi_user::vpi_iterate(vpi_user::vpiModule as i32, std::ptr::null_mut()) };
        if iterator.is_null() {
            unsafe { vpi_user::vpi_free_object(iterator) };
            return Err(());
        }
        let root = unsafe { vpi_user::vpi_scan(iterator) };
        if root.is_null() {
            unsafe { vpi_user::vpi_free_object(iterator) };
            return Err(());
        }
        result = root as usize;

        let name = match self.get_full_name(root as usize) {
            Ok(string) => string,
            Err(_) => {
                unsafe { vpi_user::vpi_free_object(iterator) };
                return Err(()); // Cannot get module's name => consider it invalid
            }
        };

        if name.eq("$unit") {
            // According to the SystemVerilog documentation:
            // "Compilation unit: A collection of one or more SystemVerilog source files compiled
            // together."
            // Looking if an other module if available:
            let othermod = unsafe { vpi_user::vpi_scan(iterator) };
            if !othermod.is_null() {
                // If there is an other module
                result = othermod as usize;
            } else {
                unsafe { vpi_user::vpi_free_object(iterator) };
                return Err(());
            }
        }
        unsafe { vpi_user::vpi_free_object(iterator) }; // Free memory used by the iterator
        Ok(result)
    }
    fn register_callback_rw(&self) -> SimpleResult<usize> {
        const reason: i32 = vpi_user::cbReadWriteSynch as i32;
        let time = vpi_user::t_vpi_time {
            type_: vpi_user::vpiSuppressTime as i32,
            ..Default::default()
        };
        let value = vpi_user::t_vpi_value {
            format: vpi_user::vpiSuppressVal as i32,
            ..Default::default()
        };
        let handle = std::ptr::null_mut();
        Ok(unsafe { self._register_callback(reason, time, value, handle, react_vpi_rw) })
    }
    fn register_callback_ro(&self) -> SimpleResult<usize> {
        const reason: i32 = vpi_user::cbReadOnlySynch as i32;
        let time = vpi_user::t_vpi_time {
            type_: vpi_user::vpiSuppressTime as i32,
            ..Default::default()
        };
        let value = vpi_user::t_vpi_value {
            format: vpi_user::vpiSuppressVal as i32,
            ..Default::default()
        };
        let handle = std::ptr::null_mut();
        Ok(unsafe { self._register_callback(reason, time, value, handle, react_vpi_ro) })
    }
    fn register_callback_time(&self, t: u64) -> SimpleResult<usize> {
        const reason: i32 = vpi_user::cbAfterDelay as i32;
        let time = vpi_user::t_vpi_time {
            type_: vpi_user::vpiSimTime as i32,
            high: (t >> 32) as u32,
            low: (t & 0xFFFF_FFFF) as u32,
            ..Default::default()
        };
        let value = vpi_user::t_vpi_value {
            format: vpi_user::vpiSuppressVal as i32,
            ..Default::default()
        };
        let handle = std::ptr::null_mut();
        Ok(unsafe { self._register_callback(reason, time, value, handle, react_vpi_time) })
    }
    fn register_callback_edge(&self, sig_hdl: usize) -> SimpleResult<usize> {
        const reason: i32 = vpi_user::cbValueChange as i32;
        let time = vpi_user::t_vpi_time {
            type_: vpi_user::vpiSuppressTime as i32,
            ..Default::default()
        };
        let value = vpi_user::t_vpi_value {
            format: vpi_user::vpiIntVal as i32,
            ..Default::default()
        };
        let handle = sig_hdl as *mut u32;
        Ok(unsafe { self._register_callback(reason, time, value, handle, react_vpi_edge) })
    }
    fn cancel_callback(&self, cb_hdl: usize) -> SimpleResult<()> {
        match unsafe { vpi_user::vpi_remove_cb(cb_hdl as *mut u32) } {
            1 => Ok(()),
            _ => Err(()),
        }
    }
}

#[no_mangle]
pub(crate) extern "C" fn react_vpi_edge(cb_data: *mut vpi_user::t_cb_data) -> vpi_user::PLI_INT32 {
    let hdl = unsafe { (*cb_data).obj as usize };
    let mut edge = EdgeKind::Any;
    if SIM_IF.get_size(hdl) == 1 {
        unsafe {
            if !(*cb_data).value.is_null() {
                // this actually happens under some conditions?
                edge = match (*(*cb_data).value).value.integer {
                    0 => EdgeKind::Falling,
                    _ => EdgeKind::Rising,
                }
            }
        };
    }
    trigger::react_edge(hdl, edge);
    0
}

#[no_mangle]
pub(crate) extern "C" fn react_vpi_time(cb_data: *mut vpi_user::t_cb_data) -> vpi_user::PLI_INT32 {
    let t = unsafe { (*cb_data).decode_time() };
    trigger::react_time(t);
    0
}

#[no_mangle]
pub(crate) extern "C" fn react_vpi_ro(_: *mut vpi_user::t_cb_data) -> vpi_user::PLI_INT32 {
    trigger::react_ro();
    0
}

#[no_mangle]
pub(crate) extern "C" fn react_vpi_rw(_: *mut vpi_user::t_cb_data) -> vpi_user::PLI_INT32 {
    trigger::react_rw();
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
    pub(crate) unsafe fn decode_time(&self) -> u64 {
        // if self.reason != vpi_user::cbAfterDelay as i32 {
        //     panic!("Unexpected callback type. Expected 'Time'.");
        // }
        (u64::from((*self.time).high) << 32) + u64::from((*self.time).low)
    }
    pub(crate) fn to_sim_callback(&self) -> Option<SimCallback> {
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

pub(crate) fn print_vpi_cb_data(cb_data: *mut vpi_user::t_cb_data) {
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

pub fn get_size_raw(obj: usize) -> i32 {
    unsafe { vpi_user::vpi_get(vpi_user::vpiSize as i32, obj as *mut u32) }
}

pub fn get_kind_raw(obj: usize) -> i32 {
    unsafe { vpi_user::vpi_get(vpi_user::vpiType as i32, obj as *mut u32) }
}

fn get_name(obj: usize) -> Option<String> {
    unsafe {
        let ptr = vpi_user::vpi_get_str(vpi_user::vpiName as i32, obj as *mut u32);
        let s = CStr::from_ptr(check_null(ptr).ok()?)
            .to_owned()
            .into_string()
            .unwrap();
        Some(s)
    }
}

pub fn discover_nets(handle: usize) -> Vec<usize> {
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

fn check_null<T>(ptr: *mut T) -> SimpleResult<*mut T> {
    if ptr.is_null() {
        Err(())
    } else {
        Ok(ptr)
    }
}
