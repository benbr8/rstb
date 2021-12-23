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

// #[allow()]
impl SimIf for Verilator {
    fn set_value(&self, obj: &SimObject, value: u32, force: bool) -> SimpleResult<()> {
        force_panic(force);
        if let ObjectKind::Int(size) = obj.kind {
            match size {
                8 => unsafe { vl_set_var_u8(obj.handle, value as u8) },
                16 => unsafe { vl_set_var_u16(obj.handle, value as u16) },
                32 => unsafe { vl_set_var_u32(obj.handle, value as u32) },
                _ =>  { crate::cold(); return Err(()) }
            };
            Ok(())
        } else {
            Err(())
        }
    }
    fn get_value(&self, obj: &SimObject) -> SimpleResult<u32> {
        if let ObjectKind::Int(size) = obj.kind {
            match size {
                8 => unsafe { Ok(vl_get_var_u8(obj.handle) as u32) },
                16 => unsafe { Ok(vl_get_var_u16(obj.handle) as u32) },
                32 => unsafe { Ok(vl_get_var_u32(obj.handle)) },
                _ =>  { crate::cold(); Err(()) }
            }
        } else {
            Err(())
        }
    }
    fn get_object_by_name(&self, name: &str) -> SimpleResult<SimObject> {
        let cstr = CString::new(name).unwrap();
        let scope_hdl = unsafe { vl_get_scope_by_name(cstr.as_ptr()) };
        if scope_hdl != 0 {
            return Ok(SimObject {
                handle: scope_hdl,
                kind: ObjectKind::Hier,
            })
        }
        // if name is not a scope, split string on last '.' -> try to get scope and var in scope
        if let Some((scope_name, var_name)) = name.rsplit_once('.') {
            let cstr = CString::new(scope_name).unwrap();
            let scope_hdl = unsafe { vl_get_scope_by_name(cstr.as_ptr()) };
            if scope_hdl != 0 {
                let cstr = CString::new(var_name).unwrap();
                let var_hdl = unsafe { vl_get_var_by_name(scope_hdl, cstr.as_ptr()) };
                let var_size = type_to_size(unsafe { vl_get_var_type(var_hdl) }).unwrap();
                return Ok(SimObject {
                    handle: var_hdl,
                    kind: ObjectKind::Int(var_size),
                });
            }
        }
        Err(())
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
            _ => panic!("Verilator does not expose full name from var handle.")
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
        let hdl = unsafe { vl_get_root_scope() };
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


#[inline]
fn force_panic(force: bool) {
    if force {
        panic!("Force/Release not supported for Verilator.")
    }
}

fn type_to_size(type_: u8) -> SimpleResult<i32> {
    match type_ {
        2 => Ok(8),
        3 => Ok(16),
        4 => Ok(32),
        5 => Ok(64),
        _ => Err(())
    }
}