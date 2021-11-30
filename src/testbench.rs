use crate::prelude::*;
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub struct Scoreboard<T: PartialEq>(AnyObj<ScoreboardInner<T>>);

impl<T: 'static + PartialEq> Scoreboard<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(AnyObj::new_from(ScoreboardInner {
            exp_q: VecDeque::new(),
            recv_q: VecDeque::new(),
            errors: 0,
            expected: 0,
            received: 0,
            matched: 0,
        }))
    }
    pub fn add_exp(&self, data: T) {
        self.0.with_mut(|s| {
            s.exp_q.push_back(data);
            s.expected += 1;
        });
        self.compare();
    }
    pub fn add_recv(&self, data: T) {
        self.0.with_mut(|s| {
            s.recv_q.push_back(data);
            s.received += 1;
        });
        self.compare();
    }
    fn compare(&self) {
        self.0.with_mut(|s| {
            while !s.exp_q.is_empty() && !s.recv_q.is_empty() {
                match s.exp_q.pop_front() == s.recv_q.pop_front() {
                    true => s.matched += 1,
                    false => s.errors += 1,
                }
            }
        });
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

/*
 * MONITOR
 */
#[derive(Clone, Copy)]
pub struct Monitor<T: PartialEq>(AnyObj<MonitorInner<T>>);

impl<T: 'static + PartialEq> Monitor<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(AnyObj::new_from(MonitorInner {
            enable: true,
            exp_not_recv: true,
            scoreboard: None,
        }))
    }
    pub fn set_scoreboard(&self, sb: Scoreboard<T>, exp_not_recv: bool) {
        self.0.with_mut(|m| {
            m.exp_not_recv = exp_not_recv;
            m.scoreboard = Some(sb);
        })
    }
    pub fn to_scoreboard(&self, data: T) {
        self.0.with_mut(|m| {
            if let Some(ref mut s) = m.scoreboard {
                match m.exp_not_recv {
                    true => s.add_exp(data),
                    false => s.add_recv(data),
                }
            } else {
                panic!("Can't forward data to scoreboard. No scoreboard set");
            }
        })
    }
}
#[derive(Clone, Copy)]
struct MonitorInner<T: PartialEq> {
    enable: bool,
    exp_not_recv: bool,
    scoreboard: Option<Scoreboard<T>>,
}

/*
 * CLOCK
 */
#[allow(unreachable_code)]
pub async fn clock(clk: SimObject, period: u32, unit: &str) -> RstbResult {
    let high_t = period / 2;
    let low_t = period - high_t;
    if period % 2 != 0 {
        SIM_IF.log(&format!("Warning: Clock period {period}{unit} not dividable by 2. High time will be {high}{unit}; low time will be {low}{unit}.", period=period, unit=unit, high=high_t, low=low_t));
    }
    loop {
        clk.set(0);
        Trigger::timer(low_t as u64, unit).await;
        clk.set(1);
        Trigger::timer(high_t as u64, unit).await;
    }
    Ok(Val::None)
}
