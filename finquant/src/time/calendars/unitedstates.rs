// Holidays in United States.
use crate::time::calendars::Calendar;
use chrono::{NaiveDate, Weekday};

pub enum UnitedStatesMarket {
    Settlement,
    Libor,
    NYSE,
    GovernmentBond,
    SOFR,
    NERC,
    FederalReserve,
    None,
}

pub struct UnitedStates {
    market: UnitedStatesMarket,
}

impl UnitedStates {
    fn is_washington_birthday(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);
        ((y >= 1971) && ((15..=21).contains(&d) && w == Weekday::Mon && m == 2))
            || ((y < 1971)
                && (d == 22 || (d == 23 && w == Weekday::Mon) || (d == 21 && w == Weekday::Fri))
                && (m == 2))
    }

    fn is_memorial_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);
        ((y >= 1971) && (d >= 25 && w == Weekday::Mon && m == 5))
            || ((y < 1971)
                && ((d == 30 || (d == 31 && w == Weekday::Mon) || (d == 29 && w == Weekday::Fri))
                    && m == 5))
    }

    fn is_labor_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, _y, _) = self.naive_date_to_dkmy(date);
        d <= 7 && w == Weekday::Mon && m == 9
    }

    fn is_columbus_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);
        (8..=14).contains(&d) && w == Weekday::Mon && m == 10 && y >= 1971
    }

    fn is_veterans_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);
        // 11 11th, adjusted (before 1970, after 1978)
        // Fourth Monday in 10 (1970 - 1978)
        ((y <= 1970 || y >= 1978)
            && ((d == 11 || (d == 12 && w == Weekday::Mon) || (d == 10 && w == Weekday::Fri))
                && m == 11))
            || ((y > 1970 && y < 1978) && ((22..=28).contains(&d) && w == Weekday::Mon && m == 10))
    }

    fn is_veterans_day_no_saturday(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);
        // 11 11th, adjusted, but no Saturday to Friday (before 1970, after 1978)
        // Fourth Monday in 10 (1970 - 1978)
        ((y <= 1970 || y >= 1978) && ((d == 11 || (d == 12 && w == Weekday::Mon)) && m == 11))
            || ((y > 1970 && y < 1978) && ((22..=28).contains(&d) && w == Weekday::Mon && m == 10))
    }

    fn is_juneteenth(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);
        (d == 19 || (d == 20 && w == Weekday::Mon) || (d == 18 && w == Weekday::Fri))
            && m == 6
            && y >= 2022
    }

    fn settlement_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);
        if self.is_weekend(date)
            // New Year's Day (possibly moved to Monday if on Sunday)
            || ((d == 1 || (d == 2 && w == Weekday::Mon)) && m == 1)
            // (or to Friday if on Saturday)
            || (d == 31 && w == Weekday::Fri && m == 12)
            // Martin Luther King's birthday (third Monday in 1)
            || ((15..=21).contains(&d) && w == Weekday::Mon && m == 1 && y >= 1983)
            // Washington's birthday (third Monday in 2)
            || self.is_washington_birthday(date)
            // Memorial Day (last Monday in May)
            || self.is_memorial_day(date)
            // 6teenth (Monday if Sunday or Friday if Saturday)
            || self.is_juneteenth(date)
            // Independence Day (Monday if Sunday or Friday if Saturday)
            || ((d == 4 || (d == 5 && w == Weekday::Mon) || (d == 3 && w == Weekday::Fri)) && m == 7)
            // Labor Day (first Monday in 9)
            || self.is_labor_day(date)
            // Columbus Day (second Monday in 10)
            || self.is_columbus_day(date)
            // Veteran's Day (Monday if Sunday or Friday if Saturday)
            || self.is_veterans_day(date)
            // Thanksgiving Day (fourth Thursday in 11)
            || ((22..=28).contains(&d) && w == Weekday::Thu && m == 11)
            // Christmas (Monday if Sunday or Friday if Saturday)
            || ((d == 25 || (d == 26 && w == Weekday::Mon) ||
            (d == 24 && w == Weekday::Fri)) && m == 12)
        {
            false
        } else {
            true
        }
    }

    fn libor_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _) = self.naive_date_to_dkmy(date);
        if ((d == 5 && w == Weekday::Mon) || (d == 3 && w == Weekday::Fri)) && m == 7 && y >= 2015 {
            return true;
        }
        self.settlement_is_business_day(date)
    }

    fn nyse_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day (possibly moved to Monday if on Sunday)
            || ((d == 1 || (d == 2 && w == Weekday::Mon)) && m == 1)
            // Washington's birthday (third Monday in 2)
            || self.is_washington_birthday(date)
            // Good Friday
            || (dd == em-3)
            // Memorial Day (last Monday in May)
            || self.is_memorial_day(date)
            // 6teenth (Monday if Sunday or Friday if Saturday)
            || self.is_juneteenth(date)
            // Independence Day (Monday if Sunday or Friday if Saturday)
            || ((d == 4 || (d == 5 && w == Weekday::Mon) || (d == 3 && w == Weekday::Fri)) && m == 7)
            // Labor Day (first Monday in 9)
            || self.is_labor_day(date)
            // Thanksgiving Day (fourth Thursday in November)
            || ((22..=28).contains(&d) && w == Weekday::Thu && m == 11)
            // Christmas (Monday if Sunday or Friday if Saturday)
            || ((d == 25 || (d == 26 && w == Weekday::Mon) || (d == 24 && w == Weekday::Fri)) && m == 12)
        {
            return false;
        }

        // Martin Luther King's birthday (third Monday in 1)
        if y >= 1998 && (15..=21).contains(&d) && w == Weekday::Mon && m == 1 {
            return false;
        }

        // Presidential election days
        if (y <= 1968 || (y <= 1980 && y % 4 == 0)) && m == 11 && d <= 7 && w == Weekday::Tue {
            return false;
        }

        // Special closings
        if
        // President Bush's Funeral
        (y == 2018 && m == 12 && d == 5)
                // Hurricane Sandy
                || (y == 2012 && m == 10 && (d == 29 || d == 30))
                // President Ford's funeral
                || (y == 2007 && m == 1 && d == 2)
                // President Reagan's funeral
                || (y == 2004 && m == 6 && d == 11)
                // 9 11-14, 2001
                || (y == 2001 && m == 9 && (11..=14).contains(&d))
                // President Nixon's funeral
                || (y == 1994 && m == 4 && d == 27)
                // Hurricane Gloria
                || (y == 1985 && m == 9 && d == 27)
                // 1977 Blackout
                || (y == 1977 && m == 7 && d == 14)
                // Funeral of former President Lyndon B. Johnson.
                || (y == 1973 && m == 1 && d == 25)
                // Funeral of former President Harry S. Truman
                || (y == 1972 && m == 12 && d == 28)
                // National Day of Participation for the lunar exploration.
                || (y == 1969 && m == 7 && d == 21)
                // Funeral of former President Eisenhower.
                || (y == 1969 && m == 3 && d == 31)
                // Closed all day - heavy snow.
                || (y == 1969 && m == 2 && d == 10)
                // Day after Independence Day.
                || (y == 1968 && m == 7 && d == 5)
                // 6 12-Dec. 31, 1968
                // Four day week (closed on Wednesdays) - Paperwork Crisis
                || (y == 1968 && dd >= 163 && w == Weekday::Wed)
                // Day of mourning for Martin Luther King Jr.
                || (y == 1968 && m == 4 && d == 9)
                // Funeral of President Kennedy
                || (y == 1963 && m == 11 && d == 25)
                // Day before Decoration Day
                || (y == 1961 && m == 5 && d == 29)
                // Day after Christmas
                || (y == 1958 && m == 12 && d == 26)
                // Christmas Eve
                || ((y == 1954 || y == 1956 || y == 1965)
                && m == 12 && d == 24)
        {
            return false;
        }

        true
    }

    fn government_bond_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, dd) = self.naive_date_to_dkmy(date);
        let em = self.easter_monday(y);

        if self.is_weekend(date)
            // New Year's Day (possibly moved to Monday if on Sunday)
            || ((d == 1 || (d == 2 && w == Weekday::Mon)) && m == 1)
            // Martin Luther King's birthday (third Monday in January)
            || ((15..=21).contains(&d) && w == Weekday::Mon && m == 1 && y >= 1983)
            // Washington's birthday (third Monday in February)
            || self.is_washington_birthday(date)
            // Good Friday (2015, 2021, 2023 are half day due to NFP/SIFMA;
            // see <https://www.sifma.org/resources/general/holiday-schedule/>)
            || (dd == em-3 && y != 2015 && y != 2021 && y != 2023)
            // Memorial Day (last Monday in May)
            || self.is_memorial_day(date)
            // Juneteenth (Monday if Sunday or Friday if Saturday)
            || self.is_juneteenth(date)
            // Independence Day (Monday if Sunday or Friday if Saturday)
            || ((d == 4 || (d == 5 && w == Weekday::Mon) || (d == 3 && w == Weekday::Fri)) && m == 7)
            // Labor Day (first Monday in September)
            || self.is_labor_day(date)
            // Columbus Day (second Monday in October)
            || self.is_columbus_day(date)
            // Veteran's Day (Monday if Sunday)
            || self.is_veterans_day_no_saturday(date)
            // Thanksgiving Day (fourth Thursday in November)
            || ((22..=28).contains(&d) && w == Weekday::Thu && m == 11)
            // Christmas (Monday if Sunday or Friday if Saturday)
            || ((d == 25 || (d == 26 && w == Weekday::Mon) || (d == 24 && w == Weekday::Fri)) && m == 12)
        {
            return false;
        }

        // Special closings
        if
        // President Bush's Funeral
        (y == 2018 && m == 12 && d == 5)
                // Hurricane Sandy
                || (y == 2012 && m == 10 && (d == 30))
                // President Reagan's funeral
                || (y == 2004 && m == 6 && d == 11)
        {
            return false;
        }

        true
    }

    fn sofr_is_business_day(&self, date: NaiveDate) -> bool {
        // Good Friday 2023 was only a half close for SIFMA but SOFR didn't fix
        if date == NaiveDate::from_ymd_opt(2023, 4, 7).unwrap() {
            false
        } else {
            self.government_bond_is_business_day(date)
        }
    }

    fn nerc_is_business_day(&self, date: NaiveDate) -> bool {
        let (d, w, m, _, _) = self.naive_date_to_dkmy(date);
        if self.is_weekend(date)
            // New Year's Day (possibly moved to Monday if on Sunday)
            || ((d == 1 || (d == 2 && w == Weekday::Mon)) && m == 1)
            // Memorial Day (last Monday in May)
            || self.is_memorial_day(date)
            // Independence Day (Monday if Sunday)
            || ((d == 4 || (d == 5 && w == Weekday::Mon)) && m == 7)
            // Labor Day (first Monday in September)
            || self.is_labor_day(date)
            // Thanksgiving Day (fourth Thursday in November)
            || ((22..=28).contains(&d) && w == Weekday::Thu && m == 11)
            // Christmas (Monday if Sunday)
            || ((d == 25 || (d == 26 && w == Weekday::Mon)) && m == 12)
        {
            false
        } else {
            true
        }
    }

    fn federal_reserve_is_holiday(&self, date: NaiveDate) -> bool {
        let (d, w, m, y, _dd) = self.naive_date_to_dkmy(date);

        if self.is_weekend(date)
            // New Year's Day (possibly moved to Monday if on Sunday)
            || ((d == 1 || (d == 2 && w == Weekday::Mon)) && m == 1)
            // Martin Luther King's birthday (third Monday in January)
            || ((15..=21).contains(&d) && w == Weekday::Mon && m == 1 && y >= 1983)
            // Washington's birthday (third Monday in February)
            || self.is_washington_birthday(date)
            // Memorial Day (last Monday in May)
            || self.is_memorial_day(date)
            // Juneteenth (Monday if Sunday or Friday if Saturday)
            || self.is_juneteenth(date)
            // Independence Day (Monday if Sunday)
            || ((d == 4 || (d == 5 && w == Weekday::Mon)) && m == 7)
            // Labor Day (first Monday in September)
            || self.is_labor_day(date)
            // Columbus Day (second Monday in October)
            || self.is_columbus_day(date)
            // Veteran's Day (Monday if Sunday)
            || self.is_veterans_day_no_saturday(date)
            // Thanksgiving Day (fourth Thursday in November)
            || ((22..=28).contains(&d) && w == Weekday::Thu && m == 11)
            // Christmas (Monday if Sunday)
            || ((d == 25 || (d == 26 && w == Weekday::Mon)) && m == 12)
        {
            false
        } else {
            true
        }
    }
}

impl Calendar for UnitedStates {
    fn is_business_day(&self, date: NaiveDate) -> bool {
        match self.market {
            UnitedStatesMarket::Settlement => self.settlement_is_business_day(date),
            UnitedStatesMarket::Libor => self.libor_is_business_day(date),
            UnitedStatesMarket::NYSE => self.nyse_is_business_day(date),
            UnitedStatesMarket::GovernmentBond => self.government_bond_is_business_day(date),
            UnitedStatesMarket::SOFR => self.sofr_is_business_day(date),
            UnitedStatesMarket::NERC => self.nerc_is_business_day(date),
            UnitedStatesMarket::FederalReserve => self.federal_reserve_is_holiday(date),
            UnitedStatesMarket::None => self.settlement_is_business_day(date),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Calendar;
    use super::UnitedStates;
    use super::UnitedStatesMarket;
    use chrono::{Duration, NaiveDate};

    #[test]
    fn test_us_sofr_holiday() {
        // Test all results from 2023-01-01 to 2023-12-31
        let expected_results_for_2023 = vec![
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, false, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, false, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, true, false, false, true, true, true, true, true, false, false, true, true, true,
            true, false, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, false,
            true, true, true, true, false, false, true, true, true, true, true, false, false, true,
            true, true, true, true, false, false, false, true, true, true, true, false, false,
            true, true, true, true, true, false, false, true, false, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, true, true, true, true, true, false, false, true, true, true, true, true, false,
            false, false, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, true, true, true, true, true,
            false, false, true, true, true, true, true, false, false, false, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true, true,
            true, false, false, true, true, true, true, true, false, false, true, true, true,
            false, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, true, true, true, true, true, false, false, true, true,
            true, true, true, false, false, false, true, true, true, true, false, false,
        ];
        let first_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        for n in 0i32..365 {
            let target_date = first_date + Duration::days(n as i64);
            let expected = expected_results_for_2023[n as usize];
            assert_eq!(
                UnitedStates {
                    market: UnitedStatesMarket::SOFR
                }
                .is_business_day(target_date),
                expected
            );
        }
    }
}
