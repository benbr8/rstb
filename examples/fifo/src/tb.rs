#![allow(unreachable_code, unused_must_use)]

use crate::scoreboard::*;
use rstb::prelude::*;

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

#[derive(Clone)]
pub struct FifoTb {
    pub scoreboard: RstbObj<Scoreboard<u32>>,
    dut: SimObject,
    clk: SimObject,
}

impl FifoTb {
    pub fn new(dut: SimObject) -> Self {
        let tb = Self {
            scoreboard: RstbObj::new(Scoreboard::new()),
            dut,
            clk: dut.c("clk"),
        };
        Task::fork(tb.clone().clock_stim(10));
        tb
    }

    pub async fn reset(&self) -> RstbResult {
        let rst = self.dut.c("rst");

        self.clk.rising_edge().await;
        self.dut.c("wr_en").set(0);
        self.dut.c("rd_en").set(0);
        rst.set(1);
        utils::clock_cycles(self.clk, 10).await;
        rst.set(0);
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

    pub async fn write_mon(self) -> RstbResult {
        let wr_en = self.dut.c("wr_en");
        let full = self.dut.c("full");
        let din = self.dut.c("din");
        loop {
            self.clk.rising_edge().await;
            Trigger::read_only().await;
            if wr_en.u32() == 1 && full.u32() == 0 {
                // SIM_IF.log(&format!("Adding expected to scoreboard: {}", dut.c("din").u32()));
                self.scoreboard.get_mut().add_exp(din.u32());
            }
        }
        Ok(Val::None)
    }

    pub async fn read_mon(self) -> RstbResult {
        let rd_en = self.dut.c("rd_en");
        let empty = self.dut.c("empty");
        let dout = self.dut.c("dout");
        loop {
            self.clk.rising_edge().await;
            Trigger::read_only().await;
            if rd_en.u32() == 1 && empty.u32() == 0 {
                // SIM_IF.log(&format!("Adding received to scoreboard: {}", dut.c("dout").u32()));
                self.scoreboard.get_mut().add_recv(dout.u32());
            }
        }
        Ok(Val::None)
    }
}
