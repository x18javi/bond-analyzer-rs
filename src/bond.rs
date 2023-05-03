use chrono::{Datelike, Months, NaiveDate};
use std::collections::BTreeMap;
use tabled::{builder::Builder, Table};
use thiserror::Error;
use yearfrac::DayCountConvention;

use crate::args;

#[derive(Debug)]
pub struct Bond {
    pub coupon: f64,
    pub price: f64,
    pub day_count: DayCountConvention,
    pub frequency: u32,
    pub settlement_date: NaiveDate,
    pub maturity_date: NaiveDate,
    pub cashflow_curve: Vec<NaiveDate>,
    pub ytm: f64,
}

#[derive(Error, Debug)]
pub enum BondCalculatorError {
    #[error(
        "invalid daycount value [ {daycount} ]. Has to be one of: nasd30/360, act/act, act360, act365, eur30/360."
    )]
    DaycountError { daycount: String },
    #[error("tried to create a bad date with {year} {month} {day}. Cannot continue.")]
    DateError { year: i32, month: u32, day: u32 },
    #[error("couldn't access element in cashflows. Cannot continue.")]
    CurveError,
}

impl Bond {
    pub fn new(bond: args::BondCli) -> Result<Self, BondCalculatorError> {
        let coupon = bond.coupon;

        let day_count = DayCountConvention::from_str(&bond.daycount).map_err(|_| {
            BondCalculatorError::DaycountError {
                daycount: bond.daycount,
            }
        })?;

        let cashflow_curve = build_curve_dates(
            &bond.maturity_date_arg,
            &bond.settlementdate,
            &bond.frequency,
        )?;

        Ok(Self {
            coupon,
            price: bond.price,
            day_count,
            frequency: bond.frequency,
            settlement_date: bond.settlementdate,
            maturity_date: bond.maturity_date_arg,
            cashflow_curve,
            ytm: 0_f64,
        })
    }

    fn cashflows(&self) -> BTreeMap<&NaiveDate, f64> {
        let mut cashflows: BTreeMap<&NaiveDate, f64> = BTreeMap::new();
        let coupon_split = self.coupon / self.frequency as f64;

        for d in self.cashflow_curve.iter() {
            if d == &self.maturity_date {
                cashflows.insert(d, coupon_split + 100_f64);
            } else {
                cashflows.insert(d, coupon_split);
            }
        }
        cashflows
    }

    fn sum_pv(&self, rate: f64) -> f64 {
        let mut pv = 0 as f64;
        let rate_adj = rate / self.frequency as f64;

        let f = &self.unaccrued_fraction();
        for (i, cf) in self.cashflows().values().enumerate() {
            pv += cf / (((rate_adj) + 1_f64).powf(i as f64 + f))
        }
        pv
    }

    fn create_yield_to_maturity(&mut self) {
        self.ytm = self.bisection_find(0_f64, 2_f64);
    }

    fn accrued_fraction(&self) -> Result<f64, BondCalculatorError> {
        // create the previous coupon in the curve by:
        //  1. get the first coupon in our curve, which needs to fail if it cant be found
        //  2. subtract from this 12 divided by the payment frequency. This needs to fail if that date cannot be made
        let prev_coupon = self
            .cashflow_curve
            .last()
            .ok_or(BondCalculatorError::CurveError)
            .and_then(|next_coupon| {
                next_coupon
                    .checked_sub_months(Months::new(12 / self.frequency))
                    .ok_or(BondCalculatorError::DateError {
                        year: next_coupon.year(),
                        month: next_coupon.month(),
                        day: next_coupon.day(),
                    })
            })?;

        let days_since_last_coupon: f64 =
            self.day_count.yearfrac(prev_coupon, self.settlement_date) * self.frequency as f64;
        Ok(days_since_last_coupon)
    }

    fn unaccrued_fraction(&self) -> f64 {
        1_f64 - self.accrued_fraction().unwrap()
    }

    fn bisection_find(&self, low: f64, high: f64) -> f64 {
        let mid = (high + low) / 2_f64;
        let pv = self.sum_pv(mid);

        if (high - low).abs() > 0.00000001 {
            if pv > self.price {
                self.bisection_find(mid, high)
            } else {
                self.bisection_find(low, mid)
            }
        } else {
            mid
        }
    }

    fn macaulay_duration(&self) -> f64 {
        let mut pv = 0 as f64;
        let rate_adj = self.ytm / self.frequency as f64;
        let f = &self.unaccrued_fraction();

        for (i, cf) in self.cashflows().values().enumerate() {
            pv += cf * ((f + i as f64) / ((rate_adj + 1_f64).powf(f + i as f64)));
        }
        (pv / self.price) / self.frequency as f64
    }

    fn modified_duration(&self) -> f64 {
        self.macaulay_duration() / (1_f64 + (self.ytm) / self.frequency as f64)
    }

    pub fn analysis_table(&mut self) -> Table {
        self.create_yield_to_maturity();

        let mut builder = Builder::default();
        builder
            .set_header(["Metric", "Result"])
            .push_record(["YTM", &(round_to_3dp(self.ytm * 100_f64))])
            .push_record(["Macaulay Duration", &round_to_3dp(self.macaulay_duration())])
            .push_record(["Modified Duration", &round_to_3dp(self.modified_duration())]);

        let table = builder.build();
        table
    }

    pub fn cashflows_table(&self) -> Table {
        let mut builder = Builder::default();
        builder.set_header(["Date", "Coupon"]);
        for (d, c) in self.cashflows().iter() {
            builder.push_record([d.to_string(), c.to_string()]);
        }
        let table = builder.build();
        table
    }
}

fn ndays_in_month(year: i32, month: u32) -> Option<u32> {
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let d = NaiveDate::from_ymd_opt(y, m, 1);
    match d {
        Some(d) => Some(d.pred_opt().unwrap().day()),
        _ => None,
    }
}

fn round_to_3dp(x: f64) -> String {
    let x1 = x * 1000_f64;
    format!("{}", x1.round() / 1000_f64)
}

fn build_curve_dates(
    maturity_date: &NaiveDate,
    settlement_date: &NaiveDate,
    frequency: &u32,
) -> Result<Vec<NaiveDate>, BondCalculatorError> {
    let mut curve: Vec<NaiveDate> = vec![];
    let mut cf_date: NaiveDate = maturity_date.clone();

    curve.push(cf_date);
    loop {
        cf_date = cf_date
            .checked_sub_months(Months::new(12 / frequency))
            .ok_or_else(|| BondCalculatorError::DateError {
                year: cf_date.year(),
                month: cf_date.month(),
                day: cf_date.day(),
            })?;

        if cf_date <= *settlement_date {
            break;
        }

        let year = cf_date.year();
        let month = cf_date.month();
        let mut day = cf_date.day();

        if let Some(max_day_in_month) = ndays_in_month(year, month) {
            if max_day_in_month < cf_date.day() {
                day = max_day_in_month
            }
        }
        let cf_date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| BondCalculatorError::DateError { year, month, day })?;

        curve.push(cf_date);
    }
    Ok(curve)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_to_3dp() {
        let val = 1.3961324324;
        assert_eq!(round_to_3dp(val), "1.396".to_string())
    }

    #[test]
    fn test_cashflow_dates_gilt() {
        // https://www.dmo.gov.uk/media/qncg02s4/prosp140722.pdf
        let maturity_date = NaiveDate::from_ymd_opt(2025, 01, 31).unwrap();
        let settlement_date = NaiveDate::from_ymd_opt(2023, 05, 03).unwrap();
        let frequency = 2;

        let uk_2025 = build_curve_dates(&maturity_date, &settlement_date, &frequency);

        let predicted = vec![
            NaiveDate::from_ymd_opt(2025, 01, 31).unwrap(),
            NaiveDate::from_ymd_opt(2024, 07, 31).unwrap(),
            NaiveDate::from_ymd_opt(2024, 01, 31).unwrap(),
            NaiveDate::from_ymd_opt(2023, 07, 31).unwrap(),
        ];

        assert_eq!(uk_2025.unwrap(), predicted);
    }

    #[test]
    #[ignore]
    fn test_cashflow_dates_no_val() {
        let maturity_date = NaiveDate::from_ymd_opt(2025, 01, 33).unwrap();
        let settlement_date = NaiveDate::from_ymd_opt(2023, 05, 35).unwrap();
        let frequency = 2;

        let bad_curve = build_curve_dates(&maturity_date, &settlement_date, &frequency);

        println!("{:#?}", bad_curve);
    }
}
