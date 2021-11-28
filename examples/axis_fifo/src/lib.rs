#![allow(unreachable_code, unused_must_use, dead_code)]
mod tb;

use rstb::prelude::*;


async fn rd_en(dut: SimObject) -> RstbResult {
    let clk = dut.c("clk");
    let rd_en = dut.c("m_tready");
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
    tb.reset().await;

    // Use a Model of the memory inside the FIFO instead of the HDL implementation
    // Just because we can :)
    let mem = tb::MemModel::new(dut.c("fifo").c("mem"), 1<<4);
    Task::spawn(mem.exec());
    Task::spawn(rd_en(dut));


    // Using these prevents HashMap lookups in the loop
    let clk = dut.c("clk");
    let tdata = dut.c("s_tdata");
    let tvalid = dut.c("s_tvalid");

    for j in 0..100_000 {
        clk.rising_edge_rw().await;

        if utils::rand() < 0.5 {
            tdata.set_u32(j % (1 << 4));
            tvalid.set(1);
        } else {
            tvalid.set(0);
        }
    }
    tvalid.set(0);

    Trigger::timer(1, "us").await;

    tb.scoreboard.result()
}

// Specify tests to be executed
rstb::run_with_vpi!(/*assertion_setup,*/ test_fifo);



async fn assertion_setup(dut: SimObject) -> RstbResult {
    let clk = dut.c("clk");
    let s_tvalid = dut.c("s_tvalid");
    let s_tready = dut.c("s_tready");
    let s_tdata = dut.c("s_tdata");
    let m_tvalid = dut.c("m_tvalid");
    let m_tready = dut.c("m_tready");
    let m_tdata = dut.c("m_tdata");

    // This assertion will check that every word that is input to the fifo
    // will come out at the output after at most 16 reads.
    add_assertion! (
        "input_to_output",                                  // name
        clk.rising_edge(),                                  // trigger
        async move {                                        // condition
            check!(s_tvalid.u32() == 1 && s_tready.u32() == 1)
        },
        move |_| { async move {                             // checking function
            let data = s_tdata.u32();
            let mut rd_cnt = 0;
            loop {
                clk.rising_edge_ro().await;
                if m_tvalid.u32() == 1 && m_tready.u32() == 1 {
                    if m_tdata.u32() == data {
                        return Ok(Val::None)
                    }
                    if rd_cnt > 16 {
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

