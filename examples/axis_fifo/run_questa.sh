#!/bin/bash

cargo build --release
vlog -O4 hdl/mem.v hdl/fifo_fwft.v hdl/axis_fifo.v
vsim -c -do ../.questa/questa.do -pli target/release/librstb_fifo.so axis_fifo