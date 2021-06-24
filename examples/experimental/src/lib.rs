use rstb::prelude::*;


async fn test_default(dut: SimObject) -> RstbResult {
    let c = dut.c("clk");

    run_all_assertions();

    Trigger::timer(1, "us").await;
    c.set(0);
    // Trigger::timer(1, "us").await;
    // c.set(1);
    // Trigger::timer(10, "ns").await;
    // c.set(0);
    // Trigger::timer(10, "ns").await;
    // c.set(1);
    // Trigger::timer(10, "ns").await;
    // c.set(0);
    Trigger::timer(1, "us").await;
    Ok(Val::None)
}

async fn assertion_setup(dut: SimObject) -> RstbResult {
    sequence!("seq0", async move {
        Trigger::timer(5, "ns").await;
        SIM_IF.log("Seq0 returning");
        Ok(Val::None)
    });
    sequence!("seq1", async move {
        Trigger::timer(6, "ns").await;
        SIM_IF.log("Seq1 returning");
        Ok(Val::None)
    });
    sequence!("seq2", async move {
        Trigger::timer(7, "ns").await;
        SIM_IF.log("Seq2 returning");
        Err(Val::None)
    });

    let c = dut.c("clk");
    assertion!(async move {
        Sequence::use_seq("seq0").await?;
        Sequence::use_seq("seq1").await?;
        Sequence::use_seq("seq2").await?;
        Ok(Val::None)
    }, vec![Trigger::edge(c)]);
    run_all_assertions();

    // let c = dut.c("clk");
    // assertion!(async move {
    //     match c.i32() {
    //         0 => Val::Error,
    //         _ => Val::None
    //     }
    // }, vec![Trigger::edge(c)]);
    // run_all_assertions();



    Ok(Val::None)
}


rstb::run_with_vpi!(assertion_setup, test_default);
