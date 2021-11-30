`timescale 1ps/1ps

module dut (
    input rstn,
    input clk,
    input req,
    output reg ack);

// initial begin
//     $dumpfile ("dut.vcd");
//     $dumpvars (0, dut);
//     #1;
// end

reg [2:0] req_d;

assign ack = req_d[2];

always @ (posedge clk) begin
    if (!rstn) begin
        req_d <= 0;
    end else begin
        req_d <= req_d << 1;
        req_d[0] <= req;
    end
end

endmodule