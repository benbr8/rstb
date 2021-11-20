#![allow(unreachable_code, unused_must_use)]
mod tb;
mod scoreboard;

use rstb::prelude::*;


async fn rd_en(dut: SimObject) -> RstbResult {
    let clk = dut.c("clk");
    let rd_en = dut.c("rd_en");
    loop {
        clk.rising_edge().await;
        Trigger::read_write().await;
        if utils::rand() < 0.5 {
            rd_en.set(1)
        } else {
            rd_en.set(0)
        }
    }
    Ok(Val::None)
}


pub async fn test_fifo(dut: SimObject) -> RstbResult {
    run_all_assertions();
    let tb = tb::FifoTb::new(dut);

    // drop in replacement model emulating the memory within the FIFO
    let mem = tb::MemModel::new(dut.c("mem"), 1<<4);
    Task::fork(mem.exec());

    tb.reset().await;

    Task::fork(tb.clone().read_mon());
    Task::fork(tb.clone().write_mon());
    Task::fork(rd_en(dut));

    let clk = dut.c("clk");
    let din = dut.c("din");
    let wr_en = dut.c("wr_en");

    for j in 0..100_000 {
        clk.rising_edge().await;
        Trigger::read_write().await;

        if utils::rand() < 0.5 {
            // dut.c("din").set_u32(utils::rand_int(1 << 8));
            din.set_u32(j % (1 << 4));
            wr_en.set(1);
        } else {
            wr_en.set(0);
        }
    }
    wr_en.set(0);

    Trigger::timer(1, "us").await;

    tb.clone().scoreboard.get().result()
    // Ok(Val::None)
}


// Specify tests to be executed
rstb::run_with_vpi!(assertion_setup, test_fifo);

async fn assertion_setup(dut: SimObject) -> RstbResult {
    add_assertion! (
        "check_consequence",                                // name
        dut.c("clk").rising_edge(),                         // trigger
        async move {                                        // condition
            check!(dut.c("wr_en").u32() == 1 && dut.c("full").u32() == 0)
        },
        check_dout                                          // check
    );

    Ok(Val::None)
}

async fn check_dout(ctx: AssertionContext) -> RstbResult {
    let clk = ctx.dut().c("clk");
    let rd_en = ctx.dut().c("rd_en");
    let empty = ctx.dut().c("empty");
    let dout = ctx.dut().c("dout");
    let data = ctx.dut().c("din").u32();
    let mut cnt = 0;
    loop {
        clk.rising_edge_ro().await;
        if rd_en.u32() == 1 && empty.u32() == 0 {
            if dout.u32() == data {
                return Ok(Val::None)
            }
            if cnt > 20 {
                return Err(Val::None)
            } else {
                cnt += 1;
            }
        }
    }
}

