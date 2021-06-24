use rstb::prelude::*;

async fn test_default(dut: SimObject) -> RstbResult {
    let c = dut.c("clk");

    run_all_assertions();

    Trigger::timer(1, "us").await;
    c.set(0);
    Trigger::timer(1, "us").await;
    c.set(1);
    Trigger::timer(10, "ns").await;
    c.set(0);
    Trigger::timer(10, "ns").await;
    c.set(1);
    Trigger::timer(10, "ns").await;
    c.set(0);
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
        Ok(Val::None)
    });

    let c = dut.c("clk");
    assertion_with_condition!(
        "some_name",
        async move {
            Sequence::get("seq0").await?;
            Sequence::get("seq1").await?;
            Sequence::get("seq2").await?;
            Ok(Val::None)
        },
        async move {
            SIM_IF.log(&format!("clk = {}", dut.c("clk").u32()));
            check!(dut.c("clk").u32() == 1)?;
            Ok(Val::None)
        },
        vec![Trigger::edge(c)]
    );

    Ok(Val::None)
}

rstb::run_with_vpi!(assertion_setup, test_default);
