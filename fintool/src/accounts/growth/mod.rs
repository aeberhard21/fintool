use crate::accounts::AccountData;
use crate::accounts::base::{HasContext, HasVariableAccountContext, variable_account::VariableValuable, Valuable};
use crate::tui::query_user_for_analysis_period;
use chrono::{Days,Datelike,NaiveDate};
use inquire::Select;
use shared_lib::TransferType;

pub enum GrowthMetric { 
    SimpleReturn,
    CAGR, 
    MWRR, 
    TWRR,
}

pub trait GrowthCalculable: HasContext + Valuable { 
    fn calculate_growth(&self, metric: GrowthMetric, start_date : NaiveDate, end_date : NaiveDate) -> f32;
}

pub fn simple_rate_of_return<T: HasContext + Valuable + ?Sized>(acct: &T, start_date : NaiveDate, end_date : NaiveDate) -> f32 {
    let ev_opt = acct.get_account_value_on_day(&end_date);
    if ev_opt.is_none() {
        return f32::NAN;
    }
    let ev = ev_opt.unwrap();
    let sv_opt = acct.get_account_value_on_day(&start_date);
    if sv_opt.is_none() {
        return f32::NAN;
    }
    let sv = sv_opt.unwrap();
    return (ev-sv)/(sv)*100.;
}

pub fn compound_annual_growth_rate<T: HasContext + Valuable + ?Sized>(acct: &T, start_date : NaiveDate, end_date : NaiveDate) -> f32 {
    let cr = (simple_rate_of_return(acct, start_date, end_date))/100.;
    let days = end_date.num_days_from_ce() - start_date.num_days_from_ce();
    let n = (days as f32) / 365.25;
    return ((1. + cr).powf(1. / n) - 1.) * 100.;
}

pub fn money_weighted_return<T: HasContext + Valuable + ?Sized>(acct: &T, start_date : NaiveDate, end_date : NaiveDate) -> f32 {
    #[derive(Debug)]
    struct CashFlow {
        amount: f32,
        t: f32,
    };

    fn irr(flows: &[CashFlow]) -> Option<f32> {
        let mut low = -0.29999;
        let mut high = 1.; // allow very high return
        let tolerance = 1e-2;

        fn npv(rate: f32, flows: &[CashFlow]) -> f32 {
            flows
                .iter()
                .map(|x| x.amount / (1.0 + rate).powf(x.t))
                .sum()
        }

        if npv(low, flows) * npv(high, flows) > 0.0 {
            return None; // no guaranteed root
        }

        while (high - low) > tolerance {
            let mid = (low + high) / 2.0;
            let value = npv(mid, flows);

            if value > 0.0 {
                low = mid;
            } else {
                high = mid;
            }
        }

        Some((low + high) / 2.0)
    }

    let ctx = acct.ctx();

    let mut cfs: Vec<CashFlow> = Vec::new();

    let day_before = start_date.checked_sub_days(Days::new(1)).unwrap();
    let initial_value = acct.get_account_value_on_day(&day_before).unwrap();
    cfs.push(CashFlow {
        amount: -initial_value,
        t: 0.0,
    });

    let txns = ctx.db
        .get_ledger_entries_within_timestamps(ctx.uid, ctx.aid, start_date, end_date)
        .unwrap();

    for txn in txns {
        let txn_date = NaiveDate::parse_from_str(&txn.info.date, "%Y-%m-%d").unwrap();
        let amount = match txn.info.transfer_type {
            TransferType::DepositFromExternalAccount => -txn.info.amount,
            TransferType::WithdrawalToExternalAccount => txn.info.amount,
            _ => {
                continue;
            }
        };

        let t = (txn_date - start_date).num_days() as f32 / 365.25;
        let cf = CashFlow {
            amount: amount,
            t: t,
        };
        cfs.push(cf);
    }

    let final_value_opt = acct.get_account_value_on_day(&end_date);
    if final_value_opt.is_none() {
        return f32::NAN;
    }
    let final_value = final_value_opt.unwrap();
    let final_t = (end_date - start_date).num_days() as f32 / 365.25;
    cfs.push(CashFlow {
        amount: final_value,
        t: final_t,
    });

    let irr_opt = irr(&cfs);
    if irr_opt.is_none() {
        f32::NAN
    } else {
        irr_opt.unwrap() * 100.
    }
}

pub fn time_weighted_return<T: HasContext + HasVariableAccountContext + VariableValuable + ?Sized>(acct: &T, period_start: NaiveDate, period_end: NaiveDate) -> f32 {
    let mut cf: f32 = 0.0;
    let mut hps: Vec<f32> = Vec::new();
    let mut hp: f32;
    let mut vf;
    let mut vi;
    let mut rate = 0.0;

    let ctx = acct.ctx();

    let starting_fixed_value_opt = ctx
        .db
        .get_cumulative_total_of_ledger_on_date(ctx.uid, ctx.aid, period_start)
        .unwrap();

    let starting_fixed_value;
    if starting_fixed_value_opt.is_some() {
        starting_fixed_value = starting_fixed_value_opt.unwrap();
    } else {
        return f32::NAN;
    }

    let starting_variable_value = acct.positions_value_on_day(
        &period_start
            .checked_sub_days(Days::new(1))
            .expect("Invalid date!"),
    );

    vi = starting_fixed_value + starting_variable_value;

    let external_transactions = Some(
        ctx.db
            .get_ledger_entries_within_timestamps(ctx.uid, ctx.aid, period_start, period_end)
            .unwrap(),
    );
    if let Some(transactions) = external_transactions {
        if !transactions.is_empty() {
            vf = 0.0;
            for txn in transactions {
                let end_period = NaiveDate::parse_from_str(&txn.info.date, "%Y-%m-%d")
                    .expect(format!("Invalid date format: {}", txn.info.date).as_str());
                cf = match txn.info.transfer_type {
                    TransferType::DepositFromExternalAccount => txn.info.amount,
                    TransferType::WithdrawalToExternalAccount => -txn.info.amount,
                    _ => 0.0,
                };
                let final_fixed_value_opt = ctx
                    .db
                    .get_cumulative_total_of_ledger_on_date(ctx.uid, ctx.aid, end_period)
                    .unwrap();
                let final_fixed_value;
                if final_fixed_value_opt.is_some() {
                    final_fixed_value = final_fixed_value_opt.unwrap();
                } else {
                    return f32::NAN;
                }

                let final_variable_value = acct.positions_value_on_day(&end_period);
                vf = final_fixed_value + final_variable_value;
                hp = (vf - (cf + vi)) / (cf + vi);
                hps.push(hp);

                vi = vf;
            }
        }
    }

    let final_fixed_value_opt = ctx
        .db
        .get_cumulative_total_of_ledger_on_date(ctx.uid, ctx.aid, period_end)
        .unwrap();
    let final_fixed_value;
    if final_fixed_value_opt.is_some() {
        final_fixed_value = final_fixed_value_opt.unwrap();
    } else {
        return f32::NAN;
    }

    let final_variable_value = acct.positions_value_on_day(&period_end);
    vf = final_fixed_value + final_variable_value;
    hp = (vf - vi) / vi;
    hps.push(hp);

    let hp1 = hps.pop().expect("No valid cash flow periods!");
    let twr = hps.iter().fold(1.0 + hp1, |acc, hp| acc * (1.0 + hp)) - 1.0;
    rate = twr * 100.0;

    return rate;
}

pub fn report_growth<T: GrowthCalculable + AccountData>(acct: &T) -> Option<f32> {
    const GROWTH_OPTIONS: [&'static str; 5] = [
        "Compound Annual Growth Rate",
        "Money Weighted Rate of Return",
        "Simple Rate of Return",
        "Time Weighted Rate of Return",
        "None",
    ];
    let choice = Select::new("Select growth type to report: ", GROWTH_OPTIONS.to_vec())
        .prompt()
        .unwrap()
        .to_string();
    let growth_metric = match choice.as_str() {
        "Compound Annual Growth Rate" => { GrowthMetric::CAGR }
        "Money Weighted Rate of Return"=> { GrowthMetric::MWRR },
        "Simple Rate of Return"=> { GrowthMetric::SimpleReturn },
        "Time Weighted Rate of Return"=> { GrowthMetric::TWRR },
        "None" => {
            return None;
        }
        _ => {
            panic!("Unrecognized input!");
        }
    };
    let (period_start, period_end, _) =
        query_user_for_analysis_period(acct.get_open_date());
    let rate = acct.calculate_growth(growth_metric, period_start, period_end);
    return Some(rate);
}