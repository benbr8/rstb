use lazy_mut::lazy_mut;
use intmap::IntMap;
use crate::seamap::SeaMap;
use crate::sim_if::{ObjectKind, SIM_IF};
use crate::SimpleResult;
use crate::trigger::Trigger;

lazy_mut! {
    static mut SIG_MAP_NAME: SeaMap<String, usize> = SeaMap::new();
}
lazy_mut! {
    static mut SIG_MAP: IntMap<SimObject> = IntMap::new();
}

#[derive(Clone, Copy, Debug)]
pub struct SimObject {
    handle: usize,
    kind: ObjectKind,
    size: i32,
    signed: bool,
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
            .get_full_name(self.handle)
            .expect("Couldn't get name of ObjectInner")
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn is_signed(&self) -> bool {
        SIM_IF.is_signed(self.handle)
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
        let handle = match unsafe { SIG_MAP_NAME.get(full_name) } {
            Some(h) => Some(h.to_owned()),
            _ => None,
        };
        match handle {
            Some(h) => SimObject::from_handle(h),
            _ => Ok(SimObject::new_from_name(full_name)?),
        }
    }

    fn new_from_name(full_name: &str) -> SimpleResult<Self> {
        let handle = SIM_IF.get_handle_by_name(full_name)?;
        Ok(SimObject::new_from_handle(handle))
    }

    fn new_from_handle(handle: usize) -> Self {
        let signal = SimObject {
            handle,
            kind: SIM_IF.get_kind(handle),
            size: SIM_IF.get_size(handle),
            signed: SIM_IF.is_signed(handle),
        };
        unsafe {
            SIG_MAP.insert(handle as u64, signal);
            SIG_MAP_NAME.insert(signal.name(), handle);
        };
        signal
    }

    pub fn get_root() -> SimpleResult<Self> {
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

    pub fn i32(&self) -> i32 {
        if matches!(self.kind, ObjectKind::Bits) && self.size <= 32 {
            let val = SIM_IF.get_value_int(self.handle()).unwrap() as i64;
            // Some simulators don't return negative value for any vector size (Questa)
            let ceil = 1i64 << (self.size - 1);
            if val >= ceil {
                (-2 * ceil + val) as i32
            } else {
                val as i32
            }
        } else {
            panic!("Couldn't get value of {} as i32 type.", self.name());
        }
    }

    pub fn u32(&self) -> u32 {
        if matches!(self.kind, ObjectKind::Bits) && self.size <= 32 {
            let val = SIM_IF.get_value_int(self.handle()).unwrap() as i64;
            let ceil = 1i64 << self.size;
            if val < 0 {
                (val + ceil) as u32
            } else {
                val as u32
            }
        } else {
            panic!("Couldn't get value of {} as u32 type.", self.name());
        }
    }

    pub fn bin(&self) -> String {
        SIM_IF.get_value_bin(self.handle).unwrap()
    }

    pub fn c(&self, name: &str) -> Self {
        self.get_child(name)
            .unwrap_or_else(|_| panic!("Could not get object with name {}.{}", self.name(), name))
    }

    pub fn release(&self) {
        SIM_IF.release(self.handle).unwrap();
    }

    pub fn set(&self, val: i32) {
        self._set(val, false);
    }

    pub fn force(&self, val: i32) {
        self._set(val, true);
    }

    #[inline]
    fn _set(&self, val: i32, force: bool) {
        if !matches!(self.kind, ObjectKind::Bits) {
            panic!(
                "Can't set signal {} of kind {:?} using set() or set_u32()",
                self.name(),
                self.kind
            );
        }
        SIM_IF.set_value_int(self.handle, val, force).unwrap();
    }

    pub fn set_u32(&self, val: u32) {
        self._set_u32(val, false)
    }

    pub fn force_u32(&self, val: u32) {
        self._set_u32(val, true)
    }

    #[inline]
    fn _set_u32(&self, val: u32, force: bool) {
        if val >= 1 << 31 {
            let val_i32: i32 = unsafe { std::mem::transmute(val) };
            self._set(val_i32, force);
        } else {
            self._set(val as i32, force);
        }
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
        let stripped = val.replace("0b", "");
        let stripped = stripped.replace("_", "");
        if stripped.len() == self.size as usize {
            let is_valid = stripped.chars().all(|c| valid_char(c));
            if is_valid {
                SIM_IF.set_value_bin(self.handle, stripped, force).unwrap();
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
