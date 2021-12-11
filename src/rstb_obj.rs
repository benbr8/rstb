
use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
    sync::{Arc, Mutex, MutexGuard},
};
use lazy_mut::lazy_mut;
use intmap::IntMap;
use std::any::Any;
use std::marker::PhantomData;


lazy_mut! {
    static mut OBJ_MAP: IntMap<Rc<RefCell<dyn Any>>> = IntMap::new();
}

static mut OBJ_CNT: u64 = 0;


#[derive(Default)]
pub struct AnyObj<T>(u64, PhantomData<T>);

impl<T> Clone for AnyObj<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}
impl<T> Copy for AnyObj<T> {}

impl<T: 'static> AnyObj<T> {
    pub fn new() -> Self {
        Self(0, PhantomData)
    }
    pub fn delete(self) {
        unsafe { OBJ_MAP.remove(self.0).expect("Could not find specified object.") };
    }
    pub fn new_from(data: T) -> Self {
        let obj_id = unsafe {
            OBJ_CNT += 1;
            OBJ_CNT
        };
        let obj = Self(obj_id, PhantomData);
        obj.set(data);
        obj
    }
    pub fn with<F: FnOnce(&T)>(&self, f: F)
    {
        let a = unsafe { OBJ_MAP.get(self.0).unwrap().borrow() };
        let b = a.downcast_ref::<T>().unwrap();
        f(b);
    }
    pub fn with_mut<F: FnOnce(&mut T)>(&self, f: F)
    {
        let mut a = unsafe { OBJ_MAP.get(self.0)
            .expect("AnyObj not yet initialized.")
            .borrow_mut() };
        let b = a.downcast_mut::<T>().unwrap();
        f(b);
    }

    pub fn get(&self) -> Ref<T> {
        let a = unsafe { OBJ_MAP.get(self.0).unwrap()};
        let b = a.borrow();
        Ref::map(b, |a| {
            a.downcast_ref::<T>().unwrap()
        })
    }
    pub fn get_mut(&self) -> RefMut<T> {
        let a = unsafe { OBJ_MAP.get(self.0).unwrap()};
        let b = a.borrow_mut();
        RefMut::map(b, |a| {
            a.downcast_mut::<T>().unwrap()
        })
    }

    fn set(&self, data: T) {
        let r = Rc::new(RefCell::new(data));
        unsafe { OBJ_MAP.insert(self.0, r) };
    }
}

pub(crate) fn clear_objects() {
    unsafe { OBJ_MAP.clear() };
}



// RstbObj shall allow the user to mutably share test objects (such as a Scoreboard, etc.)
// between Tasks. Since the simulation is single threaded, we can use Rc, RefCell, which
// are not Send + Sync without worrying.
pub struct RstbObj<T>(Rc<RefCell<T>>);

impl<T> RstbObj<T> {
    pub fn new(val: T) -> RstbObj<T> {
        RstbObj(Rc::new(RefCell::new(val)))
    }
    pub fn get(&self) -> Ref<T> {
        (*self.0).borrow()
    }
    pub fn get_mut(&self) -> RefMut<T> {
        (*self.0).borrow_mut()
    }
    pub fn replace(&self, val: T) {
        self.0.replace(val);
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
#[derive(Debug)]
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
    pub fn with_mut<F: FnOnce(MutexGuard<T>) -> R, R>(&self, f: F) -> R {
        let guard = self.0.lock().unwrap();
        f(guard)
    }
}

impl<T> Clone for RstbObjSafe<T> {
    fn clone(&self) -> Self {
        RstbObjSafe(self.0.clone())
    }
}

// pub(crate) fn with_mutex<T, F: FnOnce(MutexGuard<T>)>(mutex: &Mutex<T>, f: F) {
//     let guard = mutex.lock().unwrap();
//     f(guard);
// }