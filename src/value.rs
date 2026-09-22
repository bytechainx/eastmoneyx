//! eastmoneyx 的基础值对象：日期 / 期间 / 频率 / 单位 / 源侧码，以及产品范围守卫。
//!
//! 本模块只表达**源事实的身份与量纲**：不引入日期库、不做单位换算、
//! 不实现任何派生指标（净投放 5d / 中美利差 / 剪刀差等均属 analytics）。

use crate::error::{EastMoneyError, EastMoneyResult};

mod observation;

pub use observation::{
    validate_money_supply, EastMoneyAggregateFinancing, EastMoneyInterbankRate,
    EastMoneyInterbankSeries, EastMoneyMissingReason, EastMoneyMoneySupply,
    EastMoneyNorthboundFlow, EastMoneyOmoOperation, EastMoneySeriesKind, EastMoneyValue,
    SERIES_KINDS,
};

// ---------------------------------------------------------------------------
// 日期与期间
// ---------------------------------------------------------------------------

/// 日历日期（严格 ISO `YYYY-MM-DD` 形态的身份，不含时区与时刻）。
///
/// 字段公开以便调用方直接读取年份 / 月份 / 日；构造 MUST 经 [`Date::new`] 或
/// [`Date::parse`] 校验，否则可能得到非法日期（如 2 月 30 日）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// 年份（4 位形态，取值 1000–9999）。
    pub year: i16,
    /// 月份（1–12）。
    pub month: u8,
    /// 日（按当月天数与闰年规则校验）。
    pub day: u8,
}

impl Date {
    /// 构造并校验一个日期。
    ///
    /// # Errors
    ///
    /// 年份非 4 位形态、月份越界、或日超出该月实际天数时返回
    /// [`EastMoneyError::Invalid`]。
    pub fn new(year: i16, month: u8, day: u8) -> EastMoneyResult<Self> {
        if !(1000..=9999).contains(&year) {
            return Err(EastMoneyError::Invalid(
                "年份须为 4 位形态（1000–9999）".into(),
            ));
        }
        if !(1..=12).contains(&month) {
            return Err(EastMoneyError::Invalid("月份须在 1–12 之间".into()));
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(EastMoneyError::Invalid(
                "日超出该月实际天数（已含闰年规则）".into(),
            ));
        }
        Ok(Self { year, month, day })
    }

    /// 解析严格 ISO `YYYY-MM-DD`。
    ///
    /// 月与日 MUST 两位补零；MUST NOT 接受 `2026-2-3`、`2026/02/03` 或带时间部分者。
    ///
    /// # Errors
    ///
    /// 形态不符或日历非法时返回 [`EastMoneyError::Invalid`]。
    ///
    /// # Examples
    ///
    /// ```
    /// use eastmoneyx::Date;
    ///
    /// assert_eq!(Date::parse("2026-08-15")?.year, 2026);
    /// assert!(Date::parse("2026-2-3").is_err(), "月/日必须两位补零");
    /// assert!(Date::parse("2026/08/15").is_err(), "分隔符必须是 '-'");
    /// assert!(Date::parse("2026-02-30").is_err(), "闰年规则必须校验");
    /// # Ok::<(), eastmoneyx::EastMoneyError>(())
    /// ```
    pub fn parse(input: &str) -> EastMoneyResult<Self> {
        let b = input.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return Err(EastMoneyError::Invalid(
                "日期须为严格 ISO 形态 YYYY-MM-DD".into(),
            ));
        }
        let year = four_digits(&b[0..4])? as i16;
        let month = two_digits(&b[5..7])?;
        let day = two_digits(&b[8..10])?;
        Self::new(year, month, day)
    }
}

/// 该月实际天数（含闰年规则）。
fn days_in_month(year: i16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// 闰年判定：能被 4 整除且（不能被 100 整除 或 能被 400 整除）。
fn is_leap_year(year: i16) -> bool {
    let y = i32::from(year);
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

/// 解析 4 位 ASCII 数字。
fn four_digits(b: &[u8]) -> EastMoneyResult<u16> {
    if b.len() != 4 || !b.iter().all(u8::is_ascii_digit) {
        return Err(EastMoneyError::Invalid("年份须为 4 位数字".into()));
    }
    Ok(u16::from(b[0] - b'0') * 1000
        + u16::from(b[1] - b'0') * 100
        + u16::from(b[2] - b'0') * 10
        + u16::from(b[3] - b'0'))
}

/// 解析 2 位 ASCII 数字。
fn two_digits(b: &[u8]) -> EastMoneyResult<u8> {
    if b.len() != 2 || !b.iter().all(u8::is_ascii_digit) {
        return Err(EastMoneyError::Invalid("月/日须为 2 位数字".into()));
    }
    Ok((b[0] - b'0') * 10 + (b[1] - b'0'))
}

/// 解析 1 位 ASCII 数字。
fn one_digit(b: &[u8]) -> EastMoneyResult<u8> {
    if b.len() != 1 || !b[0].is_ascii_digit() {
        return Err(EastMoneyError::Invalid("季度须为 1 位数字".into()));
    }
    Ok(b[0] - b'0')
}

/// 业务期间的身份。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Period {
    /// 日度期间。
    Day(Date),
    /// 月度期间。
    Month {
        /// 年份。
        year: i16,
        /// 月份（1–12）。
        month: u8,
    },
    /// 季度期间。
    Quarter {
        /// 年份。
        year: i16,
        /// 季度（1–4）。
        quarter: u8,
    },
    /// 年度期间。
    Year(i16),
    /// 事件型期间（时点事件，如一次公开市场操作）。
    Event {
        /// 事件日期。
        date: Date,
    },
}

impl Period {
    /// 复验公开期间分量，避免直接构造绕过日期范围。
    pub(crate) fn validate(&self) -> EastMoneyResult<()> {
        match *self {
            Self::Day(date) | Self::Event { date } => {
                Date::new(date.year, date.month, date.day).map(|_| ())
            }
            Self::Month { year, month } => Date::new(year, month, 1).map(|_| ()),
            Self::Quarter { year, quarter } => {
                Date::new(year, 1, 1)?;
                if !(1..=4).contains(&quarter) {
                    return Err(EastMoneyError::Invalid("季度须在 1–4 之间".into()));
                }
                Ok(())
            }
            Self::Year(year) => Date::new(year, 1, 1).map(|_| ()),
        }
    }

    /// 解析期间形态：`YYYY-MM-DD` → [`Period::Day`]、`YYYY-MM` → [`Period::Month`]、
    /// `YYYY-Qn` → [`Period::Quarter`]、`YYYY` → [`Period::Year`]。
    ///
    /// # Errors
    ///
    /// 形态不属上述四种、或字段越界时返回 [`EastMoneyError::Invalid`]。
    pub fn parse(input: &str) -> EastMoneyResult<Self> {
        let b = input.as_bytes();
        let period = match b.len() {
            10 => Ok(Self::Day(Date::parse(input)?)),
            7 if b[4] == b'-' && (b[5] == b'Q' || b[5] == b'q') => {
                let year = four_digits(&b[0..4])? as i16;
                let quarter = one_digit(&b[6..7])?;
                if !(1..=4).contains(&quarter) {
                    return Err(EastMoneyError::Invalid("季度须在 1–4 之间".into()));
                }
                Ok(Self::Quarter { year, quarter })
            }
            7 if b[4] == b'-' => {
                let year = four_digits(&b[0..4])? as i16;
                let month = two_digits(&b[5..7])?;
                if !(1..=12).contains(&month) {
                    return Err(EastMoneyError::Invalid("月份须在 1–12 之间".into()));
                }
                Ok(Self::Month { year, month })
            }
            4 => Ok(Self::Year(four_digits(b)? as i16)),
            _ => Err(EastMoneyError::Invalid(
                "期间须为 YYYY / YYYY-MM / YYYY-Qn / YYYY-MM-DD 之一".into(),
            )),
        }?;
        period.validate()?;
        Ok(period)
    }
}

/// 观测频率。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frequency {
    /// 日频。
    Daily,
    /// 周频。
    Weekly,
    /// 月频。
    Monthly,
    /// 季频。
    Quarterly,
    /// 年频。
    Annual,
    /// 事件频（不定期发生时点）。
    Event,
    /// 不规则。
    Irregular,
}

/// 源侧单位。**保留源单位**，换算归下游 Normalize，本层不做。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unit {
    /// 百分比（如同比、利率的百分数形态）。
    Percent,
    /// 基点（如利率变动 `change_bp`）。
    BasisPoint,
    /// 亿元人民币（源侧口径原样保留）。
    HundredMillionYuan,
    /// 元人民币。
    Yuan,
    /// 源侧未声明单位（MUST NOT 静默假定为某个单位）。
    Undeclared,
}

// ---------------------------------------------------------------------------
// 源侧码
// ---------------------------------------------------------------------------

/// 源侧标识码 / 标签。
///
/// 清单**未钉死**其取值域（如公开市场操作的 `op_type`），故本层只做合法性校验，
/// MUST NOT 虚构取值全集。长度上限仅用于防御异常输入，不代表源侧约束。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EastMoneyCode(String);

impl EastMoneyCode {
    /// 构造并校验一个源侧码。
    ///
    /// # Errors
    ///
    /// 空串、前后空白、含控制字符或超过 64 个字符时返回 [`EastMoneyError::Invalid`]。
    pub fn new(value: &str) -> EastMoneyResult<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(EastMoneyError::Missing("源侧码不得为空".into()));
        }
        if trimmed.len() > 64 {
            return Err(EastMoneyError::Invalid("源侧码过长（上限 64）".into()));
        }
        if trimmed.chars().any(char::is_control) {
            return Err(EastMoneyError::Invalid("源侧码不得含控制字符".into()));
        }
        if trimmed != value {
            return Err(EastMoneyError::Invalid(
                "源侧码不得含前后空白（避免静默等值）".into(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// 以字符串切片读取。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// 产品范围守卫
// ---------------------------------------------------------------------------

/// 条目类型。当前合同只承认宏观观测。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EastMoneyKind {
    /// 观测（`kind=observation`）。
    Observation,
}

/// 数据产品类别。
///
/// **当前合同只允许 [`EastMoneyProduct::Macro`]**；行情 / 汇率 / 搜索 / 日历
/// MUST 被拒绝并视为路由他处。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EastMoneyProduct {
    /// 宏观观测（`product=macro`，当前合同唯一允许的产品）。
    Macro,
    /// 行情（拒绝）。
    Quote,
    /// 汇率（拒绝）。
    Forex,
    /// 搜索（拒绝）。
    Search,
    /// 日历（拒绝）。
    Calendar,
}

/// 校验「条目类型 + 产品」是否落在当前合同范围内。
///
/// 只放行 `kind=observation` + `product=macro`；其余产品返回
/// [`EastMoneyError::RoutedElsewhere`]（本域不是它们的所有者）。
///
/// # Errors
///
/// 产品不是宏观观测时返回 [`EastMoneyError::RoutedElsewhere`]。
///
/// # Examples
///
/// ```
/// use eastmoneyx::{validate_observation_scope, EastMoneyKind, EastMoneyProduct};
///
/// assert!(validate_observation_scope(EastMoneyKind::Observation, EastMoneyProduct::Macro).is_ok());
/// assert!(validate_observation_scope(EastMoneyKind::Observation, EastMoneyProduct::Quote).is_err());
/// ```
pub fn validate_observation_scope(
    kind: EastMoneyKind,
    product: EastMoneyProduct,
) -> EastMoneyResult<()> {
    match (kind, product) {
        (EastMoneyKind::Observation, EastMoneyProduct::Macro) => Ok(()),
        (EastMoneyKind::Observation, other) => Err(EastMoneyError::RoutedElsewhere(format!(
            "{} 不属于本域（当前合同仅脱敏宏观观测）",
            product_label(other)
        ))),
    }
}

/// 产品类别的可读名（用于错误消息，不做字符串分流）。
fn product_label(product: EastMoneyProduct) -> &'static str {
    match product {
        EastMoneyProduct::Macro => "macro",
        EastMoneyProduct::Quote => "行情（quote）",
        EastMoneyProduct::Forex => "汇率（forex）",
        EastMoneyProduct::Search => "搜索（search）",
        EastMoneyProduct::Calendar => "日历（calendar）",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_parses_strict_iso_only() {
        assert_eq!(
            Date::parse("2026-08-15").expect("合法日期"),
            Date {
                year: 2026,
                month: 8,
                day: 15
            }
        );
        for bad in [
            "2026-2-3",
            "2026/08/15",
            "2026-08-15T00:00:00",
            "20260815",
            "2026-13-01",
            "2026-00-10",
            "2026-02-29",
            "abcd-08-15",
            "2026-08-0",
            "",
        ] {
            assert!(Date::parse(bad).is_err(), "{bad} 必须被拒绝");
        }
    }

    #[test]
    fn date_honours_leap_year_rules() {
        assert!(Date::parse("2024-02-29").is_ok(), "2024 是闰年");
        assert!(Date::parse("2000-02-29").is_ok(), "2000 是 400 倍数闰年");
        assert!(Date::parse("1900-02-29").is_err(), "1900 非闰年");
        assert!(Date::parse("2023-02-29").is_err());
    }

    #[test]
    fn date_range_is_enforced() {
        assert!(Date::new(999, 1, 1).is_err(), "非 4 位形态年拒绝");
        assert!(Date::new(10000, 1, 1).is_err(), "超过 4 位形态年拒绝");
        assert_eq!(
            Date::new(2026, 8, 5).expect("合法日期"),
            Date {
                year: 2026,
                month: 8,
                day: 5
            }
        );
    }

    #[test]
    fn period_parses_four_forms_and_rejects_others() {
        assert_eq!(
            Period::parse("2026-08-15").expect("合法日期间"),
            Period::Day(Date {
                year: 2026,
                month: 8,
                day: 15
            })
        );
        assert_eq!(
            Period::parse("2026-08").expect("合法月度期间"),
            Period::Month {
                year: 2026,
                month: 8
            }
        );
        assert_eq!(
            Period::parse("2026-Q3").expect("合法季度期间"),
            Period::Quarter {
                year: 2026,
                quarter: 3
            }
        );
        assert_eq!(
            Period::parse("2026").expect("合法年度期间"),
            Period::Year(2026)
        );
        for bad in [
            "2026-13",
            "2026-Q5",
            "26-08",
            "2026-8",
            "2026-08-1",
            "2026/08",
        ] {
            assert!(Period::parse(bad).is_err(), "{bad} 必须被拒绝");
        }
    }

    #[test]
    fn code_rejects_blank_and_overlong() {
        assert_eq!(
            EastMoneyCode::new("reverse_repo").expect("合法码").as_str(),
            "reverse_repo"
        );
        assert!(EastMoneyCode::new("").is_err());
        assert!(EastMoneyCode::new("   ").is_err());
        assert!(EastMoneyCode::new(" a").is_err(), "前后空白必须拒绝");
        assert!(EastMoneyCode::new("a\nb").is_err(), "控制字符必须拒绝");
        assert!(EastMoneyCode::new(&"x".repeat(65)).is_err(), "过长必须拒绝");
    }

    #[test]
    fn scope_guard_only_admits_macro_observation() {
        assert!(
            validate_observation_scope(EastMoneyKind::Observation, EastMoneyProduct::Macro).is_ok()
        );
        for product in [
            EastMoneyProduct::Quote,
            EastMoneyProduct::Forex,
            EastMoneyProduct::Search,
            EastMoneyProduct::Calendar,
        ] {
            let error =
                validate_observation_scope(EastMoneyKind::Observation, product).expect_err("拒绝");
            assert_eq!(error.kind(), crate::EastMoneyErrorKind::RoutedElsewhere);
        }
    }

    #[test]
    fn adversarial_period_year_range_is_consistent() {
        for period in ["0000", "0999", "0000-01", "0999-Q1"] {
            assert!(Period::parse(period).is_err(), "非法年份：{period}");
        }
    }
}
