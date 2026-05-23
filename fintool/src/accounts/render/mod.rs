use crate::accounts::{Account, AnalysisPeriod, DisplayablePositionStatistics, KEY_COMPOUNDED_ANNUAL_RATE_OF_RETURN, KEY_MONEY_WEIGHTED_RATE_OF_RETURN, KEY_SIMPLE_RATE_OF_RETURN, KEY_TIME_WEIGHTED_RATE_OF_RETURN};
use crate::accounts::base::budget::Budget;
use crate::app::app::{App, BarChartData, DisplayValue, LineChart};
use crate::app::screen::{ledger_table_constraint_len_calculator, positions_table_constraint_len_calculator};
use crate::accounts::{KEY_BARCHART_BUDGET, KEY_BARCHART_EXPENDITURES, KEY_TOTAL_VALUE, KEY_REMAINING_CONTRIBUTION, KEY_CONTRIBUTION_LIMIT, KEY_CREDIT_LINE, KEY_REMAINING_CREDIT, KEY_DAYS_UNTIL_DUE, KEY_STATEMENT_DUE_DATE, KEY_MATURITY_DATE, KEY_DAYS_TO_MATURITY};
use crate::accounts::base::{HasContext, LedgerOps, Valuable};
use crate::types::ledger::{Expenditure, LedgerInfo, LedgerRecord};
use crate::ui::{float_range};
use chrono::{Datelike, Days, Local, NaiveDate, NaiveDateTime, NaiveTime};
use shared_lib::TransferType;
use std::collections::HashMap;
use std::iter::zip;

use ratatui::{
    buffer::Buffer,
    layout::{self, Constraint, Direction, Layout, Rect},
    style::{palette, palette::tailwind, Color, Modifier, Style, Stylize},
    symbols::{self, Marker},
    text::{Line, Span, Text as ratatuiText},
    widgets::{
        Axis, Bar, BarChart, BarGroup, Block, Borders, Cell, Chart, Clear, Dataset, GraphType,
        HighlightSpacing, LegendPosition, List, ListItem, Padding, Paragraph, Row, Table, Tabs, Widget, Wrap,
    },
    Frame,
};

pub fn get_account_value_linechart<T: HasContext + LedgerOps>(acct: &T, app: &mut App) -> Option<LineChart> {
    let ctx = acct.ctx();
    let (mut start, end) = (app.analysis_start, app.analysis_end);
    if start < ctx.open_date {
        start = ctx.open_date;
    }
    let starting_amount_opt = ctx
        .db
        .get_cumulative_total_of_ledger_before_date(ctx.uid, ctx.aid, start)
        .unwrap();
    let mut entries: Vec<LedgerRecord> = if starting_amount_opt.is_some() {
        let starting_amount = starting_amount_opt.unwrap();
        vec![LedgerRecord {
            id: 0,
            info: LedgerInfo {
                date: start.checked_add_days(Days::new(1)).unwrap().to_string(),
                amount: starting_amount,
                transfer_type: TransferType::ZeroSumChange,
                participant: 0,
                category_id: 0,
                description: "initial".to_string(),
            },
        }]
    } else {
        vec![LedgerRecord {
            id: 0,
            info: LedgerInfo {
                date: start.checked_add_days(Days::new(1)).unwrap().to_string(),
                amount: 0.0,
                transfer_type: TransferType::ZeroSumChange,
                participant: 0,
                category_id: 0,
                description: "initial".to_string(),
            },
        }]
    };
    entries.append(&mut acct.get_ledger_entries_between_timestamps(start, end));
    if !(entries.len() == 1) {
        entries.reverse();
        let last = entries.pop().unwrap();

        let mut aggregate: f64 = last.info.amount as f64;
        let starting_date = NaiveDate::parse_from_str(&last.info.date, "%Y-%m-%d").unwrap();
        let mut min_total = aggregate;
        let mut max_total = aggregate;
        let tstamp_min = starting_date
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .and_utc()
            .timestamp_millis() as f64;
        let mut tstamp_max = tstamp_min;

        let data: Vec<(f64, f64)> = entries
            .iter()
            .rev()
            .map(|record| {
                let date = NaiveDate::parse_from_str(&record.info.date, "%Y-%m-%d").unwrap();
                let dt = date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                let tstamp = dt.and_utc().timestamp_millis() as f64;
                aggregate = match record.info.transfer_type {
                    TransferType::DepositFromExternalAccount
                    | TransferType::DepositFromInternalAccount => {
                        aggregate + record.info.amount as f64
                    }
                    TransferType::WithdrawalToExternalAccount
                    | TransferType::WithdrawalToInternalAccount => {
                        aggregate - record.info.amount as f64
                    }
                    TransferType::ZeroSumChange => aggregate,
                };
                max_total = if aggregate > max_total {
                    aggregate
                } else {
                    max_total
                };
                min_total = if aggregate < min_total {
                    aggregate
                } else {
                    min_total
                };
                tstamp_max = if tstamp > tstamp_max {
                    tstamp
                } else {
                    tstamp_max
                };
                (tstamp, aggregate)
            })
            .collect();

        Some(LineChart {
            datasets: vec![data],
            y_max: max_total,
            y_min: min_total,
            y_step: (max_total - min_total) / 5.0,
            x_max: tstamp_max,
            x_min: tstamp_min,
            x_labels: vec![last.info.date, entries[0].info.date.clone()],
            y_labels: float_range(min_total, max_total, (max_total - min_total) / 5.0)
                .into_iter()
                .map(|x| format!("{:.2}", x))
                .collect(),
        })
    } else {
        None
    }
}

pub fn get_time_period_investment_linechart<T: HasContext + LedgerOps + Valuable>(acct: &T, app: &mut App) -> Option<LineChart> {
    let ctx = acct.ctx();
    let (mut start, end) = (app.analysis_start, app.analysis_end);
    if start < ctx.open_date {
        start = ctx.open_date;
    }
    let mut ledger = acct.get_ledger_entries_between_timestamps(start, end);
    ledger.push(LedgerRecord {
        id: 0,
        info: LedgerInfo {
            date: Local::now().date_naive().to_string(),
            amount: 0.0,
            transfer_type: TransferType::ZeroSumChange,
            participant: 0,
            category_id: 0,
            description: "".to_string(),
        },
    });
    let external_transfers = acct
        .get_external_transactions_between_timestamps(start, end);

    let mut tstamp_min = f64::MAX;
    let mut tstamp_max = f64::MIN;
    let mut min_total = f64::MAX;
    let mut max_total = f64::MIN;

    // time period starting amount
    let time_period_investments_opt = if let Some(mut transactions) = external_transfers {
        if !transactions.is_empty() {
            // this has to return a value because it will be inclusive of first entry
            let tpi_starting_amount = ctx.db
                .get_cumulative_total_of_ledger_of_external_transactions_on_date(
                    ctx.uid, ctx.aid, start,
                )
                .unwrap()
                .unwrap();
            let initial = transactions.remove(0);
            let timestamp = NaiveDate::parse_from_str(&initial.info.date, "%Y-%m-%d")
                .expect(format!("Unexpected data: {}", initial.info.date).as_str())
                .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc()
                .timestamp_millis() as f64;
            let mut aggregate = tpi_starting_amount as f64;
            let mut dataset = vec![(timestamp, aggregate)];
            transactions.push(LedgerRecord {
                id: 0,
                info: LedgerInfo {
                    date: Local::now().date_naive().to_string(),
                    amount: 0.0,
                    transfer_type: TransferType::ZeroSumChange,
                    participant: 0,
                    category_id: 0,
                    description: "".to_string(),
                },
            });
            min_total = aggregate;
            max_total = aggregate;
            tstamp_min = timestamp;
            tstamp_max = tstamp_min;

            dataset.append(
                &mut transactions
                    .iter()
                    .map(|record| {
                        let date =
                            NaiveDate::parse_from_str(&record.info.date, "%Y-%m-%d").unwrap();
                        let dt = date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                        let tstamp = dt.and_utc().timestamp_millis() as f64;
                        aggregate = match record.info.transfer_type {
                            TransferType::DepositFromExternalAccount => {
                                aggregate + record.info.amount as f64
                            }
                            TransferType::WithdrawalToExternalAccount => {
                                aggregate - record.info.amount as f64
                            }
                            _ => aggregate,
                        };
                        max_total = if aggregate > max_total {
                            aggregate
                        } else {
                            max_total
                        };
                        min_total = if aggregate < min_total {
                            aggregate
                        } else {
                            min_total
                        };
                        tstamp_max = if tstamp > tstamp_max {
                            tstamp
                        } else {
                            tstamp_max
                        };
                        (tstamp, aggregate)
                    })
                    .collect(),
            );
            Some(dataset)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(time_period_investments) = time_period_investments_opt {
        let mut date = start;
        let today = Local::now().date_naive();
        let ytd_days_elapsed = (today-start).num_days() as u32;
        let mut total_account_values = Vec::new();
        while date < end {
            let value = acct.get_account_value_on_day(&date.clone());
            if value.is_none() {
                break;
            } else {
                let tstamp = NaiveDate::parse_from_str(&date.to_string(), "%Y-%m-%d")
                    .expect(format!("Unexpected data: {}", date).as_str())
                    .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                    .and_utc()
                    .timestamp_millis() as f64;

                let partial_value = acct.get_account_value_on_day(&date);
                let mut aggregate = 0.0;
                if partial_value.is_none() {
                    aggregate = aggregate;
                } else {
                    aggregate = partial_value.unwrap() as f64;
                }
                max_total = if aggregate > max_total {
                    aggregate
                } else {
                    max_total
                };
                min_total = if aggregate < min_total {
                    aggregate
                } else {
                    min_total
                };
                tstamp_max = if tstamp > tstamp_max {
                    tstamp
                } else {
                    tstamp_max
                };
                total_account_values.push((tstamp, aggregate));

                date = match app.analysis_period {
                    AnalysisPeriod::OneDay | AnalysisPeriod::OneWeek => {
                        date.checked_add_days(Days::new(1)).unwrap()
                    }
                    AnalysisPeriod::OneMonth => date.checked_add_days(Days::new(2)).unwrap(),
                    AnalysisPeriod::OneYear
                    | AnalysisPeriod::ThreeMonths
                    | AnalysisPeriod::SixMonths => date.checked_add_days(Days::new(7)).unwrap(),
                    AnalysisPeriod::YTD => {
                        if ytd_days_elapsed > 30 {
                            // if time elapsed greater than a month then only look every 7 days
                            date.checked_add_days(Days::new(7)).unwrap()
                        } else if ytd_days_elapsed > 7 {
                            // if time elapsed greater than a week then only look every 2 days
                            date.checked_add_days(Days::new(2)).unwrap()
                        } else {
                            date.checked_add_days(Days::new(1)).unwrap()
                        }
                    }
                    AnalysisPeriod::TwoYears => date.checked_add_days(Days::new(20)).unwrap(),
                    AnalysisPeriod::FiveYears => date.checked_add_days(Days::new(50)).unwrap(),
                    AnalysisPeriod::TenYears => date.checked_add_days(Days::new(100)).unwrap(),
                    AnalysisPeriod::Custom | AnalysisPeriod::AllTime => {
                        let diff = (end.num_days_from_ce()- start.num_days_from_ce()) as u32;
                        let days_to_add: u32 = if diff <= 365 {
                            1
                        } else if diff <= (365 * 2) {
                            2
                        } else if diff <= (365 * 5) {
                            5
                        } else {
                            10
                        };
                        date.checked_add_days(Days::new(days_to_add as u64))
                            .unwrap()
                    }
                };
            }
        }

        Some(LineChart {
            datasets: vec![time_period_investments, total_account_values],
            y_max: max_total,
            y_min: min_total,
            y_step: (max_total - min_total) / 5.0,
            x_max: tstamp_max,
            x_min: tstamp_min,
            x_labels: vec![start.to_string(), end.to_string()],
            y_labels: float_range(min_total, max_total, (max_total - min_total) / 5.0)
                .into_iter()
                .map(|x| format!("{:.2}", x))
                .collect(),
        })
    } else {
        None
    }
}

pub fn get_budget_barchart_data<T: HasContext + LedgerOps + Account + Budget>(acct: &T, app: &mut App) -> Option<BarChartData> {
    let ctx = acct.ctx();
    if let Some(mut expenditures) = ctx
        .db
        .get_expenditures_between_dates(ctx.uid, ctx.aid, app.analysis_start, app.analysis_end)
        .unwrap()
    {
        let bar_groups = if acct.has_budget() {
            let mut budget = acct.get_budget();
            if budget.is_empty() {
                panic!("No budget found for account '{}'!", ctx.aid);
            }
            let categories = acct.get_budget_categories();
            if categories.is_empty() {
                panic!("No categories found for account '{}'!", ctx.aid);
            }

            // sort expenditures alphabetically
            expenditures.sort_by(|x, y| (x.category).cmp(&y.category));
            // sort budget alphabetically
            budget.sort_by(|x, y| {
                (ctx.db
                    .get_category_name(ctx.uid, ctx.aid, x.item.category_id)
                    .unwrap())
                .cmp(
                    (&ctx.db
                        .get_category_name(ctx.uid, ctx.aid, y.item.category_id)
                        .unwrap()),
                )
            });

            // remove any expenditures that don't map to a budget category, place in to misc category
            let mut misc_expenditures = Expenditure {
                category: "Misc".to_string(),
                amount: 0.0,
            };
            expenditures.retain(|expenditure| {
                if budget
                    .iter()
                    .map(|element| {
                        ctx.db
                            .get_category_name(ctx.uid, ctx.aid, element.item.category_id)
                            .unwrap()
                    })
                    .collect::<Vec<String>>()
                    .binary_search(&expenditure.category)
                    .is_ok()
                {
                    true
                } else {
                    misc_expenditures.amount = misc_expenditures.amount + expenditure.amount;
                    false
                }
            });

            let mut labels: Vec<String> = Vec::new();
            let mut budget_dataset: HashMap<String, (f32, u64)> = HashMap::new();
            let mut expenditure_dataset: HashMap<String, (f32, u64)> = HashMap::new();
            for elem in zip(budget, expenditures) {
                let budget_value = super::base::budget::scale_budget_value_to_analysis_period(
                    elem.0.item.value,
                    app.analysis_start,
                    app.analysis_end,
                );
                let expenditure_value = elem.1.amount;

                labels.push(elem.1.category.clone());
                budget_dataset
                    .insert(elem.1.category.clone(), (budget_value, budget_value as u64));
                expenditure_dataset.insert(
                    elem.1.category.clone(),
                    (expenditure_value, expenditure_value as u64),
                );
            }

            if misc_expenditures.amount > 0.0 {
                let label: String = "Misc".into();
                labels.push(label.clone());
                budget_dataset.insert(label.clone(), (0.0, 0));
                expenditure_dataset.insert(
                    label,
                    (misc_expenditures.amount, misc_expenditures.amount as u64),
                );
            }

            let mut bars: HashMap<String, HashMap<String, (f32, u64)>> = HashMap::new();
            bars.insert(KEY_BARCHART_BUDGET.into(), budget_dataset);
            bars.insert(KEY_BARCHART_EXPENDITURES.into(), expenditure_dataset);

            return Some(BarChartData {
                labels: labels,
                groups: bars,
            });
        } else {
            // group anything less than the top 10 categories into a "miscellaneous" category
            expenditures.sort_by(|x, y| {
                (x.amount)
                    .partial_cmp(&y.amount)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let grouped_others: Option<Expenditure> = if expenditures.len() > 10 {
                let misc = expenditures
                    .drain(10..expenditures.len() - 1)
                    .collect::<Vec<Expenditure>>();
                let amount = misc.into_iter().map(|x| x.amount).sum();
                Some(Expenditure {
                    category: "Misc".to_string(),
                    amount: amount,
                })
            } else {
                None
            };

            if let Some(grouped_others) = grouped_others {
                expenditures.push(grouped_others);
            }

            let mut labels: Vec<String> = Vec::new();
            let mut expenditure_dataset: HashMap<String, (f32, u64)> = HashMap::new();
            for elem in expenditures {
                let expenditure_value = elem.amount;
                labels.push(elem.category.clone());
                expenditure_dataset.insert(
                    elem.category.clone(),
                    (expenditure_value, expenditure_value as u64),
                );
            }

            let mut bars: HashMap<String, HashMap<String, (f32, u64)>> = HashMap::new();
            bars.insert("Expenditures".into(), expenditure_dataset);

            return Some(BarChartData {
                labels: labels,
                groups: bars,
            });
        };
    } else {
        None
    }
}

pub fn render_account_value_linechart(frame: &mut Frame, area: Rect, app: &mut App) {
    let linechart = app.linechart_cache.take();
    if let Some(line_chart) = linechart {
        app.linechart_cache = Some(line_chart.clone());

        let datasets = vec![Dataset::default()
            .name("History")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(tailwind::LIME.c400))
            .graph_type(GraphType::Line)
            .data(&line_chart.datasets[0])];

        let chart = Chart::new(datasets)
            .block(
                Block::bordered()
                    .title(Line::from(" Value Over Time ").cyan().bold().centered())
                    .style(Style::new().bg(tailwind::SLATE.c900)),
            )
            .legend_position(Some(LegendPosition::TopLeft))
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().gray())
                    .bounds([line_chart.x_min, line_chart.x_max])
                    .labels(line_chart.x_labels),
            )
            .y_axis(
                Axis::default()
                    .title("Value (💰)")
                    .style(Style::default().gray())
                    .bounds([line_chart.y_min, line_chart.y_max])
                    .labels(line_chart.y_labels),
            )
            .style(Style::new().bg(tailwind::SLATE.c900));

        frame.render_widget(chart, area);
    } else {
        let value = ratatuiText::styled(
            "No data to display!",
            Style::default().fg(tailwind::ROSE.c400).bold(),
        );

        let display = Paragraph::new(value)
            .centered()
            .alignment(layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Value Over Time")
                    .title_alignment(layout::Alignment::Center)
                    .padding(Padding::new(
                        0,
                        0,
                        (if area.height > 4 {
                            area.height / 2 - 2
                        } else {
                            0
                        }),
                        0,
                    )),
            )
            .bg(tailwind::SLATE.c900);

        frame.render_widget(display, area);
    }
}

pub fn render_time_period_investment_linechart(frame: &mut Frame, area: Rect, app: &mut App) {
    let linechart = app.linechart_cache.take();
    if let Some(line_chart) = linechart {
        app.linechart_cache = Some(line_chart.clone());

        let mut datasets = vec![Dataset::default()
            .name("Time Period Investment")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(tailwind::LIME.c400))
            .graph_type(GraphType::Line)
            .data(&line_chart.datasets[0])];

        datasets.push(
            Dataset::default()
                .name("Total Value")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(tailwind::BLUE.c400))
                .graph_type(GraphType::Line)
                .data(&line_chart.datasets[1]),
        );

        let chart = Chart::new(datasets)
            .block(
                Block::bordered()
                    .title(Line::from(" Value Over Time ").cyan().bold().centered())
                    .style(Style::new().bg(tailwind::SLATE.c900)),
            )
            .legend_position(Some(LegendPosition::TopLeft))
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().gray())
                    .bounds([line_chart.x_min, line_chart.x_max])
                    .labels(line_chart.x_labels),
            )
            .y_axis(
                Axis::default()
                    .title("Value (💰)")
                    .style(Style::default().gray())
                    .bounds([line_chart.y_min, line_chart.y_max])
                    .labels(line_chart.y_labels),
            )
            .style(Style::new().bg(tailwind::SLATE.c900));

        frame.render_widget(chart, area);
    } else {
        let value = ratatuiText::styled(
            "No data to display!",
            Style::default().fg(tailwind::ROSE.c400).bold(),
        );

        let display = Paragraph::new(value)
            .centered()
            .alignment(layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(" Value Over Time ").cyan().bold().centered())
                    .title_alignment(layout::Alignment::Center)
                    .padding(Padding::new(
                        0,
                        0,
                        (if area.height > 4 {
                            area.height / 2 - 2
                        } else {
                            0
                        }),
                        0,
                    )),
            )
            .bg(tailwind::SLATE.c900);

        frame.render_widget(display, area);
    }
}

pub fn render_ledger_table(frame: &mut Frame, area: Rect, app: &mut App) {
    let header_style = Style::default()
        .fg(app.ledger_table_colors.header_fg)
        .bg(app.ledger_table_colors.header_bg);

    let selected_row_style = Style::new()
        .add_modifier(Modifier::REVERSED)
        .fg(app.ledger_table_colors.selected_row_style_fg);

    let header = [
        "ID",
        "Date",
        "Type",
        "Amount",
        "Category",
        "Peer",
        "Description",
        "Labels",
    ]
    .into_iter()
    .map(Cell::from)
    .collect::<Row>()
    .style(header_style)
    .height(1);

    if let Some(ledger) = app.ledger_entries.clone() {
        let data = ledger;

        let rows = data.iter().enumerate().map(|(i, record)| {
            let color = match i % 2 {
                0 => app.ledger_table_colors.normal_row_color,
                _ => app.ledger_table_colors.alt_row_color,
            };
            let item = [
                &record.id.to_string(),
                &record.info.date,
                &record.info.transfer_type,
                &record.info.amount.to_string(),
                &record.info.category,
                &record.info.participant.to_string(),
                &record.info.description,
                &record.info.labels,
            ];
            item.into_iter()
                .map(|content| Cell::from(ratatuiText::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(app.ledger_table_colors.row_fg).bg(color))
                .height(4)
        });

        let bar: &'static str = " █ ";
        let constraint_lens = ledger_table_constraint_len_calculator(&data);
        let t = Table::new(
            rows,
            [
                Constraint::Length(constraint_lens.0 + 1),
                Constraint::Min(constraint_lens.1 + 1),
                Constraint::Min(constraint_lens.2 + 1),
                Constraint::Min(constraint_lens.3 + 1),
                Constraint::Min(constraint_lens.4 + 1),
                Constraint::Min(constraint_lens.5 + 1),
                // don't take more than 25% of screen when display descriptions
                Constraint::Min(area.width / 4),
                Constraint::Min(constraint_lens.7 + 1),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Transactions")
                .title_alignment(layout::Alignment::Center),
        )
        .row_highlight_style(selected_row_style)
        .highlight_symbol(ratatuiText::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .bg(app.ledger_table_colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);

        app.ledger_entries = Some(data);

        frame.render_stateful_widget(t, area, &mut app.ledger_table_state);
    } else {
        let value = ratatuiText::styled(
            "No data to display!",
            Style::default().fg(tailwind::ROSE.c400).bold(),
        );

        let display = Paragraph::new(value)
            .centered()
            .alignment(layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Value Over Time")
                    .title_alignment(layout::Alignment::Center)
                    .padding(Padding::new(
                        0,
                        0,
                        (if area.height > 4 {
                            area.height / 2 - 2
                        } else {
                            0
                        }),
                        0,
                    )),
            )
            .bg(tailwind::SLATE.c900);

        frame.render_widget(display, area);
    }
}

pub fn render_positions_table(frame: &mut Frame, area: Rect, app: &mut App) {

    let block_title = "Positions";

    let header_style = Style::default()
        .fg(app.ledger_table_colors.header_fg)
        .bg(app.ledger_table_colors.header_bg);

    let selected_row_style = Style::new()
        .add_modifier(Modifier::REVERSED)
        .fg(app.ledger_table_colors.selected_row_style_fg);

    let header = [
        DisplayablePositionStatistics::get_ticker_str(),
        DisplayablePositionStatistics::get_quantity_str(),
        DisplayablePositionStatistics::get_value_str(),
        DisplayablePositionStatistics::get_price_str(),
        DisplayablePositionStatistics::get_total_cost_basis_str(),
        DisplayablePositionStatistics::get_unit_cost_str(),
        DisplayablePositionStatistics::get_unrealized_gl_str(),
        DisplayablePositionStatistics::get_unrealized_gl_per_str()
    ]        
    .into_iter()
    .map(Cell::from)
    .collect::<Row>()
    .style(header_style)
    .height(1);

    let position_entries = app.positions_entries.take();
    if let Some(ledger) = position_entries.as_ref() {
        let data = ledger;

        let rows = data.iter().enumerate().map(|(i, record)| {
            let color = match i % 2 {
                0 => app.ledger_table_colors.normal_row_color,
                _ => app.ledger_table_colors.alt_row_color,
            };
            let item = [
                &record.ticker,
                &record.quantity,
                &record.value,
                &record.price,
                &record.total_cost_basis,
                &record.unit_cost,
                &record.unrealized_gl,
                &record.unrealized_gl_per,
            ];
            item.into_iter()
                .enumerate()
                .map(|content| {
                    let index = content.0;
                    let value = content.1;
                    match index {
                        0|1|3|5 => {
                            Cell::from(ratatuiText::from(format!("\n{value}\n")).style(tailwind::WHITE))
                        }
                        _ => {
                            if value.parse::<f32>().unwrap() < 0.0 { 
                                Cell::from(ratatuiText::from(format!("\n{value}\n")).style(tailwind::ROSE.c500))
                            } else { 
                                Cell::from(ratatuiText::from(format!("\n{value}\n")).style(tailwind::EMERALD.c500))
                            }
                        }
                    }
                })
                .collect::<Row>()
                .style(Style::new().fg(app.ledger_table_colors.row_fg).bg(color))
                .height(4)
        });

        let bar: &'static str = " █ ";
        let constraint_lens = positions_table_constraint_len_calculator(&data);
        let t = Table::new(
            rows,
            [
                Constraint::Length(constraint_lens.0 + 1),
                Constraint::Min(constraint_lens.1 + 1),
                Constraint::Min(constraint_lens.2 + 1),
                Constraint::Min(constraint_lens.3 + 1),
                Constraint::Min(constraint_lens.4 + 1),
                Constraint::Min(constraint_lens.5 + 1),
                // don't take more than 25% of screen when display descriptions
                Constraint::Min(area.width / 4),
                Constraint::Min(constraint_lens.7 + 1),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(block_title)
                .title_alignment(layout::Alignment::Center),
        )
        .row_highlight_style(selected_row_style)
        .highlight_symbol(ratatuiText::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .bg(app.ledger_table_colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(t, area, &mut app.ledger_table_state);
    } else {
        let value = ratatuiText::styled(
            "No data to display!",
            Style::default().fg(tailwind::ROSE.c400).bold(),
        );

        let display = Paragraph::new(value)
            .centered()
            .alignment(layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(block_title)
                    .title_alignment(layout::Alignment::Center)
                    .padding(Padding::new(
                        0,
                        0,
                        (if area.height > 4 {
                            area.height / 2 - 2
                        } else {
                            0
                        }),
                        0,
                    )),
            )
            .bg(tailwind::SLATE.c900);

        frame.render_widget(display, area);
    }

    app.positions_entries = position_entries;
}

pub fn render_spend_chart(frame: &mut Frame, area: Rect, app: &mut App) {
    let bar_chart = app.barchart_cache.take();
    if let Some(bar_chart) = bar_chart {
        app.barchart_cache = Some(bar_chart.clone());

        let labels = bar_chart.labels.clone();
        // let datasets = bar_chart.groups.keys().collect::<Vec<String>>();
        let mut bar_groups: Vec<BarGroup<'_>> = Vec::new();
        for label in labels {
            let mut bars: Vec<Bar<'_>> = Vec::new();
            if let Some(budget_dataset) = bar_chart.groups.get(KEY_BARCHART_BUDGET) {
                // budget found
                let budget_value = budget_dataset
                    .get(&label)
                    .expect(format!("Budget group for {} not found!", label).as_str())
                    .clone();
                let budget_bar = Bar::default()
                    .value(budget_value.1)
                    .text_value(format!("${:.2}", budget_value.0))
                    .style(Style::new().fg(tailwind::WHITE))
                    .value_style(Style::new().fg(tailwind::WHITE).reversed());
                bars.push(budget_bar);
            }
            if let Some(expenditure_dataset) = bar_chart.groups.get(KEY_BARCHART_EXPENDITURES) {
                let expenditure_value = expenditure_dataset
                    .get(&label)
                    .expect(format!("Expenditure group for {} not found!", label).as_str())
                    .clone();
                let expenditure_bar = Bar::default()
                    .value(expenditure_value.1)
                    .text_value(format!("${:.2}", expenditure_value.0))
                    .style(Style::new().fg(tailwind::AMBER.c500))
                    .value_style(Style::new().fg(tailwind::AMBER.c500).reversed());
                bars.push(expenditure_bar);
            }
            let group = BarGroup::default()
                .bars(&bars)
                .label(Line::from(label).centered());
            bar_groups.push(group);
        }

        let mut chart = BarChart::default()
            .style(Style::new().bg(tailwind::SLATE.c900))
            .block(Block::bordered().title_top(Line::from("Spend Analyzer").centered()))
            .bar_width(10)
            .group_gap(area.width / (bar_groups.len() as u16 + 10));
        for group in bar_groups {
            chart = chart.data(group);
        }

        frame.render_widget(chart, area);

        // app.barchart_cache = Some(bar_chart);
    } else {
        let value = ratatuiText::styled(
            "No data to display!",
            Style::default().fg(tailwind::ROSE.c400).bold(),
        );

        let display = Paragraph::new(value)
            .centered()
            .alignment(layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Spend Analyzer")
                    .title_alignment(layout::Alignment::Center)
                    .padding(Padding::new(
                        0,
                        0,
                        (if area.height > 4 {
                            area.height / 2 - 2
                        } else {
                            0
                        }),
                        0,
                    )),
            )
            .bg(tailwind::SLATE.c900);

        frame.render_widget(display, area);
    }
}

pub fn render_current_value(frame: &mut Frame, area: Rect, app: &mut App) {
    let current_value = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_TOTAL_VALUE)
        .and_then(DisplayValue::as_f32)
        .expect("Could not find total value!");

    let value = ratatuiText::styled(
        current_value.to_string(),
        Style::default().fg(tailwind::EMERALD.c400).bold(),
    );

    let display = Paragraph::new(value)
        .centered()
        .alignment(layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Current Balance")
                .title_alignment(layout::Alignment::Center)
                .padding(Padding::new(
                    0,
                    0,
                    (if area.height > 4 {
                        area.height / 2 - 2
                    } else {
                        0
                    }),
                    0,
                )),
        )
        .bg(tailwind::SLATE.c900);

    frame.render_widget(display, area);
}

pub fn render_simple_growth(frame: &mut Frame, area: Rect, app: &mut App){
    let value = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_SIMPLE_RATE_OF_RETURN)
        .and_then(DisplayValue::as_f32)
        .expect("Could not find growth!");

    let fg_color = if value < 0.0 {
        tailwind::ROSE.c200
    } else {
        tailwind::EMERALD.c400
    };
    let value = ratatuiText::styled(
        format!("{:.2}%", value).to_string(),
        Style::default().fg(fg_color).bold(),
    );

    let display = Paragraph::new(value)
        .centered()
        .alignment(layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Growth - {} ", app.analysis_period))
                .title_alignment(layout::Alignment::Center)
                .padding(Padding::new(
                    0,
                    0,
                    (if area.height > 4 {
                        area.height / 2 - 2
                    } else {
                        0
                    }),
                    0,
                )),
        )
        .bg(tailwind::SLATE.c900);
    frame.render_widget(display, area);
}

pub fn render_time_weighted_rate_of_return(frame: &mut Frame, area: Rect, app: &mut App) {
    let value = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_TIME_WEIGHTED_RATE_OF_RETURN)
        .and_then(DisplayValue::as_f32)
        .expect("Could not find growth rate!")
        .clone();

    let fg_color = if value < 0.0 {
        tailwind::ROSE.c200
    } else {
        tailwind::EMERALD.c400
    };
    let value = ratatuiText::styled(
        format!("{:.2}%", value).to_string(),
        Style::default().fg(fg_color).bold(),
    );

    let display = Paragraph::new(value)
        .centered()
        .alignment(layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" TWRR - {} ", app.analysis_period))
                .title_alignment(layout::Alignment::Center)
                .padding(Padding::new(
                    0,
                    0,
                    (if area.height > 4 {
                        area.height / 2 - 2
                    } else {
                        0
                    }),
                    0,
                )),
        )
        .bg(tailwind::SLATE.c900);
    frame.render_widget(display, area);
}

pub fn render_annualized_rate_of_return(frame: &mut Frame, area: Rect, app: &mut App) {
    let value = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_COMPOUNDED_ANNUAL_RATE_OF_RETURN)
        .and_then(DisplayValue::as_f32)
        .expect("Could not find growth rate!")
        .clone();

    let fg_color = if value < 0.0 {
        tailwind::ROSE.c200
    } else {
        tailwind::EMERALD.c400
    };
    let value = ratatuiText::styled(
        format!("{:.2}%", value).to_string(),
        Style::default().fg(fg_color).bold(),
    );

    let display = Paragraph::new(value)
        .centered()
        .alignment(layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" CAGR - {} ", app.analysis_period))
                .title_alignment(layout::Alignment::Center)
                .padding(Padding::new(
                    0,
                    0,
                    (if area.height > 4 {
                        area.height / 2 - 2
                    } else {
                        0
                    }),
                    0,
                )),
        )
        .bg(tailwind::SLATE.c900);
    frame.render_widget(display, area);
}

pub fn render_money_weighted_rate_of_return(frame: &mut Frame, area: Rect, app: &mut App) {
    let value = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_MONEY_WEIGHTED_RATE_OF_RETURN)
        .and_then(DisplayValue::as_f32)
        .expect("Could not find growth rate!")
        .clone();

    let fg_color = if value < 0.0 {
        tailwind::ROSE.c200
    } else {
        tailwind::EMERALD.c400
    };
    let value = ratatuiText::styled(
        format!("{:.2}%", value).to_string(),
        Style::default().fg(fg_color).bold(),
    );

    let display = Paragraph::new(value)
        .centered()
        .alignment(layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" MWRR - {} ", app.analysis_period))
                .title_alignment(layout::Alignment::Center)
                .padding(Padding::new(
                    0,
                    0,
                    (if area.height > 4 {
                        area.height / 2 - 2
                    } else {
                        0
                    }),
                    0,
                )),
        )
        .bg(tailwind::SLATE.c900);
    frame.render_widget(display, area);
}

pub fn render_remaining_contribution(frame: &mut Frame, area: Rect, app: &App) {
    let contribution_remaining = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_REMAINING_CONTRIBUTION)
        .and_then(DisplayValue::as_f32)
        .expect("Could not find remaining contribution!")
        .clone();
    let contribution_limit = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_CONTRIBUTION_LIMIT)
        .and_then(DisplayValue::as_f32)
        .expect("Could not find contribution limit!")
        .clone();

    let remaining_contribution_text = vec![
        Span::styled(
            format!("${:.2}", contribution_remaining),
            Style::default().bold().fg(if contribution_limit < 500. {
                tailwind::EMERALD.c400
            } else if contribution_remaining < 1500. {
                tailwind::ROSE.c200
            } else {
                tailwind::ROSE.c100
            }),
        ),
        Span::styled(
            format!(" of ${:.2} remaining.", contribution_limit),
            Style::default().bold().fg(tailwind::EMERALD.c400),
        ),
    ];

    let line = Line::from(remaining_contribution_text);
    let text = ratatuiText::from(line);
    let p = Paragraph::new(text)
        .centered()
        .alignment(layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Remaining Contribution")
                .title_alignment(layout::Alignment::Center)
                .padding(Padding::new(
                    0,
                    0,
                    (if area.height > 4 {
                        area.height / 2 - 2
                    } else {
                        0
                    }),
                    0,
                )),
        )
        .bg(tailwind::SLATE.c900);
    frame.render_widget(p, area);
}

pub fn render_days_until_due_date(frame: &mut Frame, area: Rect, app: &App) {
    let days_to = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_DAYS_UNTIL_DUE)
        .and_then(DisplayValue::as_uint)
        .expect("Could not find days until due date!");
    let statement_date = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_STATEMENT_DUE_DATE)
        .and_then(DisplayValue::as_text)
        .expect("Could not find statement due date!");

    let days_to_text = vec![
        Span::styled(
            format!("{} {}", days_to, if days_to > 1 { "days" } else { "day" }),
            Style::default().bold().fg(if days_to < 5 {
                tailwind::ROSE.c100
            } else if days_to < 15 {
                tailwind::ROSE.c200
            } else {
                tailwind::EMERALD.c400
            }),
        ),
        Span::styled(
            format!(" until {}", statement_date),
            Style::default().bold().fg(tailwind::EMERALD.c400),
        ),
    ];
    let line = Line::from(days_to_text);
    let text = ratatuiText::from(line);
    let p = Paragraph::new(text)
        .centered()
        .alignment(layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Statement Due Date Countdown")
                .title_alignment(layout::Alignment::Center)
                .padding(Padding::new(
                    0,
                    0,
                    (if area.height > 4 {
                        area.height / 2 - 2
                    } else {
                        0
                    }),
                    0,
                )),
        )
        .bg(tailwind::SLATE.c900);
    frame.render_widget(p, area);
}

pub fn render_remaining_credit(frame: &mut Frame, area: Rect, app: &App) {
    let credit_remaining = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_REMAINING_CREDIT)
        .and_then(DisplayValue::as_f32)
        .expect("Could not find remaining credit!");
    let credit_line = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_CREDIT_LINE)
        .and_then(DisplayValue::as_f32)
        .expect("Could not find credit line!");

    let credit_remaining_text = vec![
        Span::styled(
            format!("${:.2}", credit_remaining),
            Style::default().bold().fg(if credit_remaining < 500. {
                tailwind::ROSE.c100
            } else if credit_remaining < 100. {
                tailwind::ROSE.c200
            } else {
                tailwind::EMERALD.c400
            }),
        ),
        Span::styled(
            format!(" of ${:.2} remaining.", credit_line),
            Style::default().bold().fg(tailwind::EMERALD.c400),
        ),
    ];
    let line = Line::from(credit_remaining_text);
    let text = ratatuiText::from(line);
    let p = Paragraph::new(text)
        .centered()
        .alignment(layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Remaining Credit")
                .title_alignment(layout::Alignment::Center)
                .padding(Padding::new(
                    0,
                    0,
                    (if area.height > 4 {
                        area.height / 2 - 2
                    } else {
                        0
                    }),
                    0,
                )),
        )
        .bg(tailwind::SLATE.c900);
    frame.render_widget(p, area);
}

pub fn render_days_to_maturity(frame: &mut Frame, area: Rect, app: &mut App) {
    let maturity_date = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_MATURITY_DATE)
        .and_then(DisplayValue::as_text)
        .expect("Could not find maturity date!");

    let days_to = app
        .page_cache_f32
        .as_ref()
        .expect("Account's page has not been cached!")
        .get(KEY_DAYS_TO_MATURITY)
        .and_then(DisplayValue::as_uint)
        .expect("Could not find days to maturity!");

    let days_to_text = vec![
        Span::styled(
            format!("{} days", days_to),
            Style::default().bold().fg(if days_to < 10 {
                tailwind::ROSE.c100
            } else if days_to < 30 {
                tailwind::ROSE.c200
            } else {
                tailwind::EMERALD.c400
            }),
        ),
        Span::styled(
            format!(" to {}", maturity_date),
            Style::default().bold().fg(tailwind::EMERALD.c400),
        ),
    ];
    let line = Line::from(days_to_text);
    let text = ratatuiText::from(line);
    let p = Paragraph::new(text)
        .centered()
        .alignment(layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Maturity Date Countdown")
                .title_alignment(layout::Alignment::Center)
                .padding(Padding::new(
                    0,
                    0,
                    (if area.height > 4 {
                        area.height / 2 - 2
                    } else {
                        0
                    }),
                    0,
                )),
        )
        .bg(tailwind::SLATE.c900);
    frame.render_widget(p, area);
}



