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
    sim_if::{SimCallback, SIM_IF},
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
    // high exec prio currently only implemented for ReadOnly
    high_exec_prio: bool,
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
                        let cb = SimCallback::ReadWrite;
                        let cb_hdl = SIM_IF.register_callback(cb).unwrap();
                        READ_WRITE.handle.replace(cb_hdl);
                    }
                },
                TrigKind::ReadOnly => unsafe {
                    match self.high_exec_prio {
                        false => READ_ONLY.callbacks.push_back(shared),
                        true => READ_ONLY.callbacks.push_front(shared),
                    }
                    if READ_ONLY.handle.is_none() {
                        let cb = SimCallback::ReadOnly;
                        let cb_hdl = SIM_IF.register_callback(cb).unwrap();
                        READ_ONLY.handle.replace(cb_hdl);
                    }
                },
                TrigKind::Timer(t) => {
                    // Add current time to key since since simulator will send back absolute time, not delta
                    let abs_time = t + SIM_IF.get_sim_time_steps();
                    if let Some(callbacks) = unsafe { TIMER_MAP.get_mut(abs_time) } {
                        callbacks.callbacks.push_back(shared);
                    } else {
                        let handle = SIM_IF.register_callback(SimCallback::Time(t)).unwrap();
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
                    shared.edge_kind = edge_kind;
                    if let Some(callbacks) = unsafe { EDGE_MAP.get_mut(sig_hdl as u64) } {
                        // vpi::log("Callback already exists. Appending.");
                        callbacks.callbacks.push_back(shared);
                    } else {
                        // vpi::log("Registering callback.");
                        let handle = SIM_IF
                            .register_callback(SimCallback::Edge(sig_hdl))
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
pub fn react(cb: SimCallback, edge: Option<EdgeKind>) {
    let mut vec_wake: Option<VecDeque<TrigShared>> = None;

    match cb {
        SimCallback::ReadWrite => {
            unsafe {
                READ_WRITE.handle = None; // remove handle, since CB is now done
                if !READ_WRITE.callbacks.is_empty() {
                    vec_wake = Some(std::mem::take(&mut READ_WRITE.callbacks));
                } else {
                    panic!("Did not expect ReadWrite callback");
                }
            }
        }
        SimCallback::ReadOnly => {
            unsafe {
                READ_ONLY.handle = None; // remove handle, since CB is now done
                if !READ_ONLY.callbacks.is_empty() {
                    vec_wake = Some(std::mem::take(&mut READ_ONLY.callbacks));
                } else {
                    panic!("Did not expect ReadOnly callback");
                }
            }
        }
        SimCallback::Time(t) => {
            // SIM_IF.log(&format!("Reacting to Timer CB: t={}", t));
            if let Some(callbacks) = unsafe { TIMER_MAP.remove(t) } {
                vec_wake = Some(callbacks.callbacks);
            } else {
                panic!("Did not expect Timer callback: t={}", t);
            }
        }
        SimCallback::Edge(sig_hdl) => {
            // vpi::log("Reacting to Edge callback.");
            let callbacks = unsafe { EDGE_MAP.remove(sig_hdl as u64) };
            if let Some(mut callbacks) = callbacks {
                let edge = edge.unwrap();
                match edge {
                    EdgeKind::Any => {
                        vec_wake = Some(std::mem::take(&mut callbacks.callbacks));
                    }
                    _ => {
                        let mut vec_resched: VecDeque<TrigShared> = VecDeque::new();
                        let mut vec_wake_tmp: VecDeque<TrigShared> = VecDeque::new();
                        for trig in callbacks.callbacks.drain(..) {
                            if trig.edge_kind == EdgeKind::Any || trig.edge_kind == edge {
                                // vpi::log("Trigger will be woken.");
                                // Trigger will be woken
                                vec_wake_tmp.push_back(trig);
                            } else {
                                // vpi::log("Trigger will be rescheduled.");
                                // Trigger will be rescheduled
                                vec_resched.push_back(trig);
                            }
                        }
                        if vec_resched.is_empty() {
                            // if no callbacks are remaining, cancel
                            // vpi::log("no callbacks are remaining -> cancel");
                            SIM_IF.cancel_callback(callbacks.handle.unwrap()).unwrap();
                        } else {
                            // put rescheduled callbacks back into EDGE_MAP
                            // vpi::log("put rescheduled callbacks back into EDGE_MAP");
                            callbacks.callbacks = vec_resched;
                            unsafe { EDGE_MAP.insert(sig_hdl as u64, callbacks) };
                        }
                        if !vec_wake_tmp.is_empty() {
                            // vpi::log(&format!("Waking callbacks: n={}", vec_wake_tmp.len()));
                            vec_wake = Some(vec_wake_tmp);
                        }
                    }
                }
            } else {
                panic!("Did not expect Edge callback: sig_hdl={}", sig_hdl);
            }
        }
    }

    if let Some(vec_wake) = vec_wake {
        for shared in vec_wake {
            shared.waker.wake();
        }
        // execute woken tasks
        executor::run_once();
    }
}
