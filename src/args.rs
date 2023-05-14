use chrono::{NaiveDate, Utc};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct BondCli {
    #[arg(short = 'c', long)]
    pub coupon: f64,

    #[arg(short = 'p', long)]
    pub price: f64,

    #[arg(long, action(clap::ArgAction::SetTrue))]
    pub clean: bool,

    #[arg(short = 'd', long, default_value = "act/act")]
    pub daycount: String,

    #[arg(short = 'f', long, default_value_t = 2.0)]
    pub frequency: f64,

    #[arg(
        short = 's',
        long= "settlement-date",
        default_value_t = Utc::now().date_naive()
    )]
    pub settlementdate: NaiveDate,

    #[arg(short = 'm', long = "maturity-date")]
    pub maturity_date_arg: NaiveDate,
}
