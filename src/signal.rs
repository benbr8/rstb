use lazy_mut::lazy_mut;

use crate::seamap::SeaMap;
use crate::{RstbErr, RstbResult};
use crate::{sim_if, sim_if::SIM_IF};

lazy_mut! {
    static mut SIG_MAP_NAME: SeaMap<String, usize> = SeaMap::new();
}
lazy_mut! {
    static mut SIG_MAP: SeaMap<usize, SimObject> = SeaMap::new();
}


#[derive(Clone, Copy, Debug)]
pub struct SimObject {
    handle: usize,
    kind: sim_if::ObjectKind,
}

impl SimObject {
    pub fn set_value(&self, value: i32) -> RstbResult<()> {
        if self.is_modifiable() {
            SIM_IF.set_value_int(self.handle, value)?;
            Ok(())
        } else {
            Err(RstbErr)
        }
    }

    pub fn get_value(&self) -> RstbResult<i32> {
        if self.has_value() {
            Ok(SIM_IF.get_value_int(self.handle())?)
        } else {
            Err(RstbErr)
        }
    }

    pub fn handle(&self) -> usize {
        self.handle
    }

    pub fn kind(&self) -> sim_if::ObjectKind {
        self.kind
    }

    pub fn name(&self) -> String {
        SIM_IF.get_full_name(self.handle).expect("Couldn't get name of ObjectInner")
    }

    pub fn width(&self) -> u32 {
        match self.kind() {
            sim_if::ObjectKind::BitVector(width) | sim_if::ObjectKind::Array(width) => width,
            _ => 1,
        }
    }

    pub fn is_modifiable(&self) -> bool {
        !matches!(self.kind(), sim_if::ObjectKind::Unknown)
    }

    pub fn has_value(&self) -> bool {
        !matches!(self.kind(), sim_if::ObjectKind::Unknown)
    }

    #[allow(clippy::needless_question_mark)] // it actueally is necessary
    pub fn get_child(&self, name: &str) -> RstbResult<SimObject> {
        let mut child_name = self.name();
        child_name.push('.');
        child_name.push_str(name);
        Ok(SimObject::from_name(child_name.as_str())?)
    }

    #[allow(clippy::clone_on_copy)]
    pub fn from_handle(handle: usize) -> RstbResult<Self> {
        if let Some(signal) = unsafe{SIG_MAP.get(&handle)} {
            Ok(signal.clone())
        } else {
            Err(RstbErr)
        }
    }

    pub fn from_name(full_name: &str) -> RstbResult<Self> {
        let handle = match unsafe{SIG_MAP_NAME.get(full_name)} {
            Some(h) => Some(h.to_owned()),
            _ => None,
        };
        match handle {
            Some(h) => SimObject::from_handle(h),
            _ => Ok(SimObject::new_from_name(full_name)?),
        }
    }

    fn new_from_name(full_name: &str) -> RstbResult<Self> {
        let handle = SIM_IF.get_handle_by_name(full_name)?;
        Ok(SimObject::new_from_handle(handle))
    }

    fn new_from_handle(handle: usize) -> Self {
        let signal = SimObject {
            handle,
            kind: SIM_IF.get_kind(handle),
        };
        unsafe{
            SIG_MAP.insert(handle, signal);
            SIG_MAP_NAME.insert(signal.name(), handle);
        };
        signal
    }

    pub fn get_root() -> RstbResult<Self> {
        Ok(SimObject::new_from_handle(SIM_IF.get_root_handle()?))
    }

    // pub fn discover_nets(&self) -> Vec<SimObject> {
    //     let obj_list = vpi::discover_nets(self.handle);
    //     let mut signal_list = Vec::with_capacity(obj_list.len());
    //     for obj in obj_list {
    //         signal_list.push(SimObject::new_from_handle(obj));
    //     }
    //     signal_list
    // }

    // short interface
    pub fn i32(&self) -> i32 {
        self.get_value().unwrap()
    }
    pub fn c(&self, name: &str) -> Self {
        self.get_child(name).unwrap()
    }
    pub fn set(&self, val: i32) {
        self.set_value(val).unwrap();
    }
}
