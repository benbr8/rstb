#!/bin/bash

cargo build --release
vlog -O4 hdl/mem.v hdl/fifo_fwft.v
vsim -c -do ../.questa/questa.do -pli target/release/librstb_fifo.so fifo_fwft