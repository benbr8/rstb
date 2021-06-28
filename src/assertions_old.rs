use crate::prelude::*;
use crate::seamap::SeaMap;
use futures::future::{select_all, BoxFuture, Future};
use lazy_mut::lazy_mut;

lazy_mut! { pub static mut SEQUENCE_MAP: SeaMap<String, Sequence> = SeaMap::new(); }
lazy_mut! { pub static mut ASSERTION_MAP: SeaMap<String, Assertion> = SeaMap::new(); }

type Generator = RstbObj<Box<dyn Fn() -> BoxFuture<'static, RstbResult>>>;

// #[macro_export]
// macro_rules! assertion_with_condition {
//     ($name: expr, $checker: expr, $condition: expr, $triggers: expr) => {
//         Assertion::try_add_assertion($name, move || $checker.boxed(), move || $condition.boxed(), $triggers);
//     };
// }
// #[macro_export]
// macro_rules! assertion {
//     ($name: expr, $checker: expr, $triggers: expr) => {
//         assertion_with_condition!($name, $checker, async move { Ok(Val::None) }.boxed(), $triggers);
//     };
// }
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
macro_rules! sequence {
    ($i: expr, $j: expr) => {
        Sequence::try_add_sequence(move || $j.boxed(), $i);
    };
}

pub struct Sequence {
    name: String,
    generator: Generator,
}

impl Sequence {
    pub fn try_add_sequence(
        fut: impl Fn() -> BoxFuture<'static, RstbResult> + 'static,
        name: &str,
    ) {
        let seq = Self {
            name: name.to_string(),
            generator: RstbObj::new(Box::new(fut)),
        };
        unsafe {
            SEQUENCE_MAP.init();
            if !SEQUENCE_MAP.contains_key(name) {
                SEQUENCE_MAP.insert(name.to_string(), seq);
            }
        }
    }
    pub fn gen(&self) -> impl Future<Output = RstbResult> + Send + 'static {
        let a = (self.generator.get())();
        async move { a.await }
    }
    pub fn get(name: &str) -> impl Future<Output = RstbResult> + Send + 'static {
        unsafe {
            if SEQUENCE_MAP.contains_key(name) {
                SEQUENCE_MAP.get(name).unwrap().gen()
            } else {
                panic!("Sequence '{}' wasn't previously defined.", name);
            }
        }
    }
}

pub struct Assertion {
    name: String,
    enabled: bool,
    triggers: Vec<Trigger>,
    condition: Generator,
    checker: Generator,
    triggered: RstbObj<u32>,
    failed: RstbObj<u32>,
    passed: RstbObj<u32>,
}

impl Assertion {
    #[allow(unreachable_code)]
    async fn run(&'static self) -> RstbResult {
        loop {
            // await trigger
            let mut trig_list = Vec::with_capacity(self.triggers.len());
            for trig in self.triggers.iter().cloned() {
                trig_list.push(Task::fork(async move {
                    trig.await;
                    SIM_IF.log("triggered!");
                    Ok(Val::None)
                }));
            }
            // cancel remaining tasks. TODO: reuse without cancel + reschedule
            let (_, _, rem_vec) = select_all(trig_list).await;
            SIM_IF.log("Select awaited.");
            for rem in rem_vec {
                rem.cancel()
            }
            Trigger::read_only().await;

            // run
            if self.enabled {
                let condition = (self.condition.get())();
                let checker = (self.checker.get())();
                let _task = Task::fork(async move {
                    condition.await?;
                    self.trigger();
                    let r = checker.await;
                    SIM_IF.log(&format!("checker = {:?}", r));
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
    pub fn try_add_assertion(
        name: &str,
        checker: impl Fn() -> BoxFuture<'static, RstbResult> + 'static,
        condition: impl Fn() -> BoxFuture<'static, RstbResult> + 'static,
        triggers: Vec<Trigger>,
    ) {
        let assertion = Self {
            name: name.to_string(),
            enabled: true,
            triggers,
            // condition: None,
            condition: RstbObj::new(Box::new(condition)),
            checker: RstbObj::new(Box::new(checker)),
            triggered: RstbObj::new(0),
            failed: RstbObj::new(0),
            passed: RstbObj::new(0),
        };
        unsafe {
            ASSERTION_MAP.init();
            if !ASSERTION_MAP.contains_key(name) {
                ASSERTION_MAP.insert(name.to_string(), assertion);
            }
        }
    }
    fn trigger(&self) {
        *self.triggered.get_mut() += 1;
    }
    fn fail(&self) {
        if self.enabled {
            *self.failed.get_mut() += 1;
        }
    }
    fn pass(&self) {
        if self.enabled {
            *self.passed.get_mut() += 1;
        }
    }
    pub fn result_str(&self) -> String {
        format!(
            "Assertion {}: Triggered: {}, Passed: {}, Failed: {}.",
            self.name,
            self.triggered.get(),
            self.passed.get(),
            self.failed.get()
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

pub fn print_assertion_stats() {
    unsafe {
        ASSERTION_MAP.init();
        for (_, assertion) in ASSERTION_MAP.iter() {
            SIM_IF.log(assertion.result_str().as_str());
        }
    }
}