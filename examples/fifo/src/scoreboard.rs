use rstb::prelude::*;
use std::collections::VecDeque;

pub struct Scoreboard<T>
where
    T: PartialEq,
{
    exp_q: VecDeque<T>,
    recv_q: VecDeque<T>,
    errors: u32,
    expected: u32,
    received: u32,
    matched: u32,
}
impl<T> Scoreboard<T>
where
    T: PartialEq,
{
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
        self.compare();
    }
    pub fn add_recv(&mut self, data: T) {
        {
            self.recv_q.push_back(data);
            self.received += 1;
        }
        self.compare();
    }

    fn compare(&mut self) {
        while !self.exp_q.is_empty() && !self.recv_q.is_empty() {
            match self.exp_q.pop_front() == self.recv_q.pop_front() {
                true => self.matched += 1,
                false => self.errors += 1,
            }
        }
    }

    pub fn result(&self) -> RstbResult {
        match self.passed() {
            true => Ok(Val::String(self.result_str())),
            false => Err(Val::String(self.result_str())),
        }
    }

    pub fn result_str(&self) -> String {
        format!(
            "expected={}, received={}, matched={}, errors={}, expQ: {}, recvQ: {}",
            self.expected,
            self.received,
            self.matched,
            self.errors,
            self.exp_q.len(),
            self.recv_q.len()
        )
    }

    pub fn passed(&self) -> bool {
        self.expected > 0
            && self.received == self.expected
            && self.matched == self.received
            && self.errors == 0
            && self.exp_q.is_empty()
            && self.recv_q.is_empty()
    }

    pub fn pass_or_fail(&self) {
        if self.passed() {
            pass_test(&self.result_str());
        } else {
            fail_test(&self.result_str())
        }
    }
}
