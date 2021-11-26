#![allow(unreachable_code, unused_must_use, dead_code)]
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
    // run all registered assertions
    run_all_assertions();
    let tb = tb::FifoTb::new(dut);

    // Use a Model of the memory inside the FIFO instead of the HDL implementation
    // Just because we can :)
    let mem = tb::MemModel::new(dut.c("mem"), 1<<4);
    Task::fork(mem.exec());

    tb.reset().await;

    // Fork concurrent processes, such as interface monitors and stimulus generators
    Task::fork(tb.clone().read_mon());
    Task::fork(tb.clone().write_mon());
    Task::fork(rd_en(dut));

    // Using these prevents HashMap lookups in the loop
    let clk = dut.c("clk");
    let din = dut.c("din");
    let wr_en = dut.c("wr_en");

    for j in 0..100_000 {
        clk.rising_edge().await;
        Trigger::read_write().await;

        if utils::rand() < 0.5 {
            din.set_u32(j % (1 << 4));
            wr_en.set(1);
        } else {
            wr_en.set(0);
        }
    }
    wr_en.set(0);

    Trigger::timer(1, "us").await;

    tb.clone().scoreboard.get().result()
}

// Specify tests to be executed
rstb::run_with_vpi!(test_fifo);



async fn assertion_setup(dut: SimObject) -> RstbResult {

    // This assertion will check that every word that is input to the fifo
    // will come out at the output after at most 16 reads.
    add_assertion! (
        "input_to_output",                                  // name
        dut.c("clk").rising_edge(),                         // trigger
        async move {                                        // condition
            check!(dut.c("wr_en").u32() == 1 && dut.c("full").u32() == 0)
        },
        move |_| { async move {                             // checking function
            let data = dut.c("din").u32();
            let mut rd_cnt = 0;
            loop {
                dut.c("clk").rising_edge_ro().await;
                if dut.c("rd_en").u32() == 1 && dut.c("empty").u32() == 0 {
                    if dut.c("dout").u32() == data {
                        return Ok(Val::None)
                    }
                    if rd_cnt > 10 {
                        return Err(Val::None)
                    } else {
                        rd_cnt += 1;
                    }
                }
            }
        }}
    );

    Ok(Val::None)
}

