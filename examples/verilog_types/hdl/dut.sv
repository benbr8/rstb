`timescale 1ps/1ps

module dut (
    input d,
    input rstn,
    input clk,
    input [7:0] data,
    output reg q);

localparam constant = 4;

reg [7:0] reg8;
reg signed [7:0] reg8_signed;
reg signed [7:0] reg8_ff;
reg [31:0] reg32;
reg signed [31:0] reg32_signed;
reg signed [31:0] reg31_signed;
reg [0:0] reg1;
reg signed [0:0] reg1_signed;
reg reg_;
integer integer_;
int int_;
longint longint_;
bit [7:0] bit8;
bit bit_;
shortreal shortreal_;
real real_;
logic logic_;

reg [64:0] bits65;


always @ (posedge clk) begin
    if (!rstn) begin
        q <= 0;
    end else begin
        logic_ <= 1;
        bits65[32] <= 1;
        bits65[0] <= 1;
        bit_ <= bit8[0];
        real_ <= shortreal_;
        reg8[7] <= 1;
        reg8_signed[7] <= 1;
        reg32[31] <= 1;
        reg32_signed[31] <= 1;
        reg31_signed[30] <= 1;
        reg8_ff <= 8'hff;
        reg1[0] <= 1;
        reg1_signed[0] <= 1;
        longint_ <= int_;
        reg_ <= reg8[0];
        integer_ <= integer_ + reg8;
        int_ <= constant;
        q <= d;
    end
end

endmodule