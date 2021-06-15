#!/bin/bash

xrun -64 hdl/dff.v -loadvpi target/release/librstb_test.so:vpi_entry_point
