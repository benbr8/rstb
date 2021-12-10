mod executor;
pub mod prelude;
mod seamap;
mod signal;
mod trigger;
mod rstb_obj;
mod value;
mod assertion;
mod test;
mod junit;
pub mod sim_if;
pub mod utils;
pub mod testbench;
#[cfg(feature = "vhpi")]
mod vhpi;
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
pub mod vpi;

#[cfg(feature = "vpi")]
#[allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
mod sv_vpi_user;
#[cfg(feature = "vpi")]
#[allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    clippy::upper_case_acronyms
)]
mod vpi_user;

use executor::Task;
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use rstb_obj::RstbObjSafe;
use sim_if::SIM_IF;
use std::sync::Arc;
use std::time;
use value::Val;
use test::{RstbTests, Test};


pub type SimpleResult<T> = Result<T, ()>;
pub type RstbResult = Result<Val, Val>;


static SIM_START_TIME: OnceCell<time::Instant> = OnceCell::new();
lazy_static! { static ref TEST_START_TIME: RstbObjSafe<Option<time::Instant>> = RstbObjSafe::new(None); }

// static mut CURRENT_TEST: Option<(Arc<Task>, String)> = None;
lazy_static!{ static ref CURRENT_TEST: RstbObjSafe<Option<(Arc<Task>, RstbObjSafe<test::Test>)>> = RstbObjSafe::new(None); }
 
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
            let mut tests = RstbTests::new();
            $(tests.push(Test::new(stringify!($i).to_string(), |sim_root| { $i(sim_root).boxed() }));)+

            // add failed as default test results
            // $(init_test_result(stringify!($i).to_string());)+

            vpi_init(tests);
        }
    }
}
pub fn pass_test(msg: &str) {
    // Passes test that has not already failed/passed
    if let Some((task, test)) = CURRENT_TEST.get().take() {
        test.with_mut(|mut t| t.set_result(Ok(Val::String(msg.to_string()))));
        tear_down_test(task);
    }
}

pub fn fail_test(msg: &str) {
    // Fails test that has not already failed/passed
    if let Some((task, test)) = CURRENT_TEST.get().take() {
        test.with_mut(|mut t| t.set_result(Err(Val::String(msg.to_string()))));
        tear_down_test(task);
    }
}

fn tear_down_test(test: Arc<Task>) {
    assertion::tear_down_assertions();
    trigger::cancel_all_triggers();
    executor::clear_ready_queue();
    rstb_obj::clear_objects();
    test.cancel();
}


fn start_of_simulation() {
    // start timer
    let start = time::Instant::now();
    SIM_START_TIME.get_or_init(move || start);

    let sim_root = signal::SimObject::get_root().unwrap();

    // schedule first test
    // let (test, name) = unsafe { TEST_VEC.remove(0) };
    // let mut join_handle = executor::Task::spawn_from_future(
    //     async move {
    //         let test_handle = executor::Task::spawn_from_future(
    //             async move {
    //                 TEST_START_TIME.with_mut(|t| {
    //                     t.replace(time::Instant::now());
    //                 });
    //                 match (test)(sim_root).await {
    //                     Ok(val) => pass_test(&format!("{:?}", val)),
    //                     Err(val) => fail_test(&format!("{:?}", val)),
    //                 }

    //                 Ok(Val::None)
    //             }
    //         );
    //         unsafe { CURRENT_TEST = Some((test_handle.get_task().unwrap(), name)) };
    //         test_handle.await?;
    //         Ok(Val::None)
    //     }
    // );

    // // schedule subsequent tests
    // let n_tests = unsafe { TEST_VEC.len() };
    // for _ in 0..n_tests {
    //     if !unsafe { TEST_VEC.is_empty() } {
    //         join_handle = executor::Task::spawn_from_future(
    //             async move {
    //                 let _ = join_handle.await;
    //                 let (test, name) = unsafe { TEST_VEC.remove(0) };
    //                 let test_handle = executor::Task::spawn_from_future(
    //                     async move {
    //                         match (test)(sim_root).await {
    //                             Ok(val) => pass_test(&format!("{:?}", val)),
    //                             Err(val) => fail_test(&format!("{:?}", val)),
    //                         }
    //                         Ok(Val::None)
    //                     }
    //                 );
    //                 unsafe {
    //                     CURRENT_TEST = Some((test_handle.get_task().unwrap(), name))
    //                 };
    //                 let _ = test_handle.await;
    //                 Ok(Val::None)
    //             }
    //         );
    //     } else {
    //         break;
    //     }
    // }

    let tests = test::TESTS.get().unwrap();
    let mut join_handle = None;
    for test in tests.iter() {
        join_handle = Some(executor::Task::spawn_from_future(
            async move {
                if let Some(handle) = join_handle {
                    let _ = handle.await;
                }
                let test_handle = executor::Task::spawn_from_future(
                    async move {
                        let generator = test.get().generator;
                        match (generator)(sim_root).await {
                            Ok(val) => pass_test(&format!("{:?}", val)),
                            Err(val) => fail_test(&format!("{:?}", val)),
                        }
                        Ok(Val::None)
                    }
                );
                let test_task = test_handle.get_task().unwrap().clone();
                CURRENT_TEST.with_mut(move |mut c| {
                    let test = test.clone();
                    let _ = c.replace((test_task, test));
                });
                let _ = test_handle.await;
                Ok(Val::None)
            }
        ));
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

    assertion::print_assertion_stats();
    for (name, (passed,msg)) in unsafe { test::TEST_RESULTS.drain() } {
        let result = match passed {
            true => "Passed",
            false => "Failed",
        };
        SIM_IF.log(&format!("Result of test {}: {}({})", name, result, msg));
    }
}

/*
 *  VPI
 */

pub fn vpi_init(tests: test::RstbTests) {
    // set tests to execute
    // unsafe { *TEST_VEC = tests };
    test::TESTS.set(tests).unwrap();

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
