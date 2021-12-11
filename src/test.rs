
use lazy_mut::lazy_mut;
use futures::future::BoxFuture;
use once_cell::sync::OnceCell;
use crate::rstb_obj::RstbObjSafe;
use crate::signal;
use crate::seamap::SeaMap;
use crate::RstbResult;

#[derive(Debug)]
pub struct RstbTests (
    Vec<RstbObjSafe<Test>>,
);

impl RstbTests {
    pub fn new() -> Self {
        Self ( Vec::new() )
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn iter(&self) -> core::slice::Iter<RstbObjSafe<Test>> {
        self.0.iter()
    }
    pub fn push(&mut self, test: Test) {
        self.0.push(RstbObjSafe::new(test));
    }
}

#[derive(Debug)]
pub struct Test {
    pub name: String,
    pub generator: fn(signal::SimObject) -> BoxFuture<'static, RstbResult>,
    pub time_secs: f64,
    pub sim_time_ns: u64,
    pub result: Option<RstbResult>,
}

impl Test {
    pub fn new(name: String, generator: fn(signal::SimObject) -> BoxFuture<'static, RstbResult>) -> Self {
        Self { name, generator, time_secs: 0.0, sim_time_ns: 0, result: None }
    }
    pub fn set_result(&mut self, result: RstbResult) {
        self.result = Some(result);
    }
}

pub(crate) static TESTS: OnceCell<RstbTests> = OnceCell::new();
