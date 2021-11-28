import random
import cocotb
from cocotb.triggers import ReadWrite, RisingEdge, Timer
from cocotb.result import TestFailure, TestSuccess


async def clock(clk, time_ns):
    half = time_ns / 2
    while True:
        clk.value = 0
        await Timer(half, "ns")
        clk.value = 1
        await Timer(half, "ns")



@cocotb.test()
async def default(dut):

    # Fork clock input to run concurrently
    cocotb.fork(clock(dut.clk, 8))

    for _ in range(100_000):
        d = random.randint(0, 1)
        dut.d.value = d
        await RisingEdge(dut.clk)
        await ReadWrite()
        if dut.q.value != d:
            raise TestFailure("Q output did not match D input")

    raise TestSuccess("All transactions matched")

