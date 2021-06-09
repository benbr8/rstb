use crate::{vhpi, vpi, RstbErr, RstbResult};
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;

static IF_KIND: OnceCell<SimIfKind> = OnceCell::new();

lazy_static! {
    pub static ref SIM_IF: Box<dyn SimIf + Sync> = match IF_KIND.get().unwrap() {
        SimIfKind::Vhpi => Box::new(vhpi::Vhpi {}),
        SimIfKind::Vpi => Box::new(vpi::Vpi::new()),
    };
}

#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub enum SimCallback {
    Time(u64),
    Edge(usize),
    ReadWrite,
    ReadOnly,
}

enum SimIfKind {
    Vhpi,
    Vpi,
}

pub fn set_sim_if(variant: &str) {
    IF_KIND.get_or_init(|| {
        if variant.to_lowercase().eq("vhpi") {
            SimIfKind::Vhpi
        } else if variant.to_lowercase().eq("vpi") {
            SimIfKind::Vpi
        } else {
            panic!("{} is not a valid SimIfKind.", variant);
        }
    });
}

pub enum ObjectValue {
    Integer(i32),
    Bit(char),
    BitVector(String),
    Other,
}

#[derive(Copy, Clone, Debug)]
pub enum ObjectKind {
    Integer,
    Real,
    BitVector(u32),
    Array(u32),
    Unknown,
}

pub trait SimIf {
    fn set_value_int(&self, handle: usize, value: i32) -> RstbResult<()>;
    fn get_value_int(&self, obj: usize) -> RstbResult<i32>;
    fn get_handle_by_name(&self, name: &str) -> RstbResult<usize>;
    fn get_sim_time_steps(&self) -> u64;
    fn log(&self, s: &str);
    fn get_kind(&self, obj_handle: usize) -> ObjectKind;
    fn get_full_name(&self, obj: usize) -> RstbResult<String>;
    fn get_sim_precision(&self) -> i8;
    fn get_root_handle(&self) -> RstbResult<usize>;
    fn register_callback(&self, cb: SimCallback) -> RstbResult<usize>; // TODO
    fn cancel_callback(&self, cb_hdl: usize) -> RstbResult<()>;
    fn get_sim_time(&self, unit: &str) -> u64 {
        let t = self.get_sim_time_steps();
        let precision = self.get_sim_precision();
        ldexp10(t, precision - time_scale(unit).unwrap()).unwrap()
    }
    fn get_sim_steps(&self, time: u64, unit: &str) -> u64 {
        let precision = self.get_sim_precision();
        ldexp10(time, time_scale(unit).unwrap() - precision).unwrap()
    }
}

fn time_scale(unit: &str) -> RstbResult<i8> {
    match unit {
        "fs" => Ok(-15),
        "ps" => Ok(-12),
        "ns" => Ok(-9),
        "us" => Ok(-6),
        "ms" => Ok(-3),
        "sec" => Ok(0),
        _ => Err(RstbErr),
    }
}

fn ldexp10(frac: u64, exp: i8) -> RstbResult<u64> {
    // Like math.ldexp, but base 10
    // Stolen from cocotb.
    if exp >= 0 {
        Ok(frac * 10_u64.pow(exp as u32))
    } else {
        let div = 10_u64.pow(-exp as u32);
        if frac % div != 0 {
            Err(RstbErr)
        } else {
            Ok(frac / div)
        }
    }
}