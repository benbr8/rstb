#!/bin/bash

cargo build --release
vlog -O4 hdl/mem.v hdl/fifo_fwft.v
vsim -do ../.questa/questa_gui.do -pli target/release/librstb_fifo.so fifo_fwft