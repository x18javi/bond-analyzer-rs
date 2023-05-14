use chrono::{Datelike, Months, NaiveDate};
use std::cell::Cell;
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
    pub frequency: f64,
    pub accrued_fraction: f64,
    pub settlement_date: NaiveDate,
    pub maturity_date: NaiveDate,
    pub cashflow_curve: Vec<NaiveDate>,
    pub ytm: Cell<f64>,
}

#[derive(Error, Debug)]
pub enum BondCalculatorError {
    #[error(
        "invalid daycount value [ {daycount} ]. Has to be one of: nasd30/360, act/act, act360, act365, eur30/360."
    )]
    Daycount { daycount: String },
    #[error("tried to create a bad date with year: {0} month: {1} day: {2}. Cannot continue.")]
    Date(i32, u32, u32),
//    #[error("couldn't access element in cashflows. Cannot continue.")]
//    Curve,
}

impl Bond {
    pub fn new(bond: args::BondCli) -> Result<Self, BondCalculatorError> {
        let coupon = bond.coupon;

        let day_count = DayCountConvention::from_str(&bond.daycount).map_err(|_| {
            BondCalculatorError::Daycount {
                daycount: bond.daycount,
            }
        })?;

        let cashflow_curve =
            build_curve_dates(bond.maturity_date_arg, bond.settlementdate, bond.frequency)?;

        let accrued_fraction = accrued_fraction(
            *cashflow_curve.first().expect("error getting the next coupon date"),
            bond.settlementdate,
            bond.frequency,
            day_count,
        );

        let price = if bond.clean {
            calculate_dirty_price(accrued_fraction, bond.coupon, bond.price, bond.frequency)
        } else {
            bond.price
        };

        Ok(Self {
            coupon,
            price,
            day_count,
            frequency: bond.frequency,
            accrued_fraction,
            settlement_date: bond.settlementdate,
            maturity_date: bond.maturity_date_arg,
            cashflow_curve,
            ytm: Cell::new(0.0),
        })
    }

    fn cashflows(&self) -> impl Iterator<Item = (NaiveDate, f64)> + '_ {
        let coupon_split = self.coupon / self.frequency;

        self.cashflow_curve.iter().map(move |d| {
            if d == &self.maturity_date {
                (*d, coupon_split + 100.0)
            } else {
                (*d, coupon_split)
            }
        })
    }

    fn sum_pv(&self, rate: f64) -> f64 {
        let rate_adj = rate / self.frequency;

        let f = &self.unaccrued_fraction();
        let cashflows_map: BTreeMap<NaiveDate, f64> = self.cashflows().collect();
        let pv = cashflows_map
            .values()
            .enumerate()
            .map(|(i, cf)| cf / (((rate_adj) + 1.0).powf(i as f64 + f)))
            .sum();
        pv
    }

    fn create_yield_to_maturity(&self) {
        self.ytm.set(self.bisection_find(0.0, 2.0));
    }

    fn unaccrued_fraction(&self) -> f64 {
        1.0 - self.accrued_fraction
    }

    fn bisection_find(&self, low: f64, high: f64) -> f64 {
        let mid = (high + low) / 2.0;
        let pv = self.sum_pv(mid);

        if (high - low).abs() > 1e-9 {
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
        let rate_adj = self.ytm.get() / self.frequency;
        let f = &self.unaccrued_fraction();
        let cashflows_map: BTreeMap<NaiveDate, f64> = self.cashflows().collect();
        let pv: f64 = cashflows_map
            .values()
            .enumerate()
            .map(|(i, cf)| cf * ((f + i as f64) / ((rate_adj + 1.0).powf(f + i as f64))))
            .sum();
        (pv / self.price) / self.frequency
    }

    fn modified_duration(&self) -> f64 {
        self.macaulay_duration() / (1.0 + (self.ytm.get()) / self.frequency)
    }

    pub fn analysis_table(&self) -> Table {
        self.create_yield_to_maturity();

        let mut builder = Builder::default();
        builder
            .set_header(["Metric", "Result"])
            .push_record(["YTM", &(round_to_3dp_string(self.ytm.get() * 100.0))])
            .push_record([
                "Macaulay Duration",
                &round_to_3dp_string(self.macaulay_duration()),
            ])
            .push_record([
                "Modified Duration",
                &round_to_3dp_string(self.modified_duration()),
            ]);

        // return built table
        builder.build()
    }

    pub fn cashflows_table(&self) -> Table {
        let mut builder = Builder::default();
        builder.set_header(["Date", "Coupon"]);
        for (d, c) in self.cashflows() {
            builder.push_record([d.to_string(), c.to_string()]);
        }

        // return built table
        builder.build()
    }
}

fn ndays_in_month(year: i32, month: u32) -> Option<u32> {
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1).map(|date| {
        date.pred_opt()
            .unwrap_or_else(|| panic!("Couldn't subtract 1 day from {}-{}-1", y, m))
            .day()
    })
}

fn round_to_3dp(x: f64) -> f64 {
    let x1 = x * 1000.0;
    x1.round() / 1000.0
}

fn round_to_3dp_string(x: f64) -> String {
    format!("{}", round_to_3dp(x))
}

fn accrued_fraction(
    next_coupon: NaiveDate,
    settlement_date: NaiveDate,
    frequency: f64,
    daycount: DayCountConvention,
) -> f64 {
    let frequency_in_months = Months::new(12 / frequency as u32);
    let prev_coupon = next_coupon
        .checked_sub_months(frequency_in_months)
        .unwrap_or_else(|| {
            panic!(
                "could not generate the previous coupon date by subtracting {}months from {}.",
                frequency, next_coupon
            )
        });

    let days_since_last_coupon: f64 = daycount.yearfrac(prev_coupon, settlement_date) * frequency;

    days_since_last_coupon
}

fn calculate_dirty_price(
    days_since_last_coupon: f64,
    coupon: f64,
    price: f64,
    frequency: f64,
) -> f64 {
    let accrued_value = (coupon / frequency) * days_since_last_coupon;
    price + accrued_value
}

fn build_curve_dates(
    maturity_date: NaiveDate,
    settlement_date: NaiveDate,
    frequency: f64,
) -> Result<Vec<NaiveDate>, BondCalculatorError> {
    let mut curve: Vec<NaiveDate> = vec![];
    let mut cf_date: NaiveDate = maturity_date;

    curve.push(cf_date);
    loop {
        cf_date = cf_date
            .checked_sub_months(Months::new(12 / frequency as u32))
            .ok_or(BondCalculatorError::Date(
                cf_date.year(),
                cf_date.month(),
                cf_date.day(),
            ))?;

        if cf_date <= settlement_date {
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
            .ok_or(BondCalculatorError::Date(year, month, day))?;

        curve.push(cf_date);
    }
    // reverse dates to put them in ascending order
    curve.reverse();
    Ok(curve)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_dirty_price() {
        let next_coupon = NaiveDate::from_ymd_opt(2020, 07, 31).unwrap();
        let accrued_fraction = accrued_fraction(
            next_coupon,
            NaiveDate::from_ymd_opt(2020, 02, 20).unwrap(),
            2.0,
            DayCountConvention::ActAct,
        );

        assert!(
            (round_to_3dp(calculate_dirty_price(accrued_fraction, 1.375, 99.8984, 2.0,)) - 99.974)
                .abs()
                < 1e-9
        )
    }

    #[test]
    fn test_round_to_3dp() {
        let val = 1.3961324324;
        assert_eq!(round_to_3dp_string(val), "1.396".to_string())
    }

    #[test]
    fn test_cashflow_dates_gilt() {
        // https://www.dmo.gov.uk/media/qncg02s4/prosp140722.pdf
        let maturity_date = NaiveDate::from_ymd_opt(2025, 01, 31).unwrap();
        let settlement_date = NaiveDate::from_ymd_opt(2023, 05, 03).unwrap();
        let frequency = 2.0;

        let uk_2025 = build_curve_dates(maturity_date, settlement_date, frequency);

        let predicted = vec![
            NaiveDate::from_ymd_opt(2023, 07, 31).unwrap(),
            NaiveDate::from_ymd_opt(2024, 01, 31).unwrap(),
            NaiveDate::from_ymd_opt(2024, 07, 31).unwrap(),
            NaiveDate::from_ymd_opt(2025, 01, 31).unwrap(),
        ];
        assert_eq!(uk_2025.unwrap(), predicted);
    }
}
