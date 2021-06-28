use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
    sync::{Arc, Mutex, MutexGuard},
};

// RstbObj shall allow the user to mutably share test objects (such as a Scoreboard, etc.)
// between Tasks. Since the simulation is single threaded, we can use Rc, RefCell, which
// are not Send + Sync without worrying.
pub struct RstbObj<T>(Rc<RefCell<T>>);

impl<T> RstbObj<T> {
    pub fn new(data: T) -> RstbObj<T> {
        RstbObj(Rc::new(RefCell::new(data)))
    }
    pub fn get(&self) -> Ref<T> {
        (*self.0).borrow()
    }
    pub fn get_mut(&self) -> RefMut<T> {
        (*self.0).borrow_mut()
    }
}

impl<T> Clone for RstbObj<T> {
    fn clone(&self) -> Self {
        RstbObj(self.0.clone())
    }
}

// Rc is neither Send nor Sync but in this context its safe. We save some overhead.
unsafe impl<T> Send for RstbObj<T> {}
unsafe impl<T> Sync for RstbObj<T> {}


// safe RustObj implementation, for if there appear issues with the unsafe one
pub struct RstbObjSafe<T>(Arc<Mutex<T>>);

impl<T> RstbObjSafe<T> {
    pub fn new(data: T) -> RstbObjSafe<T> {
        RstbObjSafe(Arc::new(Mutex::new(data)))
    }
    pub fn get(&self) -> MutexGuard<T> {
        (*self.0).try_lock().unwrap()
    }
    pub fn get_mut(&self) -> MutexGuard<T> {
        (*self.0).try_lock().unwrap()
    }
}

impl<T> Clone for RstbObjSafe<T> {
    fn clone(&self) -> Self {
        RstbObjSafe(self.0.clone())
    }
}
