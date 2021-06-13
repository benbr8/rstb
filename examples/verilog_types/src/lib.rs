#![allow(unreachable_code)]
use rstb::prelude::*;
use rstb::vpi;

async fn test_default(dut: SimObject) -> RstbValue {
    SIM_IF.log("Starting test");
    Trigger::timer(10, "ns").await;

    SIM_IF.log("TOP MODULE:");
    print_obj(dut.handle());

    SIM_IF.log("TOP PORTS via iterator:");
    let list = vpi::discover_nets(dut.handle());
    for handle in list {
        print_obj(handle);
    }
    SIM_IF.log("TOP PORTS output:");
    print_obj(dut.c("q").handle());

    SIM_IF.log("DUT ELEMENTS:");
    print_obj(dut.c("reg8").handle());
    print_obj(dut.c("reg_").handle());
    print_obj(dut.c("integer_").handle());
    print_obj(dut.c("int_").handle());
    print_obj(dut.c("longint_").handle());
    print_obj(dut.c("bit8").handle());
    print_obj(dut.c("bit_").handle());
    print_obj(dut.c("shortreal_").handle());
    print_obj(dut.c("real_").handle());
    print_obj(dut.c("bits65").handle());

    SIM_IF.log(&format!("int(bits65) = {}", dut.c("bits65").i32()));
    dut.c("clk").set(1);
    Trigger::timer(1, "ns").await;
    // upper bits will be discarded, returns 1
    SIM_IF.log(&format!("int(bits65) = {}", dut.c("bits65").i32()));

    // questa will return 128 for both, icarus takes signed declaration into accounts
    SIM_IF.log(&format!("int(reg8) = {}", dut.c("reg8").i32()));
    SIM_IF.log(&format!("int(reg8_signed) = {}", dut.c("reg8_signed").i32()));
    SIM_IF.log(&format!("signed(reg8) = {}", dut.c("reg8").is_signed()));
    SIM_IF.log(&format!("signed(reg8_signed) = {}", dut.c("reg8_signed").is_signed()));
    // both return 2**-31
    SIM_IF.log(&format!("int(reg32) = {}", dut.c("reg32").i32()));
    SIM_IF.log(&format!("int(reg32_signed) = {}", dut.c("reg32_signed").i32()));
    SIM_IF.log(&format!("binstr(reg32) = {}", SIM_IF.get_value_bin(dut.c("reg32").handle()).unwrap()));
    SIM_IF.log(&format!("binstr(reg32_signed) = {}", SIM_IF.get_value_bin(dut.c("reg32_signed").handle()).unwrap()));

    let reg1_signed = dut.c("reg1_signed");
    SIM_IF.log(&format!("int(reg1) = {}", dut.c("reg1").i32()));
    SIM_IF.log(&format!("int(reg1_signed) = {}", reg1_signed.i32()));

    // can rstb handle "rising_edge" on signed regs of 1 bit size?
    // --> it can since react() checks for values != 0
    Task::fork(async move {
        Trigger::rising_edge(reg1_signed).await;
        SIM_IF.log("Rising_edge(reg1_signed) awaited");
        SIM_IF.log(&format!("int(reg1_signed) = {}", reg1_signed.i32()));
        RstbValue::None
    });
    reg1_signed.set(0);
    Trigger::timer(1, "ns").await;
    reg1_signed.set(1);
    Trigger::timer(1, "ns").await;


    SIM_IF.log("Done test");
    RstbValue::None
}

fn kind_name(kind: i32) -> String {
    match kind {
        25 => "vpiIntegerVar".to_string(),
        32 => "vpiModule".to_string(),
        36 => "vpiNet".to_string(),
        47 => "vpiRealVar".to_string(),
        48 => "vpiReg".to_string(),
        610 => "vpiLongIntVar".to_string(),
        612 => "vpiIntVar".to_string(),
        613 => "vpiShortRealVar".to_string(),
        620 => "vpiBitVar".to_string(),
        _ => "unspecified".to_string(),
    }
}

fn print_obj(handle: usize) {
    let name = SIM_IF.get_full_name(handle).unwrap();
    let kind = vpi::get_kind_raw(handle);
    let size = vpi::get_size_raw(handle);
    SIM_IF.log(&format!(
        "SimObject {}: name={}, kind={}({}), size={}: ",
        handle,
        name,
        kind,
        kind_name(kind),
        size
    ));
}

rstb::run_with_vpi!(test_default);