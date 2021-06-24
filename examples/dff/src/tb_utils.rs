
use rstb::prelude::*;
use std::collections::VecDeque;



pub async fn clock(clk: SimObject, period: u32, unit: &str) -> RstbResult {
    let half_period = period / 2;
    loop {
        clk.set(0);
        Trigger::timer(half_period as u64, unit).await;
        // SIM_IF.log(&format!("clk={}", clk.binstr()));
        clk.set(1);
        Trigger::timer(half_period as u64, unit).await;
        // SIM_IF.log(&format!("clk={}", clk.binstr()));
    }
    Ok(Val::None)
}

pub struct Scoreboard<T>
where T: PartialEq {
    exp_q: VecDeque<T>,
    recv_q: VecDeque<T>,
    errors: u32,
    expected: u32,
    received: u32,
    matched: u32,
}
impl<T> Scoreboard<T>
where T: PartialEq {
    pub fn new() -> Self {
        Scoreboard {
            exp_q: VecDeque::new(),
            recv_q: VecDeque::new(),
            errors: 0,
            expected: 0,
            received: 0,
            matched: 0,
        }
    }

    pub fn add_exp(&mut self, data: T) {
        self.exp_q.push_back(data);
        self.expected += 1;
    }
    pub fn add_recv(&mut self, data: T) {
        {
            self.recv_q.push_back(data);
            self.received += 1;
        }
        self.compare();
    }

    fn compare(&mut self) {
        for _ in 0.. std::cmp::min(self.exp_q.len(), self.recv_q.len()) {
            match self.exp_q.pop_front() == self.recv_q.pop_front() {
                true => self.matched += 1,
                false => self.errors += 1,
            }
        }
    }

    pub fn result(&self) -> String {
        format!("expected={}, received={}, matched={}, errors={}, expQ: {}, recvQ: {}",
        self.expected, self.received, self.matched, self.errors, self.exp_q.len(), self.recv_q.len())
    }
}
