#!/bin/bash

vlog -O4 hdl/dff.v
vsim -c -do ../.questa/questa.do -pli target/release/librstb_dff.so dff
