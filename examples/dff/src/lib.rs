#![allow(unreachable_code, dead_code, unused_must_use)]
mod tb_utils;

use rstb::prelude::*;

async fn clk_stim(clk: SimObject, period_ns: u64) -> RstbResult {
    loop {
        Trigger::timer_rw(period_ns/2, "ns").await;
        clk.set(1);
        Trigger::timer_rw(period_ns/2, "ns").await;
        clk.set(0);
    }
    Ok(Val::None)
}

async fn test_dff(dut: SimObject) -> RstbResult {

    // Fork clock input to run concurrently
    Task::fork(clk_stim(dut.c("clk"), 10));

    for _ in 0..100_000 {
        let q = utils::rand_int(2);
        dut.c("d").set_u32(q);
        dut.c("clk").rising_edge_rw().await;
        if dut.c("q").u32() != q {
            fail_test("Q output did not match D input")
        }
    }

    pass_test("All transactions matched");
    Ok(Val::None)
}

// Specify tests to be executed
rstb::run_with_vpi!(test_dff);

