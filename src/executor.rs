use core::panic;
use futures::{future::{BoxFuture, FutureExt}, task::{waker_ref, ArcWake, Context, Poll}};
use futures_channel::oneshot;
use lazy_mut::lazy_mut;
use std::{future::Future, pin::Pin, sync::{Arc, Mutex}};
use queues::{Queue, IsQueue};

use crate::value::Val;
use crate::RstbResult;
#[allow(unused_imports)]
use crate::sim_if::SIM_IF;


lazy_mut! {
    static mut READY_QUEUE: Queue<Arc<Task>> = Queue::new();
}


pub(crate) fn schedule_task(task: Arc<Task>) {
    unsafe {
        READY_QUEUE.add(task).expect("Error queueing task.");
    }
}
pub(crate) fn clear_ready_queue() {
    unsafe { *READY_QUEUE = Queue::new() };
}


#[inline]
fn next_task() -> Option<Arc<Task>> {
    if let Ok(task) = unsafe { READY_QUEUE.remove() } {
        Some(task)
    } else {
        None
    }
}

#[inline]
pub(crate) fn run_once() {
    while let Some(task) = next_task() {
        process_task(task);
    }
}


#[inline]
fn process_task(task: Arc<Task>) {
    if *task.state.lock().unwrap() == TaskState::Cancelled {
        // do not execute if state is cancelled, will be dropped once all references disappear
        return
    }

    let mut fut_slot = task.future.lock().unwrap();
    if let Some(mut fut) = fut_slot.take() {
        let waker = waker_ref(&task);
        let context = &mut Context::from_waker(&*waker);
        let result = match fut.as_mut().poll(context) {
            Poll::Pending => {
                *fut_slot = Some(fut);
                None
            }
            Poll::Ready(result) => Some(result),
        };
        if let Some(result) = result {
            let mut tx_slot = task.join_tx.lock().unwrap();
            if let Some(tx) = tx_slot.take() {
                let _ = tx.send(result);
            }
        }
    } else {
        panic!("Scheduled completed or uninitialized task.");
    }
}

#[derive(PartialEq)]
enum TaskState {
    Pending,
    Cancelled
}

pub struct Task {
    future: Mutex<Option<BoxFuture<'static, RstbResult>>>,
    state: Mutex<TaskState>,
    join_tx: Mutex<Option<oneshot::Sender<RstbResult>>>,
}
impl Task {
    pub fn spawn(
        future: impl Future<Output = RstbResult> + Send + 'static
    ) -> JoinHandle {
        Task::spawn_from_future(future)
    }
    pub fn spawn_from_future(
        future: impl Future<Output = RstbResult> + Send + 'static
    ) -> JoinHandle {
        let (task, join_handle) = Task::new_from_future(future);
        schedule_task(task);
        join_handle
    }
    fn new_from_future(
        future: impl Future<Output = RstbResult> + Send + 'static,
    ) -> (Arc<Self>, JoinHandle) {
        Task::new(future.boxed())
    }
    fn new(fut: BoxFuture<'static, RstbResult>) -> (Arc<Self>, JoinHandle) {
        let (tx, mut join_handle) = new_join();
        let task = Self {
            future: Mutex::new(Some(fut)),
            state: Mutex::new(TaskState::Pending),
            join_tx: Mutex::new(Some(tx)),
        };
        let arc_task = Arc::new(task);

        join_handle.set_task(&arc_task);
        (arc_task, join_handle)
    }
    pub fn new_uninitialized() -> Self {
        Self {
            future: Mutex::new(None),
            state: Mutex::new(TaskState::Pending),
            join_tx: Mutex::new(None),
        }
    }
    pub fn cancel(&self) {
        // since we can't know the tasks current trigger, we set its state
        // to cancelled so it will be discarded once the trigger fires
        *self.state.lock().unwrap() = TaskState::Cancelled;
        let mut tx_slot = self.join_tx.lock().unwrap();
        let tx = tx_slot.take().unwrap();
        let _ = tx.send(Ok(Val::None));
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        schedule_task(arc_self.clone());
    }
}

fn new_join() -> (oneshot::Sender<RstbResult>, JoinHandle) {
    let (tx, rx) = oneshot::channel::<RstbResult>();
    (tx, JoinHandle { join_rx: rx, awaited_task: None })
}

pub struct JoinHandle {
    awaited_task: Option<Arc<Task>>,
    join_rx: oneshot::Receiver<RstbResult>,
}

impl JoinHandle {
    pub(crate) fn set_task(&mut self, task: &Arc<Task>) {
        self.awaited_task.replace(task.clone());
    }
    pub(crate) fn get_task(&self) -> Option<Arc<Task>> {
        self.awaited_task
            .as_ref()
            .cloned()
    }
    pub fn cancel(mut self) {
        // take awaited_task, cancel it and drop its reference
        let task = self.awaited_task.take().expect("Task already cancelled.");
        task.cancel();
    }
    pub fn and_then(self, fut: impl Future<Output = RstbResult> + Send + 'static) -> JoinHandle {
        Task::spawn(async move {
            self.join_rx.await.unwrap()?;
            fut.await
        })
    }
}

impl Future for JoinHandle {
    type Output = RstbResult;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.join_rx.poll_unpin(cx) {
            Poll::Ready(result) => {
                Poll::Ready(result.expect("Error while awaiting forked task."))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[macro_export]
macro_rules! combine {
    ($( $i:ident ),+) => {
        Task::spawn(async move {
            let mut vec: Vec<Val> = Vec::new();
            $(vec.push($i.await.unwrap());)+
            Ok(Val::Vec(vec))
        });
    }
}
