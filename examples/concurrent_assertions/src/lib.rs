#![allow(unreachable_code, unused_must_use)]
use rstb::prelude::*;
use rand::{Rng, thread_rng};

#[allow(unreachable_code)]
async fn clock(clk: SimObject, period: u32, unit: &str) -> RstbResult {
    let half_period = period / 2;
    loop {
        clk.set(0);
        Trigger::timer(half_period as u64, unit).await;
        clk.set(1);
        Trigger::timer(half_period as u64, unit).await;
    }
    Ok(Val::None)
}

async fn test_default(dut: SimObject) -> RstbResult {
    let clk = dut.c("clk");
    let req = dut.c("req");
    run_all_assertions();

    Task::spawn(clock(clk, 8, "ns"));

    for _ in 0..10_000 {
        Trigger::rising_edge(clk).await;
        Trigger::read_write().await;
        let rnd = thread_rng().gen_weighted_bool(5);
        req.set(rnd as i32);
    }
    disable_all_assertions();
    Trigger::timer(100, "ns").await;
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

    add_assertion! (
        "check_consequence",                                // name
        Trigger::rising_edge(dut.c("clk")),                 // trigger
        async move { check!(dut.c("req").u32() == 1) },     // condition
        move |ctx: AssertionContext| { async move {         // checking function
            for _ in 0..3 {
                ctx.trig().await;
            }
            Ok(Val::None)
        }}
    );
    add_assertion! (
        "check_history",
        Trigger::rising_edge(dut.c("clk")),
        async move { check!(dut.c("ack").u32() == 1) },
        req_3_before,
        vec![dut.c("req")],                                 // signals of which to record history
        3                                                   // depth of history
    );
    Ok(Val::None)
}

async fn req_3_before(ctx: AssertionContext) -> RstbResult {
    let dut = ctx.dut();
    check!(ctx.sig_hist(dut.c("req"), 3) == Val::Int(1))
}