library ieee;
use ieee.std_logic_1164.all;

entity dff is
    port (
        clk: in std_logic;
        rstn: in std_logic;
        d: in std_logic;
        q: out std_logic
    );
end entity;

architecture rtl of dff is

begin

    dff_p: process(clk)
    begin
        if rising_edge(clk) then
            if rstn = '0' then
                q <= '0';
            else
                q <= d;
            end if;
        end if;
    end process;

end architecture;