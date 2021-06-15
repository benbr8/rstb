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
    async fn monitor_in(self, clk: SimObject, signal: SimObject) -> RstbValue {
        loop {
            Trigger::rising_edge(clk).await;
            // Trigger::read_only().await;
            self.scoreboard.get_mut().add_exp(signal.i32());
        }
        RstbValue::None
    }
    async fn monitor_out(self, clk: SimObject, signal: SimObject) -> RstbValue {
        loop {
            Trigger::rising_edge(clk).await;
            // Trigger::read_only().await;
            self.scoreboard.get_mut().add_recv(signal.i32());
        }
        RstbValue::None
    }
}

async fn d_stim(clk: SimObject, d: SimObject) -> RstbValue {
    d.set(0);
    loop {
        // SIM_IF.log("awaiting rising edge clk");
        Trigger::rising_edge(clk).await;
        // SIM_IF.log("DONE awaiting rising edge clk");
        d.set((d.i32() + 1) % 2);
    }
    RstbValue::None
}

async fn reset(dut: SimObject) -> RstbValue {
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

    RstbValue::None
}

async fn fail_after_1ms() -> RstbValue {
    Trigger::timer(1, "ms").await;
    fail_current_test("panic!");

    RstbValue::None
}

pub async fn test_default(dut: SimObject) -> RstbValue {
    let tb = DffTb::new();
    let clk = dut.c("clk");
    let d = dut.c("d");

    // start clock
    let clock_task = Task::fork(clock(clk, 8, "ns"));
    // reset
    reset(dut).await;

    Task::fork(d_stim(clk, d));
    Task::fork(tb.clone().monitor_in(clk, d));
    Task::fork(tb.clone().monitor_out(clk, d));
    // Task::fork(fail_after_1ms());


    Trigger::timer(3, "ms").await;  // 3 ms
    clock_task.cancel();

    // using combine!()
    SIM_IF.log("forking a, b");
    let a = Task::fork(async {Trigger::timer(10, "ns"); RstbValue::Int(1)});
    let b = Task::fork(async {Trigger::timer(20, "ns"); RstbValue::Int(2)});
    let c = combine!(a, b).await;
    SIM_IF.log(&format!("combine(a, b): {:?}", c));

    Trigger::timer(100, "ns").await;  // 100 ns
    SIM_IF.log(tb.scoreboard.get().result().as_str());

    pass_current_test("Some message");
    RstbValue::None
}

async fn test_default2(_dut: SimObject) -> RstbValue {
    SIM_IF.log("Starting test 2");
    Trigger::timer(100, "ns").await;  // 100 ns
    SIM_IF.log("Done test 2");
    // pass_current_test("Explicit pass");
    RstbValue::None
}


rstb::run_with_vpi!(test_default, test_default2);

