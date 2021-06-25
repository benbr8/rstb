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
    add_assertion! (
        "some_name",
        Trigger::rising_edge(dut.c("clk")),
        async move {
            Ok(Val::None)
        },
        async move {
            Ok(Val::None)
        });
    Ok(Val::None)
}

rstb::run_with_vpi!(assertion_setup, test_default);
