use crate::SimpleResult;

#[cfg(feature = "vhpi")]
use crate::vhpi;
#[cfg(feature = "vpi")]
use crate::vpi;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SIM_IF: Box<dyn SimIf + Sync> = new_interface();
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

pub enum ObjectValue {
    Integer(i32),
    Bit(char),
    BitVector(String),
    Other,
}

#[derive(Copy, Clone, Debug)]
pub enum ObjectKind {
    Bits,
    Real,
    Array,
    Other,
}

pub trait SimIf {
    fn set_value_int(&self, handle: usize, value: i32, force: bool) -> SimpleResult<()>;
    fn get_value_int(&self, obj: usize) -> SimpleResult<i32>;
    fn set_value_bin(&self, obj: usize, value: String, force: bool) -> SimpleResult<()>;
    fn get_value_bin(&self, obj: usize) -> SimpleResult<String>;
    fn release(&self, obj: usize) -> SimpleResult<()>;
    fn get_handle_by_name(&self, name: &str) -> SimpleResult<usize>;
    fn get_sim_time_steps(&self) -> u64;
    fn log(&self, s: &str);
    fn get_size(&self, obj_handle: usize) -> i32;
    fn get_kind(&self, obj_handle: usize) -> ObjectKind;
    fn is_signed(&self, obj_handle: usize) -> bool;
    fn get_full_name(&self, obj: usize) -> SimpleResult<String>;
    fn get_sim_precision(&self) -> i8;
    fn get_root_handle(&self) -> SimpleResult<usize>;
    fn register_callback(&self, cb: SimCallback) -> SimpleResult<usize>; // TODO
    fn cancel_callback(&self, cb_hdl: usize) -> SimpleResult<()>;
    fn get_sim_time(&self, unit: &str) -> f64 {
        // this function does not preserve precision, so don't use carelessly
        let t = self.get_sim_time_steps() as f64;
        let precision = self.get_sim_precision();
        ldexp10(t, precision - time_scale(unit).unwrap())
    }
    fn get_sim_steps(&self, time: f64, unit: &str) -> u64 {
        let precision = self.get_sim_precision();
        let steps = ldexp10(time, time_scale(unit).unwrap() - precision);
        if steps % 1.0 == 0.0 {
            steps as u64
        } else {
            panic!(
                "Can't convert time {} {} to sim steps without rounding (sim precision: {})",
                time,
                unit,
                scale_time(precision).unwrap()
            );
        }
    }
}

fn time_scale(unit: &str) -> SimpleResult<i8> {
    match unit {
        "fs" => Ok(-15),
        "ps" => Ok(-12),
        "ns" => Ok(-9),
        "us" => Ok(-6),
        "ms" => Ok(-3),
        "sec" => Ok(0),
        _ => Err(()),
    }
}
fn scale_time(unit: i8) -> SimpleResult<String> {
    match unit {
        -15 => Ok("fs".to_string()),
        -12 => Ok("ps".to_string()),
        -9 => Ok("ns".to_string()),
        -6 => Ok("us".to_string()),
        -3 => Ok("ms".to_string()),
        0 => Ok("sec".to_string()),
        _ => Err(()),
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
