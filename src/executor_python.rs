// use pyo3::prelude::*;

use core::panic;
use futures::{future::{BoxFuture, FutureExt}, task::{waker_ref, ArcWake, Context, Poll}};
use futures_channel::oneshot;
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use std::{future::Future, pin::Pin, sync::{mpsc, Arc, Mutex}};
use queues::{Queue, IsQueue};

use crate::vpi;

lazy_static! {
    pub static ref CURRENT_TASK: Mutex<Arc<Task>> = Mutex::new(Arc::new(Task::new_uninitialized()));
}
lazy_static! {
    pub static ref READY_QUEUE: Mutex<Queue<Arc<Task>>> = Mutex::new(Queue::new());
}
pub static READY_QUEUE_SENDER: OnceCell<mpsc::SyncSender<Arc<Task>>> = OnceCell::new();
pub static EXEC: OnceCell<Mutex<Executor>> = OnceCell::new();

pub fn run_once() {
    EXEC.get().unwrap().lock().unwrap().run_once();
}

pub fn schedule_task(task: Arc<Task>) {
    READY_QUEUE.lock()
        .unwrap()
        .add(task).expect("Error queueing task.");
}

fn next_task() -> Option<Arc<Task>> {
    if let Ok(task) = READY_QUEUE.lock().unwrap().remove() {
        Some(task)
    } else {
        None
    }
}

pub fn get_current_task() -> Arc<Task> {
    CURRENT_TASK.lock().unwrap().clone()
}

// pub fn schedule_task(task: Arc<Task>) {
//     READY_QUEUE_SENDER.get().unwrap().send(task).expect("Error while queueing task.")
// }

pub struct Executor {}

impl Executor {

    fn run_once(&self) {
        // while let Ok(task) = self.ready_queue.try_recv() {
        // while let Some(task') = next.take() {
        //     vpi::log(&format!("Processing task {}", task.name.as_ref().unwrap()));
        //     Executor::process_task(task);
        // }
        loop {
            let next = next_task();
            if let Some(task) = next {
                Executor::process_task(task);
            } else {
                break;
            }
        }
    }

    #[inline]
    fn process_task(task: Arc<Task>) {
        // vpi::log(&format!("Processing task: {:?}", task.name));
        // set current task
        if *task.state.lock().unwrap() == TaskState::Cancelled {
            // do not execute if state is cancelled, will be dropped once all references disappear
            return
        }

        *CURRENT_TASK.lock().unwrap() = task.clone();
        let mut inner_slot = task.inner.lock().unwrap();
        if let Some(mut inner) = inner_slot.take() {
            let mut result: Option<u32> = None;
            match inner {
                TaskInner::Future(ref mut fut) => {
                    let waker = waker_ref(&task);
                    let context = &mut Context::from_waker(&*waker);
                    result = match fut.as_mut().poll(context) {
                        Poll::Pending => {
                            *inner_slot = Some(inner);
                            None
                        }
                        Poll::Ready(result) => Some(result),
                    }
                    // if fut.as_mut().poll(context).is_pending() {
                    //     *inner_slot = Some(inner);
                    // }
                }
                // TaskInner::PyCoro(ref mut coro) => {
                //     result = Python::with_gil(|py| -> Option<u32> {
                //         match coro.call_method1(py, "send", (py.None(),)) {
                //             Ok(val) => {
                //                 // println!("coro pending: {:?}", val);
                //                 None
                //             }
                //             Err(e) => {
                //                 vpi::log(&format!("{:?}", e));
                //                 Some(0)
                //             }
                //         }
                //     });
                //     if result.is_none() {
                //         *inner_slot = Some(inner);
                //     }
                // }
            }
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
}

enum TaskInner {
    Future(BoxFuture<'static, u32>),
    // PyCoro(PyObject),
}

#[derive(PartialEq)]
enum TaskState {
    Done(u32),
    Pending,
    Cancelled
}

pub struct Task {
    inner: Mutex<Option<TaskInner>>,
    state: Mutex<TaskState>,
    name: Option<String>,
    join_tx: Mutex<Option<oneshot::Sender<u32>>>,
}
impl Task {
    pub fn fork(
        future: impl Future<Output = u32> + Send + 'static
    ) -> JoinHandle {
        Task::spawn_from_future(future, "forked")
    }
    pub fn spawn_from_future(
        future: impl Future<Output = u32> + Send + 'static,
        name: &str,
    ) -> JoinHandle {
        let (task, join_handle) = Task::new_from_future(future, name);
        schedule_task(task);
        join_handle
    }
    // pub fn spawn_from_coro(coro: PyObject, name: &str) -> JoinHandle {
    //     let (task, join_handle) = Task::new_from_coro(coro, name);
    //     schedule_task(task);
    //     join_handle
    // }
    fn new_from_future(
        future: impl Future<Output = u32> + Send + 'static,
        name: &str,
    ) -> (Arc<Self>, JoinHandle) {
        Task::new(TaskInner::Future(future.boxed()), name)
    }
    // fn new_from_coro(coro: PyObject, name: &str) -> (Arc<Self>, JoinHandle) {
    //     Task::new(TaskInner::PyCoro(coro), name)
    // }
    fn new(fut: TaskInner, name: &str) -> (Arc<Self>, JoinHandle) {
        let (tx, mut join_handle) = new_join();
        let task = Self {
            inner: Mutex::new(Some(fut)),
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
            inner: Mutex::new(None),
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

fn new_join() -> (oneshot::Sender<u32>, JoinHandle) {
    let (tx, rx) = oneshot::channel::<u32>();
    (tx, JoinHandle { join_rx: rx, awaited_task: None })
}

pub struct JoinHandle {
    awaited_task: Option<Arc<Task>>,
    join_rx: oneshot::Receiver<u32>,
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
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.join_rx.poll_unpin(cx) {
            Poll::Ready(result) => {
                Poll::Ready(result.expect("Error while awaiting forked task."))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
