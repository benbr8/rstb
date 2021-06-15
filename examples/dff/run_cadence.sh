#!/bin/bash

xrun -64 hdl/dff.v -access rwc -loadvpi target/release/librstb_dff.so:vpi_entry_point
