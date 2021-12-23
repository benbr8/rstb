use crate::SimpleResult;
use crate::signal::{ObjectKind, SimObject};

#[cfg(feature = "vhpi")]
use crate::vhpi;
#[cfg(feature = "vpi")]
use crate::vpi;
#[cfg(feature = "verilator")]
use crate::verilator;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SIM_IF: Box<dyn SimIf + Sync> = new_interface();
}

#[cfg(feature = "verilator")]
fn new_interface() -> Box<dyn SimIf + Sync> {
    Box::new(verilator::Verilator{})
}
#[cfg(feature = "vpi")]
fn new_interface() -> Box<dyn SimIf + Sync> {
    Box::new(vpi::Vpi::new())
}
#[cfg(feature = "vhpi")]
fn new_interface() -> Box<dyn SimIf + Sync> {
    Box::new(vhpi::Vhpi {})
}


#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub enum SimCallback {
    Time(u64),
    Edge(usize),
    ReadWrite,
    ReadOnly,
}


#[allow(clippy::result_unit_err, unused_variables)]
pub trait SimIf {
    fn set_value_i32(&self, obj: &SimObject, value: i32, force: bool) -> SimpleResult<()> { unimplemented!() }
    fn get_value_i32(&self, obj: &SimObject) -> SimpleResult<i32> { unimplemented!() }

    fn set_value_u8(&self, obj: &SimObject, value: u8, force: bool) -> SimpleResult<()> { unimplemented!() }
    fn get_value_u8(&self, obj: &SimObject) -> SimpleResult<u8> { unimplemented!() }

    fn set_value_u16(&self, obj: &SimObject, value: u16, force: bool) -> SimpleResult<()> { unimplemented!() }
    fn get_value_u16(&self, obj: &SimObject) -> SimpleResult<u16> { unimplemented!() }
    
    fn set_value_u32(&self, obj: &SimObject, value: u32, force: bool) -> SimpleResult<()> { unimplemented!() }
    fn get_value_u32(&self, obj: &SimObject) -> SimpleResult<u32> { unimplemented!() }
    
    fn set_value_u64(&self, obj: &SimObject, value: u64, force: bool) -> SimpleResult<()> { unimplemented!() }
    fn get_value_u64(&self, obj: &SimObject) -> SimpleResult<u64> { unimplemented!() }
    
    fn set_value_u128(&self, obj: &SimObject, value: u128, force: bool) -> SimpleResult<()> { unimplemented!() }
    fn get_value_u128(&self, obj: &SimObject) -> SimpleResult<u128> { unimplemented!() }

    fn set_value_bin(&self, obj: &SimObject, value: String, force: bool) -> SimpleResult<()> { unimplemented!() }
    fn get_value_bin(&self, obj: &SimObject) -> SimpleResult<String> { unimplemented!() }

    fn release(&self, obj: &SimObject) -> SimpleResult<()>;
    fn get_handle_by_name(&self, name: &str) -> SimpleResult<usize>;
    fn get_sim_time_steps(&self) -> u64;
    fn log(&self, s: &str);
    fn get_size(&self, obj_handle: usize) -> i32;
    fn get_kind(&self, obj_handle: usize) -> ObjectKind;
    // fn is_signed(&self, obj_handle: usize) -> bool;
    fn get_full_name(&self, obj: &SimObject) -> SimpleResult<String>;
    fn get_sim_precision(&self) -> i8;
    fn get_root_object(&self) -> SimpleResult<SimObject>;
    fn register_callback_rw(&self) -> SimpleResult<usize>;
    fn register_callback_ro(&self) -> SimpleResult<usize>;
    fn register_callback_time(&self, t: u64) -> SimpleResult<usize>;
    fn register_callback_edge(&self, sig_hdl: usize) -> SimpleResult<usize>;
    fn cancel_callback(&self, cb_hdl: usize) -> SimpleResult<()>;
    fn get_sim_time(&self, unit: &str) -> u64 {
        let precision = self.get_sim_precision();
        let scale = time_scale(unit);
        if scale < precision {
            panic!("Provided time unit is smaller than simulator precision.")
        }
        let steps = self.get_sim_time_steps();
        let steps_per_unit = 10_u64.pow((scale-precision) as u32);
        if steps % steps_per_unit != 0 {
            panic!("Current simulation time cannot accurately be converted to specified precision.")
        }
        steps / steps_per_unit
    }
    fn get_sim_time_f64(&self, unit: &str) -> f64 {
        // this function does not preserve precision, so don't use carelessly
        let t = self.get_sim_time_steps() as f64;
        let precision = self.get_sim_precision();
        ldexp10(t, precision - time_scale(unit))
    }
    fn get_sim_steps(&self, time: f64, unit: &str) -> u64 {
        let precision = self.get_sim_precision();
        let steps = ldexp10(time, time_scale(unit) - precision);
        if steps % 1.0 == 0.0 {
            steps as u64
        } else {
            panic!(
                "Can't convert time {} {} to sim steps without rounding (sim precision: {})",
                time,
                unit,
                scale_time(precision)
            );
        }
    }
}

fn time_scale(unit: &str) -> i8 {
    match unit {
        "fs" => -15,
        "ps" => -12,
        "ns" => -9,
        "us" => -6,
        "ms" => -3,
        "s" | "sec" => 0,
        _ => panic!("Did not recognize time unit.")
    }
}
fn scale_time(unit: i8) -> String {
    match unit {
        -15 => "fs".to_string(),
        -12 => "ps".to_string(),
        -9 => "ns".to_string(),
        -6 => "us".to_string(),
        -3 => "ms".to_string(),
        0 => "sec".to_string(),
        _ => panic!("Provided precision does not have a name."),
    }
}

fn ldexp10(frac: f64, exp: i8) -> f64 {
    // Like math.ldexp, but base 10
    // Stolen from cocotb.
    if exp >= 0 {
        frac * 10_u64.pow(exp as u32) as f64
    } else {
        let div = 10_u64.pow(-exp as u32) as f64;
        frac / div
    }
}
