## Welcome!
.. to my cli tool to help quickly analyse a bond. The args are structured like so: 

```bash
Usage: bond-analyzer.exe [OPTIONS] --coupon <COUPON> --price <PRICE> --maturity-date <MATURITY_DATE_ARG>  

Options:
  -c, --coupon <COUPON>
  -p, --price <PRICE>
      --clean
  -d, --daycount <DAYCOUNT>                [default: act/act]
  -f, --frequency <FREQUENCY>              [default: 2]
  -s, --settlement-date <SETTLEMENTDATE>   [default: today]
  -m, --maturity-date <MATURITY_DATE_ARG>
  -h, --help                               Print help
  -V, --version                            Print version
```  

Example run 
```bash
cargo run -- -c 1.375 -p 99.974 -m 2025-01-31 -s 2020-02-20
```
  
The only 3 mandatory args are the coupon, price and maturity date; from that the cli will calc the yield-to-maturity, macaulay duration and modified duration. These, along with the remaining cashflows are printed to the terminal. 
  
The tool assumes you are passing the dirty price (price+accrued interest). If the --clean flag is passed, it will calculate the dirty price from the price passed in. 
  
The --settlement date (-s) will calculate the metrics and remaining cashflows from the date passed in. If it is ommitted, it will calculate the metrics as of today.    

Calculations have been implemented per: https://research.ftserussell.com/products/downloads/FTSE_Fixed_Income_Index_Guide_to_Calculation_new.pdf

