# rstb
The rstb - Rust Test Bench - library aims to provide an easy to use Rust interface to HDL simulators which implement a VPI or VHPI interface.

It is heavily inspired by, if not "stolen from", the awesome and feature rich [cocotb](https://github.com/cocotb/cocotb) which uses Python to the same purpose.

So why bother? I use cocotb professionally and you could call me a fan boy, so this seemed like a good project to learn a new language.

Turns out Rust is ideally suited for this task. It
* interfaces easily with C,
* supports single threaded concurrency in a similar fashion to Python,
* has an extensive and fast growing ecosysem of open source packages,
* and on top is blazingly fast.

When comparing to Python, tests in Rust are more verbose and writing them is a bit more of a hassle because of the static typing and borrow checking, but test execution is a lot faster.

### Current features
- [x] Scheduling simulation callbacks through awaitable abstraction objects (Triggers)
- [x] Runtime to manage scheduling, forking, joining and cancelling of concurrent tasks
- [x] Traversing simulation object hierarchy
- [x] Getting and setting simulation object values
- [x] Macro for easily embedding user level tests

### Feature roadmap
- [x] means to pass/fail a test
- [ ] add support for real types.
- [x] force / release signal values
- [ ] vector slices and arrays
- [x] joining multiple tasks
- [x] concurrent assertions
- [ ] documentation
- [ ] a logging solution and some fancy output formatting
- [ ] a nicer way to start tests from command line
- [ ] junit output for CI
- [ ] Work on VHPI (No simulator I have access to supports it)
- [ ] Support more Simulators
- [ ] ...

There has been some work done to embed Python code using [PyO3](https://github.com/PyO3/pyo3) with the purpose of running cocotb tests on Rstb. At some point this could be taken up again.

### Not on the roadmap
* Windows (although it shouldn't be a big issue)
* Mentor/Siemens FLI

### Rstb works with
* Questa/ModelSim
* [Icarus Verilog](https://github.com/steveicarus/iverilog)
* Cadence simulators

### creating a Rstb test
* Write test (see examples in this project)
* compile as C dynamic library by adding `crate-type = ["cdylib"]` to `Cargo.toml` as with the examples using `cargo build --release`.
* Run with your favorite simulator
  * see run_questa.sh/run_icarus.sh/run_cadence.sh

Run it on gitpod: https://gitpod.io/#https://github.com/benbr8/rstb
