#!/bin/bash

vlog -O4 hdl/dut.sv
vsim -c -do ../.questa/questa.do -pli target/release/librstb_test.so dut
