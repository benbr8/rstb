#!/bin/bash

if [ -d ".sim_build" ]; then
	rm -rf .sim_build
fi
mkdir .sim_build
cp target/release/librstb_test.so target/release/librstb_test.vpi
iverilog -o .sim_build/sim.vvp -s dut -g2012 hdl/dut.sv
vvp -M target/release -m librstb_test .sim_build/sim.vvp
