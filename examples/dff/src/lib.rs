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
        // SIM_IF.log("awaiting rising edge clk");
        Trigger::rising_edge(clk).await;
        // SIM_IF.log("DONE awaiting rising edge clk");
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
    // Fail test after 1 ms, all triggers and tasks are shut down immediately.
    // Task::fork(fail_after_1ms());

    // use force() or force_bin() to force a signal value
    dut.c("q").force_bin("0");

    Trigger::timer(1, "ms").await;
    // release forced signal
    q.release();
    Trigger::timer(2, "ms").await;
    clock_task.cancel();

    // using combine!()
    SIM_IF.log("forking a, b, c");
    let a = Task::fork(async {Trigger::timer(10, "ns").await; Ok(Val::Int(1))});
    let b = Task::fork(async {Trigger::timer(20, "ns").await; Ok(Val::Int(2))});
    let c = Task::fork(async {Trigger::timer(15, "ns").await; Ok(Val::Int(3))});
    let d = combine!(a, b, c).await;
    SIM_IF.log(&format!("combine!(a, b, c): {:?}", d));

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


rstb::run_with_vpi!(test_default, test_default2);

