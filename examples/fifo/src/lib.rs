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
    let tb = tb::FifoTb::new(dut);

    // drop in replacement model emulating the memory within the FIFO
    let mem = tb::MemModel::new(dut.c("mem"), 1<<4);
    Task::fork(mem.exec());

    let clk = dut.c("clk");
    let din = dut.c("din");
    let wr_en = dut.c("wr_en");

    tb.reset().await;

    Task::fork(tb.clone().read_mon());
    Task::fork(tb.clone().write_mon());

    Task::fork(rd_en(dut));

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

    tb.scoreboard.get().pass_or_fail();
    Ok(Val::None)
}


// Specify tests to be executed
rstb::run_with_vpi!(test_fifo);

