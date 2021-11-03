# rstb
The rstb - Rust Test Bench - library aims to provide an easy to use Rust interface to HDL simulators which implement a VPI or VHPI interface.

It is heavily inspired by, the awesome and feature rich [cocotb](https://github.com/cocotb/cocotb) which uses Python to the same purpose.

So why bother? Turns out Rust is ideally suited for this task. It
* interfaces easily with C,
* since end of 2019, it supports zero-cost futures with async/await syntax, which makes writing testbenches very "ergonomic" and easy to reason about,
* has an extensive and fast growing ecosysem of open source packages,
* and on top is a blazingly fast compiled language with no runtime required.

When comparing to Python, tests in Rust are more verbose and writing them is a bit more of a hassle because of the static typing and borrow checking, but test execution is **a lot** faster.

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

There has been some work done to embed Python code using [PyO3](https://github.com/PyO3/pyo3) with the purpose of running cocotb tests on a Rstb backend. At some point this could be taken up again, although this eats up most of the performance advantage compared to the Rust only approach.

### Not on the roadmap
* Windows (although it shouldn't be a big issue)
* Mentor/Siemens FLI

### Rstb works with
* Questa/ModelSim
* [Icarus Verilog](https://github.com/steveicarus/iverilog)
* Cadence simulators

### creating a Rstb test
* Write test (see examples in this project)
* compile as C compatible dynamic library by adding `crate-type = ["cdylib"]` to `Cargo.toml` as with the examples using `cargo build --release`.
* Run with your favorite simulator
  * see run_questa.sh/run_icarus.sh/run_cadence.sh

Run it on gitpod (using Verilator): https://gitpod.io/#https://github.com/benbr8/rstb
