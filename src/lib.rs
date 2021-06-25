mod executor;
pub mod prelude;
mod seamap;
pub mod signal;
pub mod trigger;

pub mod sim_if;
#[cfg(feature = "vhpi")]
mod vhpi;
#[cfg(feature = "vpi")]
#[allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
pub mod vpi;

pub mod rstb_obj;
#[cfg(feature = "vpi")]
#[allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
pub mod sv_vpi_user;
pub mod value;
#[cfg(feature = "vhpi")]
#[allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
mod vhpi_user;
#[cfg(feature = "vpi")]
#[allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
mod vpi_user;
// mod assertions_old;
mod assertion;

use seamap::SeaMap;
use executor::Task;
use futures::future::BoxFuture;
use lazy_mut::lazy_mut;
use once_cell::sync::OnceCell;
use sim_if::SIM_IF;
use std::sync::Arc;
use std::time;
use value::Val;
use assertion::print_assertion_stats;


pub type MsgResult<T> = Result<T, String>;
pub type SimpleResult<T> = Result<T, ()>;
pub type RstbResult = Result<Val, Val>;

pub type VecTestFn = Vec<(
    fn(signal::SimObject) -> BoxFuture<'static, RstbResult>,
    String,
)>;

static SIM_START_TIME: OnceCell<time::Instant> = OnceCell::new();
lazy_mut! { static mut TEST_VEC: VecTestFn = Vec::new(); }
static mut CURRENT_TEST: Option<(Arc<Task>, String)> = None;
lazy_mut! { static mut TEST_RESULTS: SeaMap<String, (bool, String)> = SeaMap::new(); }

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
            // add tests to execution vector
            let mut tests: VecTestFn = Vec::new();
            $(tests.push((|sim_root| { $i(sim_root).boxed() }, stringify!($i).to_string()));)+

            // add failed as default test results
            $(init_test_result(stringify!($i).to_string());)+

            vpi_init(tests);
        }
    }
}
pub fn pass_test(msg: &str) {
    // Passes test has not already failed/passed
    if let Some((test, name)) = unsafe { CURRENT_TEST.take() } {
        set_test_result(name, true, msg.to_string());
        tear_down_test(test);
    }
}

pub fn fail_test(msg: &str) {
    // Fails test has not already failed/passed
    if let Some((test, name)) = unsafe { CURRENT_TEST.take() } {
        set_test_result(name, false, msg.to_string());
        tear_down_test(test);
    }
}

fn tear_down_test(test: Arc<Task>) {
    assertion::tear_down_assertions();
    trigger::cancel_all_triggers();
    executor::clear_ready_queue();
    test.cancel();
}

pub fn set_test_result(name: String, passed: bool, msg: String) {
    unsafe { *TEST_RESULTS.get_mut(&name).unwrap() = (passed, msg) };
}
pub fn init_test_result(name: String) {
    unsafe { TEST_RESULTS.insert(name, (false, "Test result defaults to failed!".to_string())) };
}

fn start_of_simulation() {
    // start timer
    let start = time::Instant::now();
    SIM_START_TIME.get_or_init(move || start);

    let sim_root = signal::SimObject::get_root().unwrap();

    // schedule first test
    let (test, name) = unsafe { TEST_VEC.remove(0) };
    let mut join_handle = executor::Task::spawn_from_future(
        async move {
            let test_handle = executor::Task::spawn_from_future(
                async move {
                    match (test)(sim_root).await {
                        Ok(_) => pass_test(""),
                        Err(_) => fail_test(""),
                    }
                    Ok(Val::None)
                },
                "TEST_INNER_0",
            );
            unsafe { CURRENT_TEST = Some((test_handle.get_task().unwrap(), name)) };
            test_handle.await?;
            Ok(Val::None)
        },
        "TEST_0",
    );

    // schedule subsequent tests
    let n_tests = unsafe { TEST_VEC.len() };
    for j in 0..n_tests {
        if !unsafe { TEST_VEC.is_empty() } {
            join_handle = executor::Task::spawn_from_future(
                async move {
                    join_handle.await;
                    let (test, name) = unsafe { TEST_VEC.remove(0) };
                    let test_handle = executor::Task::spawn_from_future(
                        async move {
                            match (test)(sim_root).await {
                                Ok(_) => pass_test(""),
                                Err(_) => fail_test(""),
                            }
                            Ok(Val::None)
                        },
                        &format!("TEST_INNER_{}", j),
                    );
                    unsafe {
                        CURRENT_TEST = Some((test_handle.get_task().unwrap(), name))
                    };
                    test_handle.await;
                    Ok(Val::None)
                },
                &format!("TEST_{}", j),
            );
        } else {
            break;
        }
    }

    // execute first simulation tick
    executor::run_once();
}

fn end_of_simulation() {
    let duration = SIM_START_TIME.get().unwrap().elapsed().as_secs_f64();
    let final_sim_time = SIM_IF.get_sim_time("ns");
    let sim_speed = final_sim_time as f64 / duration;

    print_assertion_stats();
    SIM_IF.log(&format!("Simulation time: {} ns", final_sim_time));
    SIM_IF.log(&format!("Real time: {:.3} s", duration));
    SIM_IF.log(&format!("Simulation speed: {:.3} ns/s", sim_speed));


    for (name, (passed,msg)) in unsafe {TEST_RESULTS.drain()} {
        let mut result = "Failed";
        if passed {
            result = "Passed";
        }
        SIM_IF.log(&format!("Result of test {}: {}(\"{}\")", name, result, msg));
    }

}

/*
 *  VPI
 */

pub fn vpi_init(tests: VecTestFn) {
    // set tests to execute
    unsafe { *TEST_VEC = tests };

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
extern "C" fn vpi_start_of_simulation(_cb_data: *mut vpi_user::t_cb_data) -> vpi_user::PLI_INT32 {
    eprintln!("vpi_start_of_simulation");
    start_of_simulation();
    0
}

#[no_mangle]
extern "C" fn vpi_end_of_simulation(_cb_data: *mut vpi_user::t_cb_data) -> vpi_user::PLI_INT32 {
    end_of_simulation();
    0
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
