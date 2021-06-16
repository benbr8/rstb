#!/bin/bash

vlog -O4 hdl/dff.v
vsim -do ../.questa/questa_gui.do -pli target/release/librstb_dff.so  dff