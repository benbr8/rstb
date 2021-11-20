#!/bin/bash
cargo build --release
if [ -d ".sim_build" ]; then
	rm -rf .sim_build
fi
mkdir .sim_build
cp target/release/librstb_fifo.so target/release/librstb_fifo.vpi
iverilog -o .sim_build/sim.vvp -s fifo_fwft -g2012 hdl/mem.v hdl/fifo_fwft.v
vvp -M target/release -m librstb_fifo .sim_build/sim.vvp
