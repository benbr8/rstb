#![allow(clippy::result_unit_err)]

use lazy_mut::lazy_mut;
use intmap::IntMap;
use crate::{RstbResult, seamap::SeaMap};
use crate::sim_if::SIM_IF;
use crate::SimpleResult;
use crate::trigger::Trigger;
use crate::value::Val;

lazy_mut! {
    static mut SIG_MAP_NAME: SeaMap<String, usize> = SeaMap::new();
}
lazy_mut! {
    static mut SIG_MAP: IntMap<SimObject> = IntMap::new();
}

#[derive(Clone, Copy, Debug)]
pub struct SimObject {
    pub(crate) handle: usize,
    pub(crate) kind: ObjectKind
}


#[derive(Copy, Clone, Debug)]
pub enum ObjectKind {
    Int(i32),
    Real,
    Array(i32),
    Hier,
    Other,
}


impl SimObject {
    pub fn handle(&self) -> usize {
        self.handle
    }

    pub fn kind(&self) -> ObjectKind {
        self.kind
    }

    pub fn name(&self) -> String {
        SIM_IF
            .get_full_name(self)
            .expect("Couldn't get name of ObjectInner")
    }

    pub fn size(&self) -> i32 {
        match self.kind {
            ObjectKind::Int(size) | ObjectKind::Array(size) => size,
            _ => panic!("SimObject does not have a size.")
        }
    }

    pub fn is_modifiable(&self) -> bool {
        !matches!(self.kind, ObjectKind::Other)
    }

    pub fn has_value(&self) -> bool {
        !matches!(self.kind, ObjectKind::Other)
    }

    #[allow(clippy::needless_question_mark)] // it actueally is necessary
    pub fn get_child(&self, name: &str) -> SimpleResult<Self> {
        let mut child_name = self.name();
        child_name.push('.');
        child_name.push_str(name);
        Ok(SimObject::from_name(child_name.as_str())?)
    }

    #[allow(clippy::clone_on_copy)]
    pub fn from_handle(handle: usize) -> SimpleResult<Self> {
        if let Some(signal) = unsafe { SIG_MAP.get(handle as u64) } {
            Ok(signal.clone())
        } else {
            Err(())
        }
    }

    pub fn from_name(full_name: &str) -> SimpleResult<Self> {
        let handle = unsafe { SIG_MAP_NAME.get(full_name) }
            .map(|h| h.to_owned());
        match handle {
            Some(h) => SimObject::from_handle(h),
            _ => Ok(SimObject::new_from_name(full_name)?),
        }
    }

    fn new_from_name(full_name: &str) -> SimpleResult<Self> {
        let handle = SIM_IF.get_handle_by_name(full_name)?;
        Ok(SimObject::new_from_handle(handle))
    }

    pub(crate) fn new_from_handle(handle: usize) -> Self {
        let signal = SimObject {
            handle,
            kind: SIM_IF.get_kind(handle),
        };
        unsafe {
            SIG_MAP.insert(handle as u64, signal);
            SIG_MAP_NAME.insert(signal.name(), handle);
        };
        signal
    }

    pub fn get_root() -> SimpleResult<Self> {
        SIM_IF.get_root_object()
    }

    // pub fn discover_nets(&self) -> Vec<SimObject> {
    //     let obj_list = vpi::discover_nets(self.handle);
    //     let mut signal_list = Vec::with_capacity(obj_list.len());
    //     for obj in obj_list {
    //         signal_list.push(SimObject::new_from_handle(obj));
    //     }
    //     signal_list
    // }

    pub fn i32(&self) -> i32 {
        SIM_IF.get_value_i32(self).unwrap()
    }

    pub fn u32(&self) -> u32 {
        SIM_IF.get_value_u32(self).unwrap()
    }

    pub fn bin(&self) -> String {
        SIM_IF.get_value_bin(self).unwrap()
    }

    pub fn c(&self, name: &str) -> Self {
        self.get_child(name)
            .unwrap_or_else(|_| panic!("Could not get object with name {}.{}", self.name(), name))
    }

    pub fn release(&self) {
        SIM_IF.release(self).unwrap();
    }

    pub fn set(&self, val: i32) {
        SIM_IF.set_value_i32(&self, val, false).unwrap();
    }

    pub fn force(&self, val: i32) {
        SIM_IF.set_value_i32(&self, val, true).unwrap();
    }

    pub fn set_u32(&self, val: u32) {
        SIM_IF.set_value_u32(&self, val, false).unwrap();
    }

    pub fn force_u32(&self, val: u32) {
        SIM_IF.set_value_u32(&self, val, true).unwrap();
    }

    pub fn set_bin(&self, val: &str) {
        self._set_bin(val, false)
    }

    pub fn force_bin(&self, val: &str) {
        self._set_bin(val, true)
    }

    #[inline]
    fn _set_bin(&self, val: &str, force: bool) {
        // remove '_' and 0b
        let size = match self.kind {
            ObjectKind::Int(size) => size,
            _ => panic!("Can't set {} using set_bin()", self.name())
        };
        let stripped = val.replace("0b", "");
        let stripped = stripped.replace("_", "");
        if stripped.len() == size as usize {
            let is_valid = stripped.chars().all(valid_char);
            if is_valid {
                SIM_IF.set_value_bin(self, stripped, force).unwrap();
            } else {
                panic!("Can't set {} to {}. Invalid characters.", self.name(), val);
            }
        } else {
            panic!("Can't set {} to {}. Length mismatch.", self.name(), val);
        }
    }

    // convenience functions to get edge triggers for this signal
    pub fn rising_edge(self) -> Trigger {
        Trigger::rising_edge(self)
    }
    pub async fn rising_edge_ro(self) -> RstbResult {
        self.rising_edge().await;
        Trigger::read_only().await;
        Ok(Val::None)
    }
    pub async fn rising_edge_rw(self) -> RstbResult {
        self.rising_edge().await;
        Trigger::read_write().await;
        Ok(Val::None)
    }
    pub fn falling_edge(self) -> Trigger {
        Trigger::falling_edge(self)
    }
    pub fn edge(self) -> Trigger {
        Trigger::edge(self)
    }
}

fn valid_char(c: char) -> bool {
    let l = c.to_ascii_lowercase();
    l == '0' || l == '1' || l == 'z' || l == 'x'
}
