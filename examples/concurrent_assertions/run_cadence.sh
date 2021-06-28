#!/bin/bash

xrun -64 hdl/dut.sv -access rwc -loadvpi target/release/librstb_test.so:vpi_entry_point
