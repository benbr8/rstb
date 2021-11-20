
module simple_dpram_sclk
  #(
    parameter ADDR_WIDTH = 4,
    parameter DATA_WIDTH = 32
    )
   (
    input 		    clk,
    input [ADDR_WIDTH-1:0]  raddr,
    input [ADDR_WIDTH-1:0]  waddr,
    input 		    we,
    input [DATA_WIDTH-1:0]  din,
    output reg [DATA_WIDTH-1:0] dout
    );

   reg [DATA_WIDTH-1:0]  mem[(2**ADDR_WIDTH)-1:0];

   always @(posedge clk) begin
      if (we) begin
	      mem[waddr] <= din;
      end

      dout <= mem[raddr];
   end

endmodule