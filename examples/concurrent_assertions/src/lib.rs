#![allow(unreachable_code, unused_must_use)]
use rstb::prelude::*;


async fn test_default(dut: SimObject) -> RstbResult {
    let clk = dut.c("clk");
    let req = dut.c("req");
    run_all_assertions();
    // run_assertion("check_history");
    Task::spawn(testbench::clock(clk, 8, "ns"));

    clk.rising_edge_rw().await;
    for _ in 0..100_000 {
        req.set_u32((utils::rand() < 0.2) as u32);
        clk.rising_edge_rw().await;
    }
    req.set(0);
    utils::clock_cycles(clk, 5).await;
    Ok(Val::None)
}

rstb::run_with_vpi!(assertion_setup, test_default);



// Assertions
async fn assertion_setup(dut: SimObject) -> RstbResult {
    // Concurrent assertions need to be set up once, and can be reused for every test. They are defined by
    // a name which can be used to run/enable/disable this assertion, a trigger which will start the
    // assertion execution (for example a clock edge), a condition which must return `Ok(_)` at the time of
    // the trigger for the execution to be run, a checker which returns `Ok(_)` or `Err(_)` depending on
    // assertion result.
    // Optionally an assertion can record and use past signal values, which are sampled after the trigger
    // event. Since this costs ressources, each signal to be recorded and the history depth must be specified.

    let clk = dut.c("clk");
    let req = dut.c("req");
    let ack = dut.c("ack");
    add_assertion! (
        "check_consequence",                     // name
        Trigger::rising_edge(clk),               // trigger
        async move { check!(req.u32() == 1) },   // condition
        move |_| { async move {                  // checking function
            for _ in 0..3 {
                clk.rising_edge_ro().await;
            }
            check!(ack.u32() == 1)
        }}
    );
    add_assertion! (
        "check_history",                                    // name
        Trigger::rising_edge(clk),                          // trigger
        async move { check!(ack.u32() == 1) },              // condition
        move |ctx: AssertionContext| { async move {         // checking function
            check!(ctx.sig_hist(req, 3) == Val::Int(1))
        }},
        vec![req],                                          // signals of which to record history
        3                                                   // depth of history
    );
    Ok(Val::None)
}
