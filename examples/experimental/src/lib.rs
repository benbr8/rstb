use rstb::prelude::*;


async fn test_default(dut: SimObject) -> RstbValue {
    let c = dut.c("clk");

    // Trigger::timer(1, "ns").await;
    assertion!(async move {
        match c.i32() {
            0 => RstbValue::Error,
            _ => RstbValue::None
        }
    }, vec![Trigger::edge(c)]);
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
    RstbValue::None
}


rstb::run_with_vpi!(test_default);
