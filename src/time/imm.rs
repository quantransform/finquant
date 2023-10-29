use chrono::{Datelike, NaiveDate, Weekday};
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

/// IMM Month Codes.
/// https://www.cmegroup.com/month-codes.html
#[repr(u16)]
#[derive(EnumIter, EnumString, Display, PartialEq, Debug)]
pub enum IMMMonth {
    F = 1,
    G = 2,
    H = 3,
    J = 4,
    K = 5,
    M = 6,
    N = 7,
    Q = 8,
    U = 9,
    V = 10,
    X = 11,
    Z = 12,
}

/// IMM Related.
pub struct IMM;

impl IMM {
    /// Check is IMM Date.
    /// https://en.wikipedia.org/wiki/IMM_dates
    /// The IMM dates are the four quarterly dates of each year as scheduled maturity date.
    /// The dates are the third Wednesday of March, June, September and December
    /// (i.e., between the 15th and 21st, whichever such day is a Wednesday).
    pub fn is_imm_date(&self, date: NaiveDate, main_cycle: bool) -> bool {
        if date.weekday() != Weekday::Wed {
            return false;
        }

        let d = date.day();
        if !(15..=21).contains(&d) {
            return false;
        }

        if !main_cycle {
            return true;
        }

        matches!(date.month(), 3 | 6 | 9 | 12)
    }

    /// IMM Codes are constructed by IMMMonth + year.
    pub fn is_imm_code(&self, imm_code: &str, main_cycle: bool) -> bool {
        if imm_code.len() != 2 {
            return false;
        }

        let imm_year = imm_code.chars().nth(1).unwrap();

        if !"0123456789".contains(imm_year) {
            return false;
        }

        let str = if main_cycle {
            "hmzuHMZU"
        } else {
            "fghjkmnquvxzFGHJKMNQUVXZ"
        };

        let imm_month = imm_code.chars().nth(0).unwrap();

        if !str.contains(imm_month) {
            return false;
        }

        true
    }

    /// Convert a valid date to IMM code.
    pub fn code(&self, date: NaiveDate) -> Option<String> {
        if !self.is_imm_date(date, false) {
            None
        } else {
            let y = date.year() % 10;
            let mut month = IMMMonth::iter()
                .nth((date.month() - 1) as usize)
                .unwrap()
                .to_string();
            month.push_str(y.to_string().as_str());
            Some(month)
        }
    }

    /// IMM Code to maturity date.
    pub fn date(&self, imm_code: &str, ref_date: Option<NaiveDate>) -> Option<NaiveDate> {
        if !self.is_imm_code(imm_code, false) {
            None
        } else {
            let ref_date = ref_date.unwrap_or(chrono::offset::Utc::now().date_naive());
            let month = imm_code.chars().nth(0).unwrap();
            let mut year = imm_code.chars().nth(1).unwrap().to_digit(10).unwrap() as i32;
            let imm_month = IMMMonth::from_str(&month.to_string()).unwrap() as u32;
            if year == 0 && ref_date.year() <= 1909 {
                year += 10
            }
            let ref_year = ref_date.year() % 10;
            year += ref_date.year() - ref_year;
            let result =
                self.next_date(NaiveDate::from_ymd_opt(year, imm_month, 1).unwrap(), false);
            if result < ref_date {
                Some(self.next_date(
                    NaiveDate::from_ymd_opt(year + 10, imm_month, 1).unwrap(),
                    false,
                ))
            } else {
                Some(result)
            }
        }
    }

    /// Next date.
    pub fn next_date(&self, date: NaiveDate, main_cycle: bool) -> NaiveDate {
        let mut month = date.month();
        let mut year = date.year();
        let offset = if main_cycle { 3 } else { 1 };
        let mut skip_months = offset - (date.month() % offset);
        if skip_months != offset || date.day() > 21 {
            skip_months += date.month();
            if skip_months > 12 {
                month -= 12;
                year += 1;
            }
        }
        let mut result = self.nth_weekday(3, Weekday::Wed, month, year).unwrap();
        if result <= date {
            result = self.next_date(
                NaiveDate::from_ymd_opt(year, month, 22).unwrap(),
                main_cycle,
            );
        }
        result
    }

    fn nth_weekday(&self, nth: i32, day_of_week: Weekday, m: u32, y: i32) -> Option<NaiveDate> {
        if !(0..=6).contains(&nth) {
            None
        } else {
            let first = NaiveDate::from_ymd_opt(y, m, 1).unwrap().weekday();
            let skip = nth
                - (if day_of_week.num_days_from_monday() >= first.num_days_from_monday() {
                    1
                } else {
                    0
                });
            NaiveDate::from_ymd_opt(
                y,
                m,
                1 + day_of_week.num_days_from_monday() + skip as u32 * 7
                    - first.num_days_from_monday(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IMMMonth, IMM};
    use chrono::NaiveDate;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn test_imm_month() {
        assert_eq!(IMMMonth::iter().nth(5).unwrap().to_string(), "M");
        assert_eq!(IMMMonth::from_str("F").unwrap() as u16, 1);
    }
    #[test]
    fn test_imm_code() {
        assert_eq!(IMM.is_imm_code("more_than_2", false), false);
        assert_eq!(IMM.is_imm_code("1", false), false);
        assert_eq!(IMM.is_imm_code("", false), false);
        assert_eq!(IMM.is_imm_code("1F", false), false);
        assert_eq!(IMM.is_imm_code("F1", true), false);
        assert_eq!(IMM.is_imm_code("F1", false), true);
    }

    #[test]
    fn test_generate_code() {
        assert_eq!(
            IMM.code(NaiveDate::from_ymd_opt(2023, 9, 20).unwrap()),
            Some(String::from("U3"))
        );
    }

    #[test]
    fn test_imm_code_to_date() {
        assert_eq!(
            IMM.date("X3", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2023, 11, 15)
        );
        assert_eq!(
            IMM.date("Z3", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2023, 12, 20)
        );
        assert_eq!(
            IMM.date("F4", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2024, 1, 17)
        );
        assert_eq!(
            IMM.date("G4", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2024, 2, 21)
        );
        assert_eq!(
            IMM.date("H4", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2024, 3, 20)
        );
        assert_eq!(
            IMM.date("J4", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2024, 4, 17)
        );
        assert_eq!(
            IMM.date("M4", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2024, 6, 19)
        );
        assert_eq!(
            IMM.date("U4", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2024, 9, 18)
        );
        assert_eq!(
            IMM.date("Z4", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2024, 12, 18)
        );
        assert_eq!(
            IMM.date("H5", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2025, 3, 19)
        );
        assert_eq!(
            IMM.date("M5", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2025, 6, 18)
        );
        assert_eq!(
            IMM.date("U5", NaiveDate::from_ymd_opt(2023, 10, 29)),
            NaiveDate::from_ymd_opt(2025, 9, 17)
        );
    }
}
