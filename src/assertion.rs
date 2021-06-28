use crate::prelude::*;
use crate::seamap::SeaMap;
use futures::future::{select_all, BoxFuture, Future};
use intmap::IntMap;
use lazy_mut::lazy_mut;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::VecDeque;

// lazy_mut! { pub static mut SEQUENCE_MAP: SeaMap<String, Sequence> = SeaMap::new(); }
lazy_mut! { pub static mut ASSERTION_MAP: SeaMap<String, Assertion> = SeaMap::new(); }

type Generator = RstbObj<Box<dyn Fn() -> BoxFuture<'static, RstbResult>>>;
type SequenceGenerator = RstbObj<Box<dyn Fn(AssertionContext) -> BoxFuture<'static, RstbResult>>>;

#[macro_export]
macro_rules! add_assertion {
    ($name: expr, $trigger: expr, $condition: expr, $checker: expr, $keep_hist: expr, $hist_depth: expr) => {
        Assertion::add_assertion(
            $name,
            $trigger,
            move || $condition.boxed(),
            move |ctx| $checker(ctx).boxed(),
            $keep_hist,
            $hist_depth,
        );
    };
    ($name: expr, $trigger: expr, $condition: expr, $checker: expr) => {
        Assertion::add_assertion(
            $name,
            $trigger,
            move || $condition.boxed(),
            move |ctx| $checker(ctx).boxed(),
            vec![],
            0,
        );
    };
}

#[macro_export]
macro_rules! check {
    ($bool: expr) => {
        match $bool {
            true => Ok(Val::None),
            false => Err(Val::None),
        }
    };
}

#[macro_export]
macro_rules! rose {
    ($signal: expr) => {
        if signal.i32() == ctx.hist()
    };
}

// #[macro_export]
// macro_rules! sequence {
//     ($i: expr, $j: expr) => {
//         Sequence::try_add_sequence(move || $j.boxed(), $i);
//     };
// }

struct SigHist {
    enabled: bool,
    history: IntMap<VecDeque<Val>>,
    task_hdl: Option<JoinHandle>,
}

struct Stats {
    triggered: u32,
    failed: u32,
    passed: u32,
}

#[derive(Clone)]
struct AssertionContextInner {
    trigger: Trigger,
    history: RstbObj<SigHist>,
    dut: SimObject,
}

#[derive(Clone)]
pub struct AssertionContext(AssertionContextInner);

impl AssertionContext {
    pub fn trig(&self) -> Trigger {
        self.0.trigger.clone()
    }
    pub fn dut(&self) -> SimObject {
        self.0.dut.clone()
    }
    pub fn sig_hist(&self, sig: SimObject, clocks: usize) -> Val {
        let hist_ref = self.hist();
        let hist = hist_ref
            .history
            .get(sig.handle() as u64)
            .expect("Could not find history in context.");
        hist[clocks].clone()
    }
    fn hist(&self) -> Ref<SigHist> {
        self.0.history.get()
    }
    pub fn rose(&self, sig: SimObject) -> bool {
        sig.u32() == 1 && self.sig_hist(sig, 1) != Val::Int(1)
    }
    pub fn fell(&self, sig: SimObject) -> bool {
        sig.u32() == 0 && self.sig_hist(sig, 1) != Val::Int(0)
    }
    pub fn stable(&self, sig: SimObject) -> bool {
        self.sig_hist(sig, 1) == Val::Int(sig.u32())
    }
    pub fn changed(&self, sig: SimObject) -> bool {
        self.sig_hist(sig, 1) != Val::Int(sig.u32())
    }
    fn hist_mut(&self) -> RefMut<SigHist> {
        self.0.history.get_mut()
    }
}

pub struct Assertion {
    name: String,
    enabled: RstbObj<bool>,
    condition: Generator,
    checker: SequenceGenerator,
    ctx: AssertionContext,
    stats: RstbObj<Stats>,
}

// impl Default for Assertion {
//     fn default() -> Self {
//         Assertion {
//             name: "default".to_string(),
//             enabled: RstbObj::new(true),
//             condition: RstbObj::new(Box::new(move || async move { Ok(Val::None) }.boxed())),
//             checker: RstbObj::new(Box::new(move || async move { Ok(Val::None) }.boxed())),

//         }
//     }
// }

impl Assertion {
    #[allow(unreachable_code)]
    async fn run(&'static self) -> RstbResult {
        loop {
            // start history task if it is not already running
            if self.ctx.hist().task_hdl.is_none() && self.ctx.hist().enabled {
                let task_hdl = Task::fork(async move {
                    loop {
                        self.ctx.trig().await;
                        // trigger with prio ensures execution before triggers without
                        Trigger::read_only_prio().await;
                        for (hdl, val_vec) in self.ctx.hist_mut().history.iter_mut() {
                            val_vec.pop_back();
                            let obj = SimObject::from_handle(*hdl as usize)
                                .expect("Could not resolve SimObject from handle.");
                            val_vec.push_front(Val::Int(obj.u32()));
                        }
                    }
                    Ok(Val::None)
                });
                self.ctx.hist_mut().task_hdl = Some(task_hdl);
            }

            // await trigger
            self.ctx.trig().await;
            Trigger::read_only().await;

            // run checker task
            if *self.enabled.get() {
                let condition = (self.condition.get())();
                let checker = (self.checker.get())(self.ctx.clone());
                let _task = Task::fork(async move {
                    condition.await?;
                    self.trigger();
                    let r = checker.await;
                    // SIM_IF.log(&format!("checker = {:?}", r));
                    match r {
                        Ok(_) => self.pass(),
                        Err(_) => self.fail(),
                    }
                    Ok(Val::None)
                });
            }
        }
        Ok(Val::None)
    }
    pub fn add_assertion(
        name: &str,
        trigger: Trigger,
        condition: impl Fn() -> BoxFuture<'static, RstbResult> + 'static,
        checker: impl Fn(AssertionContext) -> BoxFuture<'static, RstbResult> + 'static,
        keep_hist: Vec<SimObject>,
        hist_depth: u32,
    ) {
        let mut sig_hist = IntMap::new();
        for obj in keep_hist {
            let vec = vec![Val::None; (hist_depth + 1) as usize];
            sig_hist.insert(obj.handle() as u64, vec.into());
        }
        let assertion = Self {
            name: name.to_string(),
            enabled: RstbObj::new(true),
            ctx: AssertionContext(AssertionContextInner {
                dut: SimObject::get_root().unwrap(),
                trigger,
                history: RstbObj::new(SigHist {
                    enabled: hist_depth != 0,
                    history: sig_hist,
                    task_hdl: None,
                }),
            }),
            condition: RstbObj::new(Box::new(condition)),
            checker: RstbObj::new(Box::new(checker)),
            stats: RstbObj::new(Stats {
                triggered: 0,
                passed: 0,
                failed: 0,
            }),
        };
        unsafe {
            ASSERTION_MAP.init();
            if !ASSERTION_MAP.contains_key(name) {
                ASSERTION_MAP.insert(name.to_string(), assertion);
            }
        }
    }
    fn trigger(&self) {
        self.stats.get_mut().triggered += 1;
    }
    fn fail(&self) {
        self.stats.get_mut().failed += 1;
    }
    fn pass(&self) {
        self.stats.get_mut().passed += 1;
    }
    pub fn enable(&self) {
        *self.enabled.get_mut() = true;
    }
    pub fn disable(&self) {
        *self.enabled.get_mut() = false;
    }
    pub fn result_str(&self) -> String {
        let stats = self.stats.get_mut();
        format!(
            "Assertion {}: Triggered: {}, Passed: {}, Failed: {}.",
            self.name, stats.triggered, stats.passed, stats.failed
        )
    }
}

pub fn run_all_assertions() {
    unsafe {
        for (_, assertion) in ASSERTION_MAP.iter() {
            let fut = assertion.run();
            Task::fork(fut);
        }
    }
}
pub fn disable_all_assertions() {
    unsafe {
        for (_, assertion) in ASSERTION_MAP.iter() {
            assertion.disable()
        }
    }
}
pub fn enable_all_assertions() {
    unsafe {
        for (_, assertion) in ASSERTION_MAP.iter() {
            assertion.enable()
        }
    }
}

pub fn run_assertion(name: &str) {
    unsafe {
        if ASSERTION_MAP.contains_key(name) {
            let fut = ASSERTION_MAP.get(name).unwrap().run();
            Task::fork(fut);
        } else {
            panic!("Assertion {} not previously defined!", name);
        }
    }
}

pub(crate) fn tear_down_assertions() {
    unsafe {
        for (_, a) in ASSERTION_MAP.iter() {
            // Future will be dropped, once all references (`Trigger`s, `JoinHandle`s) are dropped
            a.ctx.hist_mut().task_hdl.take();

            // If assertion has triggered, but not completed, count it as failed
            let mut stats = a.stats.get_mut();
            stats.failed += stats.triggered - stats.passed - stats.failed;
        }
    }
}

pub fn print_assertion_stats() {
    unsafe {
        ASSERTION_MAP.init();
        for (_, assertion) in ASSERTION_MAP.iter() {
            SIM_IF.log(assertion.result_str().as_str());
        }
    }
}
