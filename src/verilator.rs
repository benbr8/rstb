use num_format::{Locale, ToFormattedString};
use std::ffi::{CString, CStr};
use std::sync::atomic::AtomicBool;

use crate::sim_if::{SimCallback, SimIf, SIM_IF};
use crate::signal::{ObjectKind, SimObject};
use crate::trigger;
use crate::trigger::EdgeKind;
use crate::test;
use crate::SimpleResult;
use crate::rstb_obj::RstbObjSafe;
use std::collections::{BTreeMap, BTreeSet};
use intmap::IntMap;
use lazy_static::lazy_static;
use crate::verilator_user::*;


enum CbKind {
    Time(u64),
    Edge(usize),
    Ro,
}

// TODO: use unsafe for performance?
lazy_static! {
    static ref CB_HDL_CNT: RstbObjSafe<usize> = RstbObjSafe::new(0);
}
lazy_static! {
    static ref CB_HDL_MAP: RstbObjSafe<IntMap<CbKind>> = RstbObjSafe::new(IntMap::new());
}
static RO: AtomicBool = AtomicBool::new(false);
lazy_static! {
    static ref TIME_SET: RstbObjSafe<BTreeSet<u64>> = RstbObjSafe::new(BTreeSet::new());
}

pub(crate) struct Verilator {
}

impl Verilator {
}

#[inline]
fn force_panic(force: bool) {
    if force {
        panic!("Force/Release not supported for Verilator.")
    }
}

// #[allow()]
impl SimIf for Verilator {
    fn set_value_i32(&self, obj: &SimObject, value: i32, force: bool) -> SimpleResult<()> {
        force_panic(force);
        todo!()
    }
    fn get_value_i32(&self, obj: &SimObject) -> SimpleResult<i32> {
        todo!();
    }
    fn set_value_bin(&self, obj: &SimObject, value: String, force: bool) -> SimpleResult<()> {
        force_panic(force);
        todo!()
    }
    fn get_value_bin(&self, _: &SimObject) -> SimpleResult<String> {
        panic!("Getting value as binary string is not implemented for Verilator.");
    }
    fn release(&self, _: &SimObject) -> SimpleResult<()> {
        force_panic(true);
        Ok(())
    }
    fn get_handle_by_name(&self, name: &str) -> SimpleResult<usize> {
        let cstr = CString::new(name).unwrap();
        let scope_hdl = unsafe { vl_get_scope_handle_by_name(cstr.as_ptr()) };
        if scope_hdl != 0 {
            return Ok(scope_hdl);
        }
        todo!() // split string at last '.' -> get scope -> get var of scope
        // refactor into get_obj_by_name --> return SimObject instead of handle
    }
    fn get_sim_time_steps(&self) -> u64 {
        unsafe { vl_get_time() }
    }
    fn log(&self, msg: &str) {
        // TODO: make pretty
        println!("{}", msg);
    }
    fn get_size(&self, obj: usize) -> i32 {
        todo!()
    }
    fn get_kind(&self, obj: usize) -> ObjectKind {
        todo!()
    }
    fn get_full_name(&self, obj: &SimObject) -> SimpleResult<String> {
        match obj.kind {
            ObjectKind::Hier => unsafe { 
                let cstr = CStr::from_ptr(vl_get_scope_name(obj.handle));
                Ok(cstr.to_str().unwrap().to_string())
            },
            _ => todo!()
        }
    }
    #[allow(unused_variables)]
    fn get_sim_time(&self, unit: &str) -> u64 {
        self.log("Warning: sim time forced to 'steps' when using Verilator.");
        self.get_sim_time_steps()
    }
    fn get_sim_precision(&self) -> i8 {
        panic!("Verilator does not expose simulation precision")
    }
    fn get_root_object(&self) -> SimpleResult<SimObject> {
        let hdl = unsafe { vl_get_root_scope_handle() };
        match hdl {
            0 => Err(()),
            _ => Ok(SimObject{
                handle: hdl,
                kind: ObjectKind::Hier,
            })
        }
    }
    fn register_callback_rw(&self) -> SimpleResult<usize> {
        panic!("Verilator does not support RW callbacks");
    }
    fn register_callback_ro(&self) -> SimpleResult<usize> {
        RO.store(true, std::sync::atomic::Ordering::Relaxed);
        let cb_hdl = new_cb_hdl();
        CB_HDL_MAP.with_mut(|mut map| {
            map.insert(cb_hdl as u64, CbKind::Ro);
        });
        Ok(cb_hdl)
    }
    fn register_callback_time(&self, t: u64) -> SimpleResult<usize> {
        let t_abs = t + unsafe { vl_get_time() };
        let cb_hdl = new_cb_hdl();
        
        TIME_SET.with_mut(|mut set| {
            if !set.insert(t_abs) {
                panic!("Can not register same timer callback twice.");
            }
        });
        CB_HDL_MAP.with_mut(|mut map| {
            map.insert(cb_hdl as u64, CbKind::Time(t_abs));
        });
        Ok(cb_hdl)
    }
    fn register_callback_edge(&self, sig_hdl: usize) -> SimpleResult<usize> {
        todo!()
    }
    fn cancel_callback(&self, cb_hdl: usize) -> SimpleResult<()> {
        let cb = CB_HDL_MAP.with_mut(|mut map| {
            map.remove(cb_hdl as u64).expect("Could not find callback handle.")
        });
        match cb {
            CbKind::Time(t_abs) => TIME_SET.with_mut(|mut set| {
                if !set.remove(&t_abs) {
                    panic!("Callback was not registered at t_abs.")
                }
            }),
            _ => todo!()
        };
        Ok(())
    }
}


#[no_mangle]
pub(crate) extern "C" fn react_vl_edge() {
    todo!()
}

fn check_null<T>(ptr: *mut T) -> SimpleResult<*mut T> {
    if ptr.is_null() {
        Err(())
    } else {
        Ok(ptr)
    }
}

fn new_cb_hdl() -> usize {
    CB_HDL_CNT.with_mut(|mut cnt| {
        let out = cnt.clone();
        *cnt += 1;
        out
    })
}

pub fn verilator_init(tests: test::RstbTests) {
    // set tests to execute
    test::TESTS.set(tests).unwrap();
    unsafe { vl_init(); }
    crate::start_of_simulation();
    run_sim();
    crate::end_of_simulation();
}

fn handle_time_callbacks() {
    let t = unsafe { vl_get_time() };
    if TIME_SET.with_mut(|mut set|{
        set.remove(&t) 
    }) {
        trigger::react_time(t);
    }
}
fn handle_ro_callbacks() {
    if RO.fetch_and(false, std::sync::atomic::Ordering::Relaxed) {
        trigger::react_ro();
    }
}

fn get_next_time() -> Option<u64> {
    TIME_SET.with_mut(|set| {
        set.iter().next().cloned()
    })
}

fn run_sim() {
    // let mut next_time = None;
    loop {
        handle_time_callbacks();
        unsafe { vl_eval(); }
        // handle_edge_callbacks();
        handle_ro_callbacks();
        if let Some(next_time) = get_next_time() {
            unsafe { vl_set_time(next_time); }
        } else {
            break;
        }
    }
    unsafe { vl_finalize(); }
}


#[macro_export]
macro_rules! run_with_verilator {
    ($( $i:ident ),+) => {
        #[allow(clippy::vec_init_then_push)]
        fn main() {
            CRATE_NAME.set(std::module_path!().to_string()).unwrap();
            // add tests to execution vector
            let mut tests = RstbTests::new();
            $(tests.push(Test::new(stringify!($i).to_string(), |sim_root| { $i(sim_root).boxed() }));)+

            verilator_init(tests);
        }
    }
}