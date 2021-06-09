use pyo3::iter::IterNextOutput;
use pyo3::prelude::*;
use pyo3::{PyAsyncProtocol, PyIterProtocol};

use std::sync::{Arc, Mutex};

use crate::executor;
use crate::signal;
use crate::trigger;
use crate::vpi;

#[pyclass]
pub struct PyInterface {}

#[pymethods]
impl PyInterface {
    #[staticmethod]
    fn spawn_coro(py: Python, coro: PyObject, name: &str) -> PyResult<PyObject> {
        let join_handle = executor::Task::spawn_from_coro(coro, name);
        let py_join_handle = PyJoinHandle::new_from_join_handle(join_handle);
        let py_obj: PyObject = PyCell::new(py, py_join_handle)?.into();
        Ok(py_obj)
    }
    #[staticmethod]
    fn timer(py: Python, t: u64) -> PyResult<PyObject> {
        let fut = trigger::Trigger::timer(t);
        let join_handle =
            executor::Task::spawn_from_future(fut, &format!("timer_{}+{}", vpi::get_sim_time(), t));
        let py_join_handle = PyJoinHandle::new_from_join_handle(join_handle);
        let py_obj: PyObject = PyCell::new(py, py_join_handle)?.into();
        Ok(py_obj)
    }

    #[staticmethod]
    fn edge(py: Python, signal: &PySignal, kind: &str) -> PyResult<PyObject> {
        // vpi::log(&format!("edge(): signal={:?}, kind={}", signal, kind));
        let edge_kind = match kind {
            "rising" => trigger::EdgeKind::Rising,
            "falling" => trigger::EdgeKind::Falling,
            _ => trigger::EdgeKind::Any,
        };
        let signal = signal.signal.clone();
        // vpi::log(&format!("edge(): edge_type={:?}", edge_type));
        let name = &format!("edge_{}_{}", kind, signal.name());
        let fut = trigger::Trigger::edge(signal, edge_kind);
        let join_handle = executor::Task::spawn_from_future(fut, name);
        let py_join_handle = PyJoinHandle::new_from_join_handle(join_handle);
        let py_obj: PyObject = PyCell::new(py, py_join_handle)?.into();
        Ok(py_obj)

    }

    #[allow(clippy::unnecessary_wraps)]
    #[staticmethod]
    fn vpi_log(_py: Python, msg: &str) -> PyResult<()> {
        vpi::log(msg);
        Ok(())
    }

    #[staticmethod]
    fn get_sim_root(py: Python) -> PyResult<PyObject> {
        // vpi::log("Accessing get_sim_root() from PY");
        let signal = signal::SimObject::get_root()?;
        let sim_obj: PyObject = PyCell::new(py, PySignal { signal })?.into();
        Ok(sim_obj)
    }
    #[staticmethod]
    fn read_only(py: Python) -> PyResult<PyObject> {
        // vpi::log("Accessing read_only() from PY");
        let fut = trigger::Trigger::read_only();
        let join_handle = executor::Task::spawn_from_future(fut, "read_only");
        let py_join_handle = PyJoinHandle::new_from_join_handle(join_handle);
        let py_obj: PyObject = PyCell::new(py, py_join_handle)?.into();
        Ok(py_obj)
    }
    #[staticmethod]
    fn read_write(py: Python) -> PyResult<PyObject> {
        // vpi::log("Accessing read_only() from PY");
        let fut = trigger::Trigger::read_write();
        let join_handle = executor::Task::spawn_from_future(fut, "read_write");
        let py_join_handle = PyJoinHandle::new_from_join_handle(join_handle);
        let py_obj: PyObject = PyCell::new(py, py_join_handle)?.into();
        Ok(py_obj)
    }
    #[allow(clippy::unnecessary_wraps)]
    #[staticmethod]
    fn get_sim_time(_py: Python) -> PyResult<u64> {
        // vpi::log("Accessing get_sim_time() from PY");
        let t = vpi::get_sim_time();
        Ok(t)
    }
    #[allow(clippy::unnecessary_wraps)]
    #[staticmethod]
    fn get_sim_precision(_py: Python) -> PyResult<i32> {
        // vpi::log("Accessing get_sim_precision() from PY");
        Ok(vpi::get_sim_precision())
    }
}

#[pyclass]
#[derive(Clone, Debug)]
struct PySignal {
    signal: signal::SimObject,
}

#[allow(clippy::unnecessary_wraps)]
#[pymethods]
impl PySignal {
    fn kind(&self, _py: Python) -> PyResult<String> {
        // vpi::log("Accessing kind() from PY");
        let _a = self.signal.kind();
        Ok("todo".to_string())
    }
    fn name(&self, _py: Python) -> PyResult<String> {
        // vpi::log("Accessing name() from PY");
        Ok(self.signal.name())
    }
    fn width(&self, _py: Python) -> PyResult<u32> {
        // vpi::log("Accessing width() from PY");
        Ok(self.signal.width())
    }
    fn set_value(&self, _py: Python, value: i32) -> PyResult<()> {
        self.signal.set_value(value)?;
        Ok(())
    }
    fn get_value(&self, _py: Python) -> PyResult<i32> {
        Ok(self.signal.get_value()?)
    }
    fn is_modifiable(&self, _py: Python) -> PyResult<bool> {
        // vpi::log("Accessing is_modifiable() from PY");
        Ok(self.signal.is_modifiable())
    }
    fn discover_nets(&self, _py: Python) -> PyResult<()> {
        // vpi::log("Accessing discover_nets() from PY");
        self.signal.discover_nets();
        Ok(())
    }
    fn get_child(&self, py: Python, name: &str) -> PyResult<PyObject> {
        // vpi::log("Accessing get_child() from PY");
        let child = self.signal.get_child(name)?;
        let cell = PyCell::new(py, PySignal { signal: child })?;
        let obj = cell.into_py(py);
        Ok(obj)
    }
}


#[pyclass]
struct PyJoinHandle {
    join_task_hdl: Option<executor::JoinHandle>,
    result: Arc<Mutex<Option<u32>>>,
    handle: Option<executor::JoinHandle>,
}

impl PyJoinHandle {
    fn new_from_join_handle(join_handle: executor::JoinHandle) -> Self {
        PyJoinHandle {
            join_task_hdl: None,
            result: Arc::new(Mutex::new(None)),
            handle: Some(join_handle),
        }
    }
}

#[pymethods]
impl PyJoinHandle {
    fn cancel(&mut self) {
        let hdl = self.handle.take().expect("Task already cancelled.");
        hdl.cancel();
        if let Some(join_task_hdl) = self.join_task_hdl.take() {
            join_task_hdl.cancel();
        }
    }
    fn kill(&mut self) {
        self.cancel();
    }
}

#[pyproto]
impl PyAsyncProtocol for PyJoinHandle {
    fn __await__(mut slf: PyRefMut<Self>) -> PyRefMut<Self> {
        // create a new support Task which awaits join_rx.recv() and then queues awaiting task
        // vpi::log("calling await on PyJoinHandle.");
        let join_handle: executor::JoinHandle = slf
            .handle
            .take()
            .expect("PyJoinHandle not initialized or already awaited.");
        let result_handle: Arc<Mutex<Option<u32>>> = slf.result.clone();
        let awaiting_task = executor::get_current_task();
        let join_task_handle = executor::Task::spawn_from_future(
            async move {
                // await/set result, then queue up awaiting task
                // vpi::log("awaiting on join_rx");
                let r = join_handle.await;
                *result_handle.lock().unwrap() = Some(r);
                // vpi::log("awaited on join_rx. Queueing awaiting task.");
                executor::schedule_task(awaiting_task);
                0
            },
            "join_task",
        );
        slf.join_task_hdl.replace(join_task_handle);
        slf
    }
}

#[pyproto]
impl PyIterProtocol for PyJoinHandle {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
    fn __next__(slf: PyRef<Self>) -> IterNextOutput<(), u32> {
        let result: Option<u32> = slf.result.lock().unwrap().take();
        // vpi::log(&format!("JoinHandle.next(): result={:?}", result));

        match result {
            None => IterNextOutput::Yield(()),
            Some(r) => IterNextOutput::Return(r),
        }
    }
}


fn py_init_tb() {
    let coro: PyObject = Python::with_gil(|py| -> PyResult<PyObject> {
        let rstb_py = py.import("rstb.rstb")?;
        let if_obj: PyObject = PyCell::new(py, python::PyInterface {}).unwrap().into();
        let obj: PyObject = rstb_py.call1("initialize_testbench", (if_obj,))?.into();
        Ok(obj)
    })
    .unwrap();
    executor::Task::spawn_from_coro(coro, "MAIN");
}

fn py_info(py: Python) -> PyResult<()> {
    let sys = py.import("sys")?;
    let os = py.import("os")?;
    let cwd: String = os.call0("getcwd")?.extract()?;
    let version: String = sys.get("version")?.extract()?;
    let exec: String = sys.get("executable")?.extract()?;
    eprintln!("Using Python {} at {}. CWD: {}", version, exec, cwd);
    Ok(())
}

pub fn py_init() {
    extern "C" {
        fn dlopen(filename: *const libc::c_char, flags: libc::c_int) -> *mut libc::c_void;
        // fn dlclose(handle: *mut libc::c_void) -> libc::c_int;
    }
    const RTLD_GLOBAL: libc::c_int = 0x00100;
    const RTLD_LAZY: libc::c_int = 0x00001;

    const LIBPYTHON: &str = "/usr/lib/x86_64-linux-gnu/libpython3.9.so\0";

    let handle: *mut libc::c_void;
    unsafe {
        handle = dlopen(LIBPYTHON.as_ptr() as *const i8, RTLD_GLOBAL | RTLD_LAZY);
        assert!(!handle.is_null());
    }

    eprintln!("start py_init");
    Python::with_gil(|py| {
        py_info(py).unwrap();
    });
    // if !handle.is_null() {
    //     unsafe {
    //         dlclose(handle);
    //     }
    // }
    eprintln!("end py_init");
}


impl std::convert::From<RstbErr> for pyo3::PyErr {
    fn from(_err: RstbErr) -> pyo3::PyErr {
        pyo3::exceptions::PyException::new_err("RstbErr".to_string())
    }
}
