
import logging
from collections import deque

import cocotb
from cocotb.triggers import RisingEdge, Timer, ReadOnly
from cocotb.log import SimLog

class Scoreboard:
    def __init__(self):
        self.log = SimLog(self.__class__.__name__)  
        self.log.setLevel(logging.INFO)
        self.errors: int = 0             
        self.expected: int = 0
        self.received: int = 0
        self.matched: int = 0
        self._expQ: deque = deque()
        self._recvQ: deque = deque()
        
    def add_exp(self, trx):
        self._expQ.appendleft(trx)
        self.expected += 1
        if self._expQ and self._recvQ:
            self._compare()

    def add_recv(self, trx):
        self._recvQ.appendleft(trx)
        self.received += 1
        if self._expQ and self._recvQ:
            self._compare()

    def _compare(self):
        exp = self._expQ.pop()
        recv = self._recvQ.pop()
        passed = exp == recv
        if not passed:  
            self.errors += 1
        else:
            self.matched += 1
    
    def print_stats(self):
        self.log.info("expected=%s, received=%s, matched=%s, errors=%s, expQ: %s, recvQ: %s", self.expected, \
             self.received, self.matched, self.errors, len(self._expQ), len(self._recvQ))
             
    def result(self) -> bool:
        self.print_stats()
        if self.errors or not self.expected or self.expected != self.received or self._expQ or self._recvQ:
            return False
        return True

class DffTb:
    def __init__(self):
        self.scoreboard = Scoreboard()

    async def monitor_in(self, clk, signal):
        while True:
            await RisingEdge(clk)
            self.scoreboard.add_exp(int(signal))

    async def monitor_out(self, clk, signal):
        while True:
            await RisingEdge(clk)
            self.scoreboard.add_recv(int(signal))

async def clock(clk, time_ns):
    half = time_ns / 2
    while True:
        clk <= 0
        await Timer(half, "ns")
        clk <= 1
        await Timer(half, "ns")


async def d_stim(clk, d):
    d <= 0
    while True:
        await RisingEdge(clk)
        d <= (int(d) + 1) % 2

async def reset(dut):
    dut.rstn <= 0
    for _ in range(10):
        await RisingEdge(dut.clk)
    dut.rstn <= 1
    for _ in range(10):
        await RisingEdge(dut.clk)


@cocotb.test()
async def default(dut):
    tb = DffTb()
    clock_task = cocotb.fork(clock(dut.clk, 8))
    await reset(dut)
    cocotb.fork(d_stim(dut.clk, dut.d))
    cocotb.fork(tb.monitor_in(dut.clk, dut.d))
    cocotb.fork(tb.monitor_out(dut.clk, dut.d))
    
    await Timer(3, "ms")
    clock_task.kill()
    
    await Timer(100, "ns")
    tb.scoreboard.print_stats()

