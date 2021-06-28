#!/bin/bash

vlog -O4 hdl/dut.sv
vsim -do ../.questa/questa_gui.do -pli target/release/librstb_test.so dut