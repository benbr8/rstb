use rstb::prelude::*;
use std::{borrow::Borrow, collections::VecDeque};

#[derive(Clone)]
pub struct Scoreboard<T: PartialEq>(RstbObj<ScoreboardInner<T>>);

impl<T: PartialEq> Scoreboard<T> {
    pub fn new() -> Self {
        Self(RstbObj::new(ScoreboardInner {
            exp_q: VecDeque::new(),
            recv_q: VecDeque::new(),
            errors: 0,
            expected: 0,
            received: 0,
            matched: 0,
        }))
    }
    pub fn add_exp(&self, data: T) {
        {
            let mut inner = self.0.get_mut();
            inner.exp_q.push_back(data);
            inner.expected += 1;
        }
        self.compare();
    }
    pub fn add_recv(&self, data: T) {
        {
            let mut inner = self.0.get_mut();
            inner.recv_q.push_back(data);
            inner.received += 1;
        }
        self.compare();
    }
    fn compare(&self) {
        let mut inner = self.0.get_mut();
        while !inner.exp_q.is_empty() && !inner.recv_q.is_empty() {
            match inner.exp_q.pop_front() == inner.recv_q.pop_front() {
                true => inner.matched += 1,
                false => inner.errors += 1,
            }
        }
    }
    pub fn result(&self) -> RstbResult {
        match self.passed() {
            true => Ok(Val::String(self.result_str())),
            false => Err(Val::String(self.result_str())),
            // true => Ok(Val::None),
            // false => Err(Val::None),
        }
    }
    pub fn passed(&self) -> bool {
        let inner = self.0.get();
        inner.expected > 0
            && inner.received == inner.expected
            && inner.matched == inner.received
            && inner.errors == 0
            && inner.exp_q.is_empty()
            && inner.recv_q.is_empty()
    }
    pub fn result_str(&self) -> String {
        let inner = self.0.get();
        format!(
            "expected={}, received={}, matched={}, errors={}, expQ: {}, recvQ: {}",
            inner.expected,
            inner.received,
            inner.matched,
            inner.errors,
            inner.exp_q.len(),
            inner.recv_q.len()
        )
    }
}


struct ScoreboardInner<T>
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