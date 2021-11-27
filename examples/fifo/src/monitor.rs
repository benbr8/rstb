use rstb::prelude::*;
use crate::scoreboard::*;

#[derive(Clone, Copy)]
pub struct Monitor<T: PartialEq>(AnyObj<MonitorInner<T>>);

impl<T: 'static + PartialEq> Monitor<T> {
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
            }
        })
    }
}


#[derive(Clone, Copy)]
struct MonitorInner<T: PartialEq>
{
    enable: bool,
    exp_not_recv: bool,
    scoreboard: Option<Scoreboard<T>>
}