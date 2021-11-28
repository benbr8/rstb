`timescale 1ps/1ps

module dff (
   input d,
   input rstn,
   input clk,
   output reg q
);

   always @ (posedge clk)
   begin
      if (!rstn)
         q <= 0;
      else
         q <= d;
   end

endmodule