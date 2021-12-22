use intmap::IntMap;
use lazy_mut::lazy_mut;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use crate::executor;
use crate::{
    RstbResult,
    signal::SimObject,
    sim_if::SIM_IF,
    value::Val,
};

// IntMap specializes on u64 keys and is faster than even SeaMap, since it doesn't actualy need to calculate a hash
lazy_mut! {
    // key is signal handle as u64
    static mut EDGE_MAP: IntMap<CallbackHandles> = IntMap::new();
}
lazy_mut! {
    // key is absolute callback time
    static mut TIMER_MAP: IntMap<CallbackHandles> = IntMap::new();
}
lazy_mut! {
    static mut READ_ONLY: CallbackHandles = CallbackHandles { handle: None, callbacks: VecDeque::new() };
}
lazy_mut! {
    static mut READ_WRITE: CallbackHandles = CallbackHandles { handle: None, callbacks: VecDeque::new() };
}

struct CallbackHandles {
    handle: Option<usize>,
    callbacks: VecDeque<TrigShared>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum EdgeKind {
    Any,
    Rising,
    Falling,
}

pub(crate) fn cancel_all_triggers() {
    unsafe {
        // RO
        READ_ONLY.callbacks = VecDeque::new();
        if let Some(handle) = READ_ONLY.handle.take() {
            SIM_IF.cancel_callback(handle).unwrap();
        }
        // RW
        READ_WRITE.callbacks = VecDeque::new();
        if let Some(handle) = READ_WRITE.handle.take() {
            SIM_IF.cancel_callback(handle).unwrap();
        }
        // Timers
        for (_, cb) in TIMER_MAP.drain() {
            // TaskCallbacks will be dropped with this context
            SIM_IF.cancel_callback(cb.handle.unwrap()).unwrap();
        }
        // Edges
        for (_, cb) in EDGE_MAP.drain() {
            // TaskCallbacks will be dropped with this context
            SIM_IF.cancel_callback(cb.handle.unwrap()).unwrap();
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrigShared {
    waker: Waker,
    // If trigger is an edge, the react method needs to know if it is a rising or falling edge
    // so an existing callback does not have to be rescheduled.
    edge_kind: EdgeKind,
}

#[derive(Clone)]
pub enum TrigKind {
    Edge(usize, EdgeKind),
    Timer(u64),
    ReadWrite,
    ReadOnly,
}

#[derive(Clone)]
pub struct Trigger {
    kind: TrigKind,
    awaited: bool,
    high_exec_prio: bool,    // high exec prio currently only implemented for ReadOnly
}

impl Trigger {
    pub fn timer(time: u64, unit: &str) -> Self {
        // SIM_IF.log(&format!("TIMER: {}{}", time, unit));
        Trigger {
            kind: TrigKind::Timer(SIM_IF.get_sim_steps(time as f64, unit)),
            awaited: false,
            high_exec_prio: false,
        }
    }
    pub fn timer_steps(steps: u64) -> Self {
        Trigger {
            kind: TrigKind::Timer(steps),
            awaited: false,
            high_exec_prio: false,
        }
    }
    pub async fn timer_ro(time: u64, unit: &str) -> RstbResult {
        Trigger::timer(time, unit).await;
        Trigger::read_only().await;
        Ok(Val::None)
    }
    pub async fn timer_rw(time: u64, unit: &str) -> RstbResult {
        Trigger::timer(time, unit).await;
        Trigger::read_write().await;
        Ok(Val::None)
    }
    pub fn edge(signal: SimObject) -> Self {
        Trigger {
            kind: TrigKind::Edge(signal.handle(), EdgeKind::Any),
            awaited: false,
            high_exec_prio: false,
        }
    }
    pub fn rising_edge(signal: SimObject) -> Self {
        Trigger {
            kind: TrigKind::Edge(signal.handle(), EdgeKind::Rising),
            awaited: false,
            high_exec_prio: false,
        }
    }
    pub fn falling_edge(signal: SimObject) -> Self {
        Trigger {
            kind: TrigKind::Edge(signal.handle(), EdgeKind::Falling),
            awaited: false,
            high_exec_prio: false,
        }
    }
    pub fn read_write() -> Self {
        Trigger {
            kind: TrigKind::ReadWrite,
            awaited: false,
            high_exec_prio: false,
        }
    }
    pub fn read_only() -> Self {
        Trigger {
            kind: TrigKind::ReadOnly,
            awaited: false,
            high_exec_prio: false,
        }
    }
    pub(crate) fn read_only_prio() -> Self {
        Trigger {
            kind: TrigKind::ReadOnly,
            awaited: false,
            high_exec_prio: true,
        }
    }
}

impl Future for Trigger {
    type Output = Val;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // vpi::log("Polling trigger");
        // Trigger must only be awaited once, so the second time it is polled it must be because
        // the waker signaled its completion.
        if self.awaited {
            // vpi::log("Trigger done!");
            Poll::Ready(Val::None)
        } else {
            self.awaited = true;
            // vpi::log("Initilaize new Trigger!");
            // initialize new Trigger
            let mut shared = TrigShared {
                waker: cx.waker().clone(),
                edge_kind: EdgeKind::Any,
            };

            match self.kind {
                TrigKind::ReadWrite => unsafe {
                    READ_WRITE.callbacks.push_back(shared);
                    if READ_WRITE.handle.is_none() {
                        let cb_hdl = SIM_IF.register_callback_rw().unwrap();
                        READ_WRITE.handle.replace(cb_hdl);
                    }
                },
                TrigKind::ReadOnly => unsafe {
                    match self.high_exec_prio {
                        false => READ_ONLY.callbacks.push_back(shared),
                        true => READ_ONLY.callbacks.push_front(shared),
                    }
                    if READ_ONLY.handle.is_none() {
                        let cb_hdl = SIM_IF.register_callback_ro().unwrap();
                        READ_ONLY.handle.replace(cb_hdl);
                    }
                },
                TrigKind::Timer(t) => {
                    assert!(t != 0, "Time must be a positive number.");
                    // Add current time to key since since simulator will send back absolute time, not delta
                    let abs_time = t + SIM_IF.get_sim_time_steps();
                    if let Some(callbacks) = unsafe { TIMER_MAP.get_mut(abs_time) } {
                        callbacks.callbacks.push_back(shared);
                    } else {
                        // SIM_IF.log("Registering time");
                        let handle = SIM_IF.register_callback_time(t).unwrap();
                        let mut vec = VecDeque::new();
                        vec.push_back(shared);
                        let callback = CallbackHandles {
                            handle: Some(handle),
                            callbacks: vec,
                        };
                        unsafe { TIMER_MAP.insert(abs_time, callback) };
                    }
                }
                TrigKind::Edge(sig_hdl, edge_kind) => {
                    // SIM_IF.log("Edge trigger");
                    shared.edge_kind = edge_kind;
                    if let Some(callbacks) = unsafe { EDGE_MAP.get_mut(sig_hdl as u64) } {
                        // vpi::log("Callback already exists. Appending.");
                        callbacks.callbacks.push_back(shared);
                    } else {
                        // vpi::log("Registering callback.");
                        // SIM_IF.log("Registering edge");
                        let handle = SIM_IF
                            .register_callback_edge(sig_hdl)
                            .unwrap();
                        let mut vec = VecDeque::new();
                        vec.push_back(shared);
                        let callback = CallbackHandles {
                            handle: Some(handle),
                            callbacks: vec,
                        };
                        unsafe { EDGE_MAP.insert(sig_hdl as u64, callback) };
                    }
                }
            }
            // vpi::log("Done: Initilaize Trigger!");
            Poll::Pending
        }
    }
}


#[inline]
fn wake(vec_wake: VecDeque<TrigShared>) {
    for shared in vec_wake {
        shared.waker.wake();
    }
    // execute woken tasks
    executor::run_once();
}

#[inline]
pub(crate) fn react_rw() {
    let vec_wake;
    unsafe {
        READ_WRITE.handle = None; // remove handle, since CB is now done
        if !READ_WRITE.callbacks.is_empty() {
            vec_wake = std::mem::take(&mut READ_WRITE.callbacks);
        } else {
            panic!("Did not expect ReadOnly callback");
        }
    }
    wake(vec_wake);
}

#[inline]
pub(crate) fn react_ro() {
    let vec_wake;
    unsafe {
        READ_ONLY.handle = None; // remove handle, since CB is now done
        if !READ_ONLY.callbacks.is_empty() {
            vec_wake = std::mem::take(&mut READ_ONLY.callbacks);
        } else {
            panic!("Did not expect ReadOnly callback");
        }
    }
    wake(vec_wake);
}

#[inline]
pub(crate) fn react_time(t: u64) {
    let cbh = unsafe { TIMER_MAP.remove(t).expect("Did not expect Timer callback at given time") };
    wake(cbh.callbacks);
}

#[inline]
pub(crate) fn react_edge(sig_hdl: usize, edge: EdgeKind) {
    let mut cbh = unsafe { EDGE_MAP.remove(sig_hdl as u64) }.unwrap();
    let mut vec_wake;
    match edge {
        EdgeKind::Any => {
            vec_wake = std::mem::take(&mut cbh.callbacks);
            // SIM_IF.log("Any: cancelling callback");
            SIM_IF.cancel_callback(cbh.handle.unwrap()).unwrap();
        }
        _ => {
            vec_wake = VecDeque::with_capacity(cbh.callbacks.len());
            let mut vec_resched: VecDeque<TrigShared> = VecDeque::with_capacity(cbh.callbacks.len());
            // dbg!(&cbh.callbacks);
            for trig in cbh.callbacks.drain(..) {
                if trig.edge_kind == EdgeKind::Any || trig.edge_kind == edge {
                    vec_wake.push_back(trig);
                } else {
                    vec_resched.push_back(trig);
                }
            }
            if vec_resched.is_empty() {
                SIM_IF.cancel_callback(cbh.handle.unwrap()).unwrap();
                // SIM_IF.log("Cancelling edge callback");
            } else {
                // std::mem::replace(&mut cbh.callbacks, vec_resched);
                // dbg!(&vec_resched);
                cbh.callbacks = vec_resched;
                unsafe { EDGE_MAP.insert(sig_hdl as u64, cbh) };
            }
        }
    }

    wake(vec_wake);
}
