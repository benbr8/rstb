
import logging
from collections import deque

import random

import cocotb
from cocotb.triggers import RisingEdge, Timer, ReadOnly
from cocotb.log import SimLog

class MemModel:
    def __init__(self, dut, depth) -> None:
        self.mem = [0] * depth
        self.dut = dut

    async def exec(self):
        while True:
            await RisingEdge(self.dut.clk)
            await Timer(1, "step")

            raddr = int(self.dut.raddr)
            data = self.mem[raddr]
            self.dut.dout <= data
            if int(self.dut.we) == 1:
                waddr = int(self.dut.waddr)
                data = int(self.dut.din)
                self.mem[waddr] = data


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

class FifoTb:
    def __init__(self):
        self.scoreboard = Scoreboard()

    async def monitor_in(self, dut):
        while True:
            await RisingEdge(dut.clk)
            await ReadOnly()
            if int(dut.wr_en) == 1 and int(dut.full) == 0:
                self.scoreboard.add_exp(int(dut.din))

    async def monitor_out(self, dut):
        while True:
            await RisingEdge(dut.clk)
            await ReadOnly()
            if int(dut.rd_en) == 1 and int(dut.empty) == 0:
                self.scoreboard.add_recv(int(dut.dout))


async def clock(clk, time_ns):
    half = time_ns / 2
    while True:
        clk.value = 0
        await Timer(half, "ns")
        clk.value = 1
        await Timer(half, "ns")


async def rd_stim(clk, rd):
    rd <= 0
    while True:
        await RisingEdge(clk)
        if random.random() < 0.5:
            rd <= 1
        else:
            rd <= 0

async def reset(dut):
    dut.rst.value = 1
    dut.rd_en <= 0
    dut.wr_en <= 0
    for _ in range(10):
        await RisingEdge(dut.clk)
    dut.rst.value = 0
    for _ in range(2):
        await RisingEdge(dut.clk)


@cocotb.test()
async def default(dut):
    tb = FifoTb()
    cocotb.fork(clock(dut.clk, 8))
    await reset(dut)
    mem = MemModel(dut.mem, 1<<4)
    cocotb.fork(mem.exec())
    cocotb.fork(rd_stim(dut.clk, dut.rd_en))
    cocotb.fork(tb.monitor_in(dut))
    cocotb.fork(tb.monitor_out(dut))

    for i in range(100_000):
        await RisingEdge(dut.clk)
        if random.random() < 0.5:
            dut.din <= i
            dut.wr_en <= 1
    dut.wr_en <= 0

    await Timer(1, "us")

    tb.scoreboard.print_stats()
