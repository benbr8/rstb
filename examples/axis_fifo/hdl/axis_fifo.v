`timescale 1ps/1ps


module axis_fifo
  #(parameter DATA_WIDTH = 8,
    parameter ADDR_WIDTH = 4)
   (
    input                   clk,
    input                   rst,
    input [DATA_WIDTH-1:0]  s_tdata,
    input                   s_tvalid,
    output                  s_tready,
    output [DATA_WIDTH-1:0] m_tdata,
    input                   m_tready,
    output                  m_tvalid);

    // initial begin
    //     $dumpfile ("axis_fifo.vcd");
    //     $dumpvars (0, axis_fifo);
    //     #1;
    // end

    wire full, empty;

    assign s_tready = !full;
    assign m_tvalid = !empty;

    // fifo instance
    fifo_fwft
    #(
        .ADDR_WIDTH(ADDR_WIDTH),
        .DATA_WIDTH(DATA_WIDTH)
    )
    fifo (
        .clk    (clk),
        .rst    (rst),
        .din    (s_tdata),
        .wr_en  (s_tvalid),
        .full   (full),
        .dout   (m_tdata),
        .rd_en  (m_tready),
        .empty  (empty)
    );

endmodule