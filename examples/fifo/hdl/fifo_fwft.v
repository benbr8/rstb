`timescale 1ps/1ps


module fifo_fwft
  #(parameter DATA_WIDTH = 8,
    parameter ADDR_WIDTH = 4)
   (
    input                   clk,
    input                   rst,
    input [DATA_WIDTH-1:0]  din,
    input                   wr_en,
    output                  full,
    output [DATA_WIDTH-1:0] dout,
    input                   rd_en,
    output                  empty);

    initial begin
        $dumpfile ("fifo_fwft.vcd");
        $dumpvars (0, fifo_fwft);
        #1;
    end

    wire full_int;
    wire empty_int;
    wire wr_int;
    wire rd_int;
    reg quadrant;
    reg [ADDR_WIDTH-1:0] wr_ptr;
    wire [ADDR_WIDTH-1:0] wr_ptr_next;
    reg [ADDR_WIDTH-1:0] rd_ptr;
    wire [ADDR_WIDTH-1:0] rd_ptr_next;
    reg [ADDR_WIDTH-1:0] count_int_tmp;
    reg [ADDR_WIDTH:0] count_int;
    wire [DATA_WIDTH-1:0] dout_int;
    reg [DATA_WIDTH-1:0] dout_local;
    reg dout_local_strb;

    localparam [ADDR_WIDTH:0] count_max = 2**ADDR_WIDTH;

    assign full = full_int;
    assign empty = empty_int;

    // calc count
    always @ (*) begin
        if (wr_ptr != rd_ptr) begin
            count_int_tmp = wr_ptr - rd_ptr;
            count_int = count_int_tmp;
        end else begin
            if (quadrant == 0)
                count_int = 0;
            else
                count_int = 2**ADDR_WIDTH;

        end
    end

    // calc quadrant
    always @(posedge clk) begin
        if (rst)
            quadrant <= 0;
        else begin
            if (count_int < 2**(ADDR_WIDTH-1))
                quadrant <= 0;
            else
                quadrant <= 1;
        end
    end

    // wr/rd
    assign empty_int = (count_int == 0) ? 1 : 0;
    assign full_int = (count_int == count_max) ? 1 : 0;
    assign wr_int = (!full_int && !rst) ? wr_en : 0;
    assign rd_int = (!empty_int && !rst) ? rd_en : 0;

    // pointer handling
    assign wr_ptr_next = wr_int ? wr_ptr+1 : wr_ptr;
    assign rd_ptr_next = rd_int ? rd_ptr+1 : rd_ptr;


    // calc quadrant
    always @(posedge clk) begin
        if (rst) begin
            wr_ptr <= 0;
            rd_ptr <= 0;
        end else begin
            wr_ptr <= wr_ptr_next;
            rd_ptr <= rd_ptr_next;
        end
    end

    // mem instance
    simple_dpram_sclk
    #(
        .ADDR_WIDTH(ADDR_WIDTH),
        .DATA_WIDTH(DATA_WIDTH)
    )
    mem (
        .clk    (clk),
        .raddr  (rd_ptr_next),
        .dout   (dout_int),
        .we     (wr_int),
        .waddr  (wr_ptr),
        .din    (din)
    );

    // If next read ptr and current write ptr are the same, fast-track the
    // input directly to output.
    always @(posedge clk) begin
        dout_local_strb <= 0;
        if (rst) begin
            dout_local <= 0;
        end else begin
            if (wr_int && (rd_ptr_next == wr_ptr)) begin
                dout_local <= din;
                dout_local_strb <= 1;
            end
        end
    end

    assign dout = dout_local_strb ? dout_local : dout_int;

endmodule