pub mod prelude;
mod executor;
mod seamap;
pub mod signal;
pub mod trigger;


#[cfg(feature = "vpi")]
#[allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
pub mod vpi;
#[cfg(feature = "vhpi")]
mod vhpi;
pub mod sim_if;

#[cfg(feature = "vpi")]
#[allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
mod vpi_user;
#[cfg(feature = "vpi")]
#[allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
pub mod sv_vpi_user;
#[allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
#[cfg(feature = "vhpi")]
mod vhpi_user;
mod rstb_obj;
mod value;

use futures::future::BoxFuture;
use once_cell::sync::OnceCell;
use std::time;
use sim_if::SIM_IF;
use value::RstbValue;


#[derive(Debug)]
pub struct RstbErr;
pub type RstbResult<T> = Result<T, RstbErr>;

pub type VecTestFn = Vec<fn(signal::SimObject) -> BoxFuture<'static, RstbValue>>;

static SIM_START_TIME: OnceCell<time::Instant> = OnceCell::new();
static mut TEST_VEC: Option<VecTestFn> = None;



#[macro_export]
macro_rules! run_with_vpi {
    ($( $i:ident ),+) => {
        #[allow(non_upper_case_globals)]
        #[no_mangle]
        pub static vlog_startup_routines: [Option<extern "C" fn()>; 2] =
            [Some(vpi_entry_point), None];


        #[allow(clippy::vec_init_then_push)]
        #[no_mangle]
        pub extern "C" fn vpi_entry_point() {
            let mut tests: VecTestFn = Vec::new();
            $(tests.push(|sim_root| { $i(sim_root).boxed() });)+
            vpi_init(tests);
        }
    }
}

#[allow(clippy::never_loop)]
#[inline]
fn start_of_simulation() {
    // start timer
    let start = time::Instant::now();
    SIM_START_TIME.get_or_init(move || start);

    let mut test_vec = unsafe { TEST_VEC.take().unwrap() };
    let sim_root = signal::SimObject::get_root().unwrap();


    let first_test = test_vec.remove(0);
    let mut join_handle =  executor::Task::spawn_from_future((first_test)(sim_root), "TEST_0");
    let mut j = 1;
    for test in test_vec {
        join_handle = executor::Task::spawn_from_future(async move {
            join_handle.await;
            // clean up remaining triggers between tests
            trigger::cancel_all_triggers();
            (test)(sim_root).await;
            RstbValue::None
        }, &format!("TEST_{}", j));
        j += 1;
    }
    // execute first simulation tick
    executor::run_once();
}

fn end_of_simulation() {
    let duration = SIM_START_TIME.get().unwrap().elapsed().as_secs_f64();
    let final_sim_time = SIM_IF.get_sim_time("ns");
    let sim_speed = final_sim_time as f64 / duration;

    SIM_IF.log(&format!("Simulation time: {} ns", final_sim_time));
    SIM_IF.log(&format!("Real time: {:.3} s", duration));
    SIM_IF.log(&format!("Simulation speed: {:.3} ns/s", sim_speed));
}


/*
 *  VPI
 */


pub fn vpi_init(tests: VecTestFn) {

    // set tests to execute
    unsafe { TEST_VEC = Some(tests) };

    unsafe {
        let mut cb_data = vpi_user::t_cb_data {
            reason: vpi_user::cbStartOfSimulation as i32,
            cb_rtn: Some(vpi_start_of_simulation),
            ..Default::default()
        };
        vpi_user::vpi_register_cb(&mut cb_data);
    };
    unsafe {
        let mut cb_data = vpi_user::t_cb_data {
            reason: vpi_user::cbEndOfSimulation as i32,
            cb_rtn: Some(vpi_end_of_simulation),
            ..Default::default()
        };
        vpi_user::vpi_register_cb(&mut cb_data);
    };
}

#[no_mangle]
extern "C" fn vpi_start_of_simulation(
    _cb_data: *mut vpi_user::t_cb_data,
) -> vpi_user::PLI_INT32 {
    eprintln!("vpi_start_of_simulation");
    start_of_simulation();
    0
}

#[no_mangle]
extern "C" fn vpi_end_of_simulation(
    _cb_data: *mut vpi_user::t_cb_data,
) -> vpi_user::PLI_INT32 {
    end_of_simulation();
    0
}


/*
 *  VHPI
 */

 #[cfg(feature = "vhpi")]
 #[allow(clippy::missing_safety_doc)]
 #[no_mangle]
 extern "C" fn vhpi_start_of_simulation (
     _cb_data: *const vhpi_user::vhpiCbDataT,
 ) {
    eprintln!("vhpi_start_of_simulation");
    start_of_simulation();
 }


 #[cfg(feature = "vhpi")]
 #[allow(clippy::missing_safety_doc)]
 #[no_mangle]
 extern "C" fn vhpi_end_of_simulation (
     _cb_data: *const vhpi_user::vhpiCbDataT,
 ) {
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