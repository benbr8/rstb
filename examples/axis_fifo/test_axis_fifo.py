
import logging
from collections import deque

import random

import cocotb
from cocotb.triggers import RisingEdge, Timer, ReadOnly
from cocotb.log import SimLog
from cocotb.result import TestFailure, TestSuccess

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

class Monitor:
    def __init__(self) -> None:
        self.scoreboard = None
        self.exp_not_recv = False

    def set_scoreboard(self, scoreboard: Scoreboard, exp_not_recv):
        self.scoreboard = scoreboard
        self.exp_not_recv = exp_not_recv

    def to_scoreboard(self, data):
        if self.exp_not_recv:
            self.scoreboard.add_exp(data)
        else:
            self.scoreboard.add_recv(data)

class AxisMonitor(Monitor):
    def __init__(self, clk, tvalid, tready, tdata) -> None:
        super().__init__()
        self.clk = clk
        self.tvalid = tvalid
        self.tready = tready
        self.tdata = tdata

    async def run(self):
        while True:
            await RisingEdge(self.clk)
            await ReadOnly()
            if self.tvalid.value == 1 and self.tready.value == 1:
                self.to_scoreboard(self.tdata.value)


class FifoTb:
    def __init__(self, dut):
        self.scoreboard = Scoreboard()
        self.mon_in = AxisMonitor(dut.clk, dut.s_tvalid, dut.s_tready, dut.s_tdata)
        self.mon_out = AxisMonitor(dut.clk, dut.m_tvalid, dut.m_tready, dut.m_tdata)
        self.mon_in.set_scoreboard(self.scoreboard, True)
        self.mon_out.set_scoreboard(self.scoreboard, False)
        cocotb.fork(clock(dut.clk, 10))
        cocotb.fork(self.mon_in.run())
        cocotb.fork(self.mon_out.run())


async def clock(clk, time_ns):
    half = time_ns * 1000 / 2
    while True:
        clk.value = 0
        await Timer(half, "ps")
        clk.value = 1
        await Timer(half, "ps")


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
    dut.s_tvalid <= 0
    dut.m_tready <= 0
    for _ in range(10):
        await RisingEdge(dut.clk)
    dut.rst.value = 0
    for _ in range(2):
        await RisingEdge(dut.clk)


@cocotb.test()
async def default(dut):
    tb = FifoTb(dut)
    mem = MemModel(dut.fifo.mem, 1<<4)
    cocotb.fork(mem.exec())
    cocotb.fork(rd_stim(dut.clk, dut.m_tready))

    await reset(dut)

    for i in range(100_000):
        await RisingEdge(dut.clk)
        if random.random() < 0.5:
            dut.s_tdata <= i
            dut.s_tvalid <= 1
        else:
            dut.s_tvalid <= 0
    dut.s_tvalid <= 0

    await Timer(1, "us")

    if tb.scoreboard.result():
        raise TestSuccess()
    raise TestFailure()
