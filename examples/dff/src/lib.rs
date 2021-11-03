#![allow(unreachable_code)]
mod tb_utils;

use rstb::prelude::*;
use tb_utils::*;

#[derive(Clone)]
struct DffTb {
    scoreboard: RstbObj<Scoreboard<i32>>,
}

impl DffTb {
    fn new() -> Self {
        Self{ scoreboard: RstbObj::new(Scoreboard::new()) }
    }
    async fn monitor_in(self, clk: SimObject, signal: SimObject) -> RstbResult {
        loop {
            Trigger::rising_edge(clk).await;
            // Trigger::read_only().await;
            self.scoreboard.get_mut().add_exp(signal.i32());
        }
        Ok(Val::None)
    }
    async fn monitor_out(self, clk: SimObject, signal: SimObject) -> RstbResult {
        loop {
            Trigger::rising_edge(clk).await;
            // Trigger::read_only().await;
            self.scoreboard.get_mut().add_recv(signal.i32());
        }
        Ok(Val::None)
    }
}

async fn d_stim(clk: SimObject, d: SimObject) -> RstbResult {
    d.set(0);
    loop {
        Trigger::rising_edge(clk).await;
        d.set((d.i32() + 1) % 2);
    }
    Ok(Val::None)
}

async fn reset(dut: SimObject) -> RstbResult {
    let rstn = dut.c("rstn");
    let clk = dut.c("clk");
    rstn.set(0);
    for _ in 0..10 {
        Trigger::rising_edge(clk).await;
    }
    rstn.set(1);
    for _ in 0..10 {
        Trigger::rising_edge(clk).await;
    }

    Ok(Val::None)
}

async fn fail_after_1ms() -> RstbResult {
    Trigger::timer(1, "ms").await;
    fail_test("panic!");

    Ok(Val::None)
}

#[allow(clippy::many_single_char_names)]
pub async fn test_default(dut: SimObject) -> RstbResult {
    let tb = DffTb::new();
    let clk = dut.c("clk");
    let d = dut.c("d");
    let q = dut.c("q");

    // start clock
    let clock_task = Task::fork(clock(clk, 8, "ns"));
    // reset
    reset(dut).await?;

    Task::fork(d_stim(clk, d));
    Task::fork(tb.clone().monitor_in(clk, d));
    Task::fork(tb.clone().monitor_out(clk, q));

    Trigger::timer(3, "ms").await;
    clock_task.cancel();

    Trigger::timer(100, "ns").await;
    SIM_IF.log(tb.scoreboard.get().result().as_str());

    pass_test("Some message");
    Ok(Val::None)
}

async fn test_default2(_dut: SimObject) -> RstbResult {
    SIM_IF.log("Starting test 2");
    Trigger::timer(100, "ns").await;
    SIM_IF.log("Done test 2");
    // pass_current_test("Explicit pass");
    Ok(Val::None)
}

// Specify tests to be executed
rstb::run_with_vpi!(test_default);
// rstb::run_with_vpi!(test_default, test_default2);

