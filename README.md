# rstb
The rstb - Rust Test Bench - library aims to provide an easy to use Rust interface to HDL simulators which implement a VPI or VHPI interface.

It is heavily inspired by, if not "stolen from", the awesome and feature rich [cocotb](https://github.com/cocotb/cocotb) which uses Python to the same purpose.

So why bother? I use cocotb professionally and you could call me a fan boy, so this seemed like a good project to learn a new language.

Turns out Rust is ideally suited for this task. It
* interfaces easily with C,
* supports single threaded concurrency in a similar fashion to Python,
* has an extensive and fast growing ecosysem of open source packages,
* and on top is blazingly fast.

Some disadvantages when comparing to Python:
* I found the learning curve to be much steeper.
* Static typing and the data ownership checking makes implementing tests slower and ...
* much more verbose. Python tests look a lot cleaner.

### Current features
- [x] Scheduling simulation callbacks through awaitable abstraction objects (Triggers)
- [x] Runtime to manage scheduling, forking, joining and cancelling of concurrent tasks
- [x] Traversing simulation object hierarchy
- [x] Getting and setting simulation object values
- [x] Macro for easily embedding user level tests

### Feature roadmap
- [ ] means to pass/fail a test
- [ ] more types of sim object value access (bin string, real, ...). Currently only integer access is supported.
- [ ] force / release signal values
- [ ] vector slices and arrays
- [ ] a logging solution and some fancy output formatting
- [ ] a nicer way to start tests from command line
- [ ] junit output for CI
- [ ] get VHPI to run (currently I can only test on Questa, with which I have some VHPI issues)
- [ ] Test with Verilator and maybe more OS simulators
- [ ] ...

There has been some work done to embed Python code using [PyO3](https://github.com/PyO3/pyo3) with the purpose of running cocotb tests on rstb. At some point this could be taken up again.

### Not on the roadmap
* Windows (although it shouldn't be a big issue)
* ModelSim FLI

### creating a rstb test
* Write test (see examples in this project)
* compile as C dynamic library by adding using `crate-type = ["cdylib"]`
* Run with your favorite simulator (tested with Questa (VPI) and Icarus)
  * see run_questa.sh and run_icarus.sh

Run it on gitpod: https://gitpod.io/#https://github.com/benbr8/rstb
