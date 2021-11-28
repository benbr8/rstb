#![allow(unreachable_code, unused_must_use)]

use rstb::prelude::*;
use testbench::{Monitor, Scoreboard};

#[derive(Clone)]
pub struct MemModel {
    dut: SimObject,
    depth: usize,
    mem: Vec<u32>,
}

impl MemModel {
    pub fn new(dut: SimObject, depth: usize) -> Self {
        Self {
            dut,
            depth,
            mem: vec![0; depth],
        }
    }

    pub async fn exec(mut self) -> RstbResult {
        let clk = self.dut.c("clk");
        let raddr = self.dut.c("raddr");
        let waddr = self.dut.c("waddr");
        let dout = self.dut.c("dout");
        let din = self.dut.c("din");
        let we = self.dut.c("we");
        loop {
            clk.rising_edge().await;
            Trigger::timer_steps(1).await;

            let addr = raddr.u32();
            let data = self.mem[addr as usize];
            dout.set_u32(data);
            if we.u32() == 1 {
                let addr = waddr.u32();
                let data = din.u32();
                self.mem[addr as usize] = data;
            }
        }

        Ok(Val::None)
    }
}

#[derive(Clone, Copy)]
pub struct AxisMonitor {
    pub mon: Monitor<u32>,
    clk: SimObject,
    tvalid: SimObject,
    tready: SimObject,
    tdata: SimObject,
}

impl AxisMonitor {
    pub fn new(clk: SimObject, tvalid: SimObject, tready: SimObject, tdata: SimObject) -> Self {
        Self {
            mon: Monitor::new(),
            clk,
            tvalid,
            tready,
            tdata,
        }
    }

    pub async fn run(self) -> RstbResult {
        loop {
            self.clk.rising_edge_ro().await;
            if self.tvalid.u32() == 1 && self.tready.u32() == 1 {
                self.mon.to_scoreboard(self.tdata.u32());
            }
        }
        Ok(Val::None)
    }
}

#[derive(Clone, Copy)]
pub struct FifoTb {
    pub scoreboard: Scoreboard<u32>,
    mon_in: AxisMonitor,
    mon_out: AxisMonitor,
    dut: SimObject,
    clk: SimObject,
}

impl FifoTb {
    pub fn new(dut: SimObject) -> Self {
        let tb = Self {
            scoreboard: Scoreboard::new(),
            dut,
            mon_in: AxisMonitor::new(dut.c("clk"), dut.c("s_tvalid"), dut.c("s_tready"), dut.c("s_tdata")),
            mon_out: AxisMonitor::new(dut.c("clk"), dut.c("m_tvalid"), dut.c("m_tready"), dut.c("m_tdata")),
            clk: dut.c("clk"),
        };
        tb.mon_in.mon.set_scoreboard(tb.scoreboard, true);
        tb.mon_out.mon.set_scoreboard(tb.scoreboard, false);
        Task::spawn(tb.clock_stim(10));
        Task::spawn(tb.mon_in.run());
        Task::spawn(tb.mon_out.run());
        tb
    }

    pub async fn reset(&self) -> RstbResult {
        self.clk.rising_edge_rw().await;
        self.dut.c("s_tvalid").set(0);
        self.dut.c("m_tready").set(0);
        self.dut.c("rst").set(1);
        utils::clock_cycles(self.clk, 10).await;
        self.dut.c("rst").set(0);
        utils::clock_cycles(self.clk, 2).await;
        Ok(Val::None)
    }

    async fn clock_stim(self, clk_period_ns: u64) -> RstbResult {
        let half_period_ps = clk_period_ns * 1000 / 2;
        loop {
            self.clk.set(0);
            Trigger::timer(half_period_ps, "ps").await;
            self.clk.set(1);
            Trigger::timer(half_period_ps, "ps").await;
        }
        Ok(Val::None)
    }
}
