use core::panic;
use futures::{future::{BoxFuture, FutureExt}, task::{waker_ref, ArcWake, Context, Poll}};
use futures_channel::oneshot;
use lazy_mut::lazy_mut;
use std::{future::Future, pin::Pin, sync::{Arc, Mutex}};
use queues::{Queue, IsQueue};

use crate::value::RstbValue;

lazy_mut! {
    static mut READY_QUEUE: Queue<Arc<Task>> = Queue::new();
}

pub fn schedule_task(task: Arc<Task>) {
    unsafe {
        READY_QUEUE.add(task).expect("Error queueing task.");
    }
}

fn next_task() -> Option<Arc<Task>> {
    if let Ok(task) = unsafe { READY_QUEUE.remove()} {
        Some(task)
    } else {
        None
    }
}

#[inline]
pub fn run_once() {
    loop {
        let next = next_task();
        if let Some(task) = next {
            process_task(task);
        } else {
            break;
        }
    }
}

#[inline]
fn process_task(task: Arc<Task>) {
    // vpi::log(&format!("Processing task: {:?}", task.name));
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
            // vpi::log(&format!("Task: {:?} comlete", task.name));
            let mut tx_slot = task.join_tx.lock().unwrap();
            let _ = tx_slot.take().unwrap().send(result);
        }
    } else {
        panic!("Scheduled completed or uninitialized task.");
    }
    // vpi::log(&format!("Processing done: {:?}", task.name));
}

#[derive(PartialEq)]
enum TaskState {
    Done(u32),
    Pending,
    Cancelled
}

pub struct Task {
    future: Mutex<Option<BoxFuture<'static, RstbValue>>>,
    state: Mutex<TaskState>,
    name: Option<String>,
    join_tx: Mutex<Option<oneshot::Sender<RstbValue>>>,
}
impl Task {
    pub fn fork(
        future: impl Future<Output = RstbValue> + Send + 'static
    ) -> JoinHandle {
        Task::spawn_from_future(future, "forked")
    }
    pub fn spawn_from_future(
        future: impl Future<Output = RstbValue> + Send + 'static,
        name: &str,
    ) -> JoinHandle {
        let (task, join_handle) = Task::new_from_future(future, name);
        schedule_task(task);
        join_handle
    }
    fn new_from_future(
        future: impl Future<Output = RstbValue> + Send + 'static,
        name: &str,
    ) -> (Arc<Self>, JoinHandle) {
        Task::new(future.boxed(), name)
    }
    fn new(fut: BoxFuture<'static, RstbValue>, name: &str) -> (Arc<Self>, JoinHandle) {
        let (tx, mut join_handle) = new_join();
        let task = Self {
            future: Mutex::new(Some(fut)),
            state: Mutex::new(TaskState::Pending),
            name: Some(name.to_string()),
            join_tx: Mutex::new(Some(tx)),
        };
        let arc_task = Arc::new(task);

        // mutating join_handle will move it for some reason, so now set_task() returns itself.
        // I don't know why this should be necessary but here we go.
        join_handle = join_handle.set_task(arc_task.clone());
        (arc_task, join_handle)
    }
    pub fn new_uninitialized() -> Self {
        Self {
            future: Mutex::new(None),
            state: Mutex::new(TaskState::Pending),
            name: None,
            join_tx: Mutex::new(None),
        }
    }
    pub fn cancel(&self) {
        // set state to Cancelled, Executor will drop the Task without execution on callback
        *self.state.lock().unwrap() = TaskState::Cancelled;
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        schedule_task(arc_self.clone());
    }
}

fn new_join() -> (oneshot::Sender<RstbValue>, JoinHandle) {
    let (tx, rx) = oneshot::channel::<RstbValue>();
    (tx, JoinHandle { join_rx: rx, awaited_task: None })
}

pub struct JoinHandle {
    awaited_task: Option<Arc<Task>>,
    join_rx: oneshot::Receiver<RstbValue>,
}

impl JoinHandle {
    pub fn set_task(mut self, task: Arc<Task>) -> Self {
        self.awaited_task.replace(task);
        self
    }
    pub fn cancel(mut self) {
        // take awaited_task, cancel it and drop its reference
        let task = self.awaited_task.take().expect("Task already cancelled.");
        task.cancel();
    }
}

impl Future for JoinHandle {
    type Output = RstbValue;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.join_rx.poll_unpin(cx) {
            Poll::Ready(result) => {
                Poll::Ready(result.expect("Error while awaiting forked task."))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
