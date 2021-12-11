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

#[allow(clippy::result_unit_err)]
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
