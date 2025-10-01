/* ------------------------------------------------------------------------
    Copyright (C) 2025  Andrew J. Eberhard

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
  -----------------------------------------------------------------------*/
use chrono::NaiveDate;

pub trait LiquidAccount {
    fn get_positive_cash_flow(&self, start: NaiveDate, end: NaiveDate) -> f32;
    fn get_negative_cash_flow(&self, start: NaiveDate, end: NaiveDate) -> f32;
    fn get_cash_flow(&self, start: NaiveDate, end: NaiveDate) -> f32;
}
