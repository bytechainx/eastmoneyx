//! eastmoneyx 的离线解析器：**只实现脱敏 CSV**。
//!
//! 入口形态固定为「字符串 → 值对象集合」，MUST NOT 接受 URL、HTTP 客户端、
//! 认证信息或任何网络相关参数。JSONP 剥离属授权后的**未来**任务，本层未实现
//! （见 [`guard_parse_format`]）。
//!
//! **表头严格匹配**：列数与列名逐字相等，多一列、少一列、改名或改序一律原子失败
//! （MUST NOT 静默忽略未知字段）。
//!
//! **重复身份**：本层选择**拒绝**（不静默去重），返回
//! [`EastMoneyError::SemanticallyRejected`]。各类型的身份定义：
//!
//! | 类型 | 身份 |
//! | --- | --- |
//! | 货币供应量 | `series + period` |
//! | 同业利率 | `series + period` |
//! | 社融 | `series + period` |
//! | 北向资金 | `date` |
//! | 公开市场操作 | `date + op_type` |
//!
//! **频率不由列给出**，而由类型决定（月频 / 日频 / 事件频），避免源侧出现
//! 「同一类型两种频率」的静默歧义。

use std::collections::HashSet;

use csv::{ReaderBuilder, StringRecord};

use crate::error::{EastMoneyError, EastMoneyResult};
use crate::value::{
    validate_money_supply, Date, EastMoneyAggregateFinancing, EastMoneyCode,
    EastMoneyInterbankRate, EastMoneyInterbankSeries, EastMoneyMissingReason, EastMoneyMoneySupply,
    EastMoneyNorthboundFlow, EastMoneyOmoOperation, EastMoneyValue, Frequency, Period, Unit,
};

/// 货币供应量表头。
const HEADER_MONEY_SUPPLY: [&str; 9] = [
    "series", "period", "m0", "m1", "m2", "m0_yoy", "m1_yoy", "m2_yoy", "unit",
];

/// 同业利率表头。
const HEADER_INTERBANK_RATE: [&str; 5] = ["period", "series", "rate", "change_bp", "unit"];

/// 社融表头。
const HEADER_AGGREGATE_FINANCING: [&str; 6] =
    ["series", "period", "increment", "stock", "yoy", "unit"];

/// 北向资金表头。
const HEADER_NORTHBOUND_FLOW: [&str; 6] =
    ["date", "sh_net", "sz_net", "total_net", "balance", "unit"];

/// 公开市场操作表头。
const HEADER_OMO_OPERATION: [&str; 7] = [
    "date",
    "op_type",
    "term_days",
    "amount",
    "rate",
    "net_injection",
    "unit",
];

// ---------------------------------------------------------------------------
// 格式守卫
// ---------------------------------------------------------------------------

/// 解析格式。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EastMoneyParseFormat {
    /// 脱敏 CSV（当前唯一实现的格式）。
    Csv,
    /// JSONP 包装（规划；当前**未实现**）。
    Jsonp,
}

/// 解析格式守卫：只放行 CSV；JSONP 剥离属未来任务，当前一律 `NotApplicable`。
///
/// # Errors
///
/// 传入 [`EastMoneyParseFormat::Jsonp`] 时返回 [`EastMoneyError::NotApplicable`]。
pub fn guard_parse_format(format: EastMoneyParseFormat) -> EastMoneyResult<()> {
    match format {
        EastMoneyParseFormat::Csv => Ok(()),
        EastMoneyParseFormat::Jsonp => Err(EastMoneyError::NotApplicable(
            "JSONP 剥离属授权后的未来 parser 任务，当前未实现（现有磁盘样本全为 CSV）".into(),
        )),
    }
}

// ---------------------------------------------------------------------------
// 解析入口
// ---------------------------------------------------------------------------

/// 解析货币供应量 CSV（表头 `series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit`）。
///
/// # Errors
///
/// 表头不符、字段缺失、数值非法、期间非月度、重复身份或校验不通过时返回对应错误。
pub fn parse_eastmoney_observations(input: &str) -> EastMoneyResult<Vec<EastMoneyMoneySupply>> {
    let rows = read_rows(input, &HEADER_MONEY_SUPPLY)?;
    let mut seen: HashSet<(String, Period)> = HashSet::new();
    let mut out = Vec::with_capacity(rows.len());
    for (index, record) in rows.iter().enumerate() {
        let series = EastMoneyCode::new(field(record, 0)?)?;
        let period = parse_month_period(field(record, 1)?)?;
        if !seen.insert((series.as_str().to_string(), period)) {
            return Err(duplicate_identity(index, "series + period"));
        }
        let observation = EastMoneyMoneySupply {
            series,
            period,
            m0: parse_value(field(record, 2)?)?,
            m1: parse_value(field(record, 3)?)?,
            m2: parse_value(field(record, 4)?)?,
            m0_yoy: parse_value(field(record, 5)?)?,
            m1_yoy: parse_value(field(record, 6)?)?,
            m2_yoy: parse_value(field(record, 7)?)?,
            unit: parse_unit(field(record, 8)?)?,
            frequency: Frequency::Monthly,
            revision: None,
        };
        validate_money_supply(&observation)?;
        out.push(observation);
    }
    Ok(out)
}

/// 解析同业利率 CSV（表头 `period,series,rate,change_bp,unit`）。
///
/// # Errors
///
/// 表头不符、序列令牌未知、数值非法、重复身份时返回对应错误。
pub fn parse_eastmoney_interbank_rates(
    input: &str,
) -> EastMoneyResult<Vec<EastMoneyInterbankRate>> {
    let rows = read_rows(input, &HEADER_INTERBANK_RATE)?;
    let mut seen: HashSet<(String, Period)> = HashSet::new();
    let mut out = Vec::with_capacity(rows.len());
    for (index, record) in rows.iter().enumerate() {
        let period = Period::Day(Date::parse(field(record, 0)?)?);
        let series = EastMoneyInterbankSeries::parse(field(record, 1)?)?;
        if !seen.insert((series.label().to_string(), period)) {
            return Err(duplicate_identity(index, "series + period"));
        }
        out.push(EastMoneyInterbankRate {
            series,
            period,
            rate: parse_value(field(record, 2)?)?,
            change_bp: parse_value(field(record, 3)?)?,
            unit: parse_unit(field(record, 4)?)?,
            frequency: Frequency::Daily,
            revision: None,
        });
    }
    Ok(out)
}

/// 解析社融 CSV（表头 `series,period,increment,stock,yoy,unit`）。
///
/// # Errors
///
/// 表头不符、字段缺失、数值非法、期间非月度、重复身份时返回对应错误。
pub fn parse_eastmoney_aggregate_financing(
    input: &str,
) -> EastMoneyResult<Vec<EastMoneyAggregateFinancing>> {
    let rows = read_rows(input, &HEADER_AGGREGATE_FINANCING)?;
    let mut seen: HashSet<(String, Period)> = HashSet::new();
    let mut out = Vec::with_capacity(rows.len());
    for (index, record) in rows.iter().enumerate() {
        let series = EastMoneyCode::new(field(record, 0)?)?;
        let period = parse_month_period(field(record, 1)?)?;
        if !seen.insert((series.as_str().to_string(), period)) {
            return Err(duplicate_identity(index, "series + period"));
        }
        out.push(EastMoneyAggregateFinancing {
            series,
            period,
            increment: parse_value(field(record, 2)?)?,
            stock: parse_value(field(record, 3)?)?,
            yoy: parse_value(field(record, 4)?)?,
            unit: parse_unit(field(record, 5)?)?,
            frequency: Frequency::Monthly,
            revision: None,
        });
    }
    Ok(out)
}

/// 解析北向资金 CSV（表头 `date,sh_net,sz_net,total_net,balance,unit`）。
///
/// # Errors
///
/// 表头不符、日期非法、数值非法、重复身份时返回对应错误。
pub fn parse_eastmoney_northbound_flows(
    input: &str,
) -> EastMoneyResult<Vec<EastMoneyNorthboundFlow>> {
    let rows = read_rows(input, &HEADER_NORTHBOUND_FLOW)?;
    let mut seen: HashSet<Date> = HashSet::new();
    let mut out = Vec::with_capacity(rows.len());
    for (index, record) in rows.iter().enumerate() {
        let date = Date::parse(field(record, 0)?)?;
        if !seen.insert(date) {
            return Err(duplicate_identity(index, "date"));
        }
        out.push(EastMoneyNorthboundFlow {
            date,
            sh_net: parse_value(field(record, 1)?)?,
            sz_net: parse_value(field(record, 2)?)?,
            total_net: parse_value(field(record, 3)?)?,
            balance: parse_value(field(record, 4)?)?,
            unit: parse_unit(field(record, 5)?)?,
            frequency: Frequency::Daily,
            revision: None,
        });
    }
    Ok(out)
}

/// 解析公开市场操作 CSV（表头 `date,op_type,term_days,amount,rate,net_injection,unit`）。
///
/// # Errors
///
/// 表头不符、日期非法、期限非法、数值非法、重复身份时返回对应错误。
pub fn parse_eastmoney_omo_operations(input: &str) -> EastMoneyResult<Vec<EastMoneyOmoOperation>> {
    let rows = read_rows(input, &HEADER_OMO_OPERATION)?;
    let mut seen: HashSet<(Date, String)> = HashSet::new();
    let mut out = Vec::with_capacity(rows.len());
    for (index, record) in rows.iter().enumerate() {
        let date = Date::parse(field(record, 0)?)?;
        let op_type = EastMoneyCode::new(field(record, 1)?)?;
        if !seen.insert((date, op_type.as_str().to_string())) {
            return Err(duplicate_identity(index, "date + op_type"));
        }
        out.push(EastMoneyOmoOperation {
            date,
            op_type,
            term_days: parse_term_days(field(record, 2)?)?,
            amount: parse_value(field(record, 3)?)?,
            rate: parse_value(field(record, 4)?)?,
            net_injection: parse_value(field(record, 5)?)?,
            unit: parse_unit(field(record, 6)?)?,
            frequency: Frequency::Event,
            revision: None,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 内部工具
// ---------------------------------------------------------------------------

/// 读取 CSV 全部记录并校验表头；返回不含表头的数据行。
fn read_rows(input: &str, expected: &[&str]) -> EastMoneyResult<Vec<StringRecord>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(false)
        .from_reader(input.as_bytes());
    let mut all = Vec::new();
    for record in reader.records() {
        all.push(record.map_err(csv_syntax_error)?);
    }
    let Some(header) = all.first() else {
        return Err(EastMoneyError::Invalid("CSV 为空（缺少表头）".into()));
    };
    if header.len() != expected.len() {
        return Err(EastMoneyError::Invalid(format!(
            "表头列数不符：期望 {} 列，实际 {} 列",
            expected.len(),
            header.len()
        )));
    }
    for (got, want) in header.iter().zip(expected.iter()) {
        if got != *want {
            return Err(EastMoneyError::Invalid(format!(
                "表头列名不符：期望 {want}，实际 {got}"
            )));
        }
    }
    Ok(all.split_off(1))
}

/// 把 CSV 语法错误映射为本层错误（**不回显**原始行内容）。
fn csv_syntax_error(error: csv::Error) -> EastMoneyError {
    match error.position() {
        Some(position) => EastMoneyError::Invalid(format!(
            "CSV 语法错误（记录 {}，行 {}）",
            position.record(),
            position.line()
        )),
        None => EastMoneyError::Invalid("CSV 语法错误（无法定位行号）".into()),
    }
}

/// 取第 `index` 列（0 起）的值。
fn field(record: &StringRecord, index: usize) -> EastMoneyResult<&str> {
    record
        .get(index)
        .ok_or_else(|| EastMoneyError::Missing(format!("第 {} 列缺失", index + 1)))
}

/// 构造「重复身份」错误。
fn duplicate_identity(index: usize, identity: &str) -> EastMoneyError {
    EastMoneyError::SemanticallyRejected(format!(
        "重复身份（{identity}）出现在第 {} 条数据行；本层选择拒绝而非静默去重",
        index + 1
    ))
}

/// 解析数值列：空 → 具名缺失；非数值 / 非有限 → 拒绝（**不回显**原始内容）。
fn parse_value(raw: &str) -> EastMoneyResult<EastMoneyValue> {
    if raw.is_empty() {
        return Ok(EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank));
    }
    let value = raw
        .parse::<f64>()
        .map_err(|_| EastMoneyError::Invalid("数值列含非数值内容".into()))?;
    if !value.is_finite() {
        return Err(EastMoneyError::Invalid(
            "数值列含非有限值（NaN / 无穷）".into(),
        ));
    }
    Ok(EastMoneyValue::Present(value))
}

/// 解析单位令牌（保留源单位，不做换算）。
fn parse_unit(raw: &str) -> EastMoneyResult<Unit> {
    match raw {
        "%" => Ok(Unit::Percent),
        "bp" => Ok(Unit::BasisPoint),
        "亿元" => Ok(Unit::HundredMillionYuan),
        "元" => Ok(Unit::Yuan),
        "" => Ok(Unit::Undeclared),
        other => Err(EastMoneyError::SemanticallyRejected(format!(
            "未知的单位令牌（长度 {}）；不得静默假定单位",
            other.len()
        ))),
    }
}

/// 解析月度期间（`YYYY-MM`）。
fn parse_month_period(raw: &str) -> EastMoneyResult<Period> {
    let period = Period::parse(raw)?;
    if !matches!(period, Period::Month { .. }) {
        return Err(EastMoneyError::Invalid(
            "该列必须是月度期间（YYYY-MM）".into(),
        ));
    }
    Ok(period)
}

/// 解析期限天数（无符号整数）。
fn parse_term_days(raw: &str) -> EastMoneyResult<u32> {
    if raw.is_empty() {
        return Err(EastMoneyError::Missing("term_days 不得为空".into()));
    }
    raw.parse::<u32>()
        .map_err(|_| EastMoneyError::Invalid("term_days 必须是无符号整数".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONEY_SUPPLY: &str = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-07,110.5,660.1,3050.2,9.1,6.2,8.0,亿元\n\
money_supply,2026-08,,,3060.9,,,8.1,亿元\n";

    const INTERBANK: &str = "period,series,rate,change_bp,unit\n\
2026-08-15,SHIBOR_ON,1.42,-3.5,%\n\
2026-08-15,DR007,1.55,2.0,%\n\
2026-08-20,LPR_1Y,3.00,0,%\n";

    const FINANCING: &str = "series,period,increment,stock,yoy,unit\n\
aggregate_financing,2026-07,7700.0,412500.0,8.4,亿元\n";

    const NORTHBOUND: &str = "date,sh_net,sz_net,total_net,balance,unit\n\
2026-08-15,1200.5,-300.2,900.3,21000.0,亿元\n";

    const OMO: &str = "date,op_type,term_days,amount,rate,net_injection,unit\n\
2026-08-15,reverse_repo,7,1200.0,1.42,700.0,亿元\n";

    #[test]
    fn money_supply_parses_with_named_missing() {
        let parsed = parse_eastmoney_observations(MONEY_SUPPLY).expect("解析成功");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].m2, EastMoneyValue::Present(3050.2));
        assert_eq!(
            parsed[1].m0,
            EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank)
        );
        assert_eq!(parsed[1].unit, Unit::HundredMillionYuan);
        assert_eq!(parsed[1].frequency, Frequency::Monthly);
        assert!(parsed[1].revision.is_none(), "无官方 vintage 面须为 None");
    }

    #[test]
    fn money_supply_rejects_unknown_and_reordered_headers() {
        let unknown = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit,extra\n\
money_supply,2026-08,1,2,3,4,5,6,亿元,0\n";
        assert!(
            parse_eastmoney_observations(unknown).is_err(),
            "多一列必须拒绝"
        );

        let renamed = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,units\n\
money_supply,2026-08,1,2,3,4,5,6,亿元\n";
        assert!(
            parse_eastmoney_observations(renamed).is_err(),
            "改名必须拒绝"
        );

        let reordered = "period,series,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
2026-08,money_supply,1,2,3,4,5,6,亿元\n";
        assert!(
            parse_eastmoney_observations(reordered).is_err(),
            "改序必须拒绝"
        );
    }

    #[test]
    fn money_supply_rejects_duplicate_identity_and_bad_values() {
        let duplicate = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,1,2,3,4,5,6,亿元\n\
money_supply,2026-08,1,2,4,4,5,7,亿元\n";
        let error = parse_eastmoney_observations(duplicate).expect_err("重复身份必须拒绝");
        assert_eq!(
            error.kind(),
            crate::EastMoneyErrorKind::SemanticallyRejected
        );

        let bad_period = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08-15,1,2,3,4,5,6,亿元\n";
        assert!(
            parse_eastmoney_observations(bad_period).is_err(),
            "非月度必须拒绝"
        );

        let bad_number = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,1,2,not-a-number,4,5,6,亿元\n";
        assert!(parse_eastmoney_observations(bad_number).is_err());

        let bad_unit = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,1,2,3,4,5,6,parsec\n";
        assert!(parse_eastmoney_observations(bad_unit).is_err());
    }

    #[test]
    fn interbank_rate_parses_three_series() {
        let parsed = parse_eastmoney_interbank_rates(INTERBANK).expect("解析成功");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].series, EastMoneyInterbankSeries::ShiborOn);
        assert_eq!(parsed[1].series, EastMoneyInterbankSeries::Dr007);
        assert_eq!(parsed[2].series, EastMoneyInterbankSeries::Lpr1y);
        assert_eq!(parsed[0].change_bp, EastMoneyValue::Present(-3.5));
    }

    #[test]
    fn interbank_rate_rejects_unknown_series_token() {
        let unknown = "period,series,rate,change_bp,unit\n2026-08-15,USD1M,1.4,0,%\n";
        let error = parse_eastmoney_interbank_rates(unknown).expect_err("未知序列必须拒绝");
        assert_eq!(
            error.kind(),
            crate::EastMoneyErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn financing_northbound_and_omo_parse() {
        assert_eq!(
            parse_eastmoney_aggregate_financing(FINANCING)
                .expect("解析成功")
                .len(),
            1
        );
        assert_eq!(
            parse_eastmoney_northbound_flows(NORTHBOUND)
                .expect("解析成功")
                .len(),
            1
        );
        let omo = parse_eastmoney_omo_operations(OMO).expect("解析成功");
        assert_eq!(omo[0].term_days, 7);
        assert_eq!(omo[0].frequency, Frequency::Event);
    }

    #[test]
    fn negative_inputs_are_rejected_per_type() {
        // 缺列（表头少一列）
        assert!(
            parse_eastmoney_aggregate_financing("series,period,increment,stock,yoy\n").is_err()
        );
        // 非法日期
        assert!(parse_eastmoney_northbound_flows(
            "date,sh_net,sz_net,total_net,balance,unit\n2026-2-3,1,2,3,4,亿元\n"
        )
        .is_err());
        // 重复身份
        let dup_north = "date,sh_net,sz_net,total_net,balance,unit\n\
2026-08-15,1,2,3,4,亿元\n\
2026-08-15,1,2,3,5,亿元\n";
        assert!(parse_eastmoney_northbound_flows(dup_north).is_err());
        // 空 term_days
        let bad_term = "date,op_type,term_days,amount,rate,net_injection,unit\n\
2026-08-15,reverse_repo,,1,2,3,亿元\n";
        assert!(parse_eastmoney_omo_operations(bad_term).is_err());
    }

    #[test]
    fn empty_input_is_rejected_as_missing_header() {
        assert!(parse_eastmoney_observations("").is_err());
        assert!(parse_eastmoney_interbank_rates("").is_err());
    }

    #[test]
    fn jsonp_format_is_not_applicable() {
        assert!(guard_parse_format(EastMoneyParseFormat::Csv).is_ok());
        let error = guard_parse_format(EastMoneyParseFormat::Jsonp).expect_err("未实现");
        assert_eq!(error.kind(), crate::EastMoneyErrorKind::NotApplicable);
    }

    #[test]
    fn adversarial_headers_and_identity_are_not_trimmed() {
        let header = "period,series,rate,change_bp,unit\n";
        for input in [
            " period ,series,rate,change_bp,unit\n2026-08-15,SHIBOR_ON,1,0,%".to_owned(),
            format!("{header}2026-08-15, SHIBOR_ON ,1,0,%"),
        ] {
            assert!(parse_eastmoney_interbank_rates(&input).is_err());
        }
    }
    #[test]
    fn adversarial_daily_rate_rejects_other_periods() {
        for period in ["2026", "2026-08", "2026-Q3"] {
            let input = format!("period,series,rate,change_bp,unit\n{period},SHIBOR_ON,1,0,%");
            assert!(parse_eastmoney_interbank_rates(&input).is_err());
        }
    }
}
