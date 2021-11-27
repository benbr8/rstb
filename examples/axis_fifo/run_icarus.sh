#!/bin/bash
cargo build --release
if [ -d ".sim_build" ]; then
	rm -rf .sim_build
fi
mkdir .sim_build
cp target/release/librstb_fifo.so target/release/librstb_fifo.vpi
iverilog -o .sim_build/sim.vvp -s axis_fifo -g2012 hdl/mem.v hdl/fifo_fwft.v hdl/axis_fifo.v
vvp -M target/release -m librstb_fifo .sim_build/sim.vvp
