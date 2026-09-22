#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! AIDD 对抗 / 边界用例（特性 005）。
//!
//! 候选由 AI 生成，逐条人工复核后仅保留「结论=保留」项；丢弃项登记于 PR 描述。
//!
//! // AIDD: 极大有限数 1e308 仍视为合法取值 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §2 值对象与期间标准 | 结论=保留
//! // AIDD: 溢出为无穷的 1e309 必须拒绝 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §2 值对象与期间标准 | 结论=保留
//! // AIDD: 空输入（无表头）必须拒绝 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 解析标准 | 结论=保留
//! // AIDD: 表头带 UTF-8 BOM 时由 CSV 读取器剥离且列名不变 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 解析标准 | 结论=保留
//! // AIDD: 未知单位令牌不得静默降级为 Undeclared | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 解析标准 | 结论=保留
//! // AIDD: 仅日期有值、其余列全空的北向观测仍可解析 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §3 解析标准 | 结论=保留
//! // AIDD: 代理轮换与联网通道同等被拒 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §4 授权与执行纪律标准 | 结论=保留
//! // AIDD: 合成夹具不得被表述为实测 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §6 合成夹具声明 | 结论=保留

use eastmoneyx::{
    guard_live_execution, parse_eastmoney_northbound_flows, parse_eastmoney_observations,
    validate_money_supply, EastMoneyCode, EastMoneyExecutionChannel, EastMoneyMissingReason,
    EastMoneyMoneySupply, EastMoneyValue, Frequency, Period, Unit,
};

/// 边界：`1e308` 是有限数，属合法取值（本层不做范围裁剪）。
#[test]
fn extreme_finite_value_is_accepted() {
    let csv = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,1e308,,,0,0,0,亿元\n";
    let parsed = parse_eastmoney_observations(csv).expect("有限极值可解析");
    assert_eq!(parsed[0].m0, EastMoneyValue::Present(1e308));
}

/// 边界：`1e309` 溢出为无穷，必须拒绝而非静默钳制。
#[test]
fn overflowing_value_is_rejected() {
    let csv = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,1e309,,,0,0,0,亿元\n";
    let error = parse_eastmoney_observations(csv).expect_err("无穷必须拒绝");
    assert_eq!(error.kind(), eastmoneyx::EastMoneyErrorKind::Invalid);
}

/// 边界：空输入与仅换行输入都视为缺少表头。
#[test]
fn empty_input_is_not_a_valid_table() {
    assert!(parse_eastmoney_observations("").is_err());
    assert!(parse_eastmoney_observations("\n").is_err());
}

/// 边界：输入首部的 UTF-8 BOM 由 CSV 读取器剥离；列名不变、列数不变。
#[test]
fn byte_order_mark_is_stripped_without_shifting_columns() {
    let csv = "\u{feff}series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,1,2,3,4,5,6,亿元\n";
    let parsed = parse_eastmoney_observations(csv).expect("BOM 由读取器剥离");
    assert_eq!(parsed.len(), 1);
    assert_eq!(
        parsed[0].series.as_str(),
        "money_supply",
        "首列列名不得因 BOM 而改变（否则第 1 条记录会被错位读出）"
    );
    assert_eq!(parsed[0].m2, EastMoneyValue::Present(3.0));
}

/// 边界：未知单位令牌必须拒绝；只有真正空单元格才落到 `Undeclared`。
#[test]
fn unknown_unit_token_is_not_silently_downgraded() {
    let unknown = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,1,2,3,4,5,6,parsec\n";
    assert!(parse_eastmoney_observations(unknown).is_err());

    let blank = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,1,2,3,4,5,6,\n";
    let parsed = parse_eastmoney_observations(blank).expect("空单位列可解析");
    assert_eq!(parsed[0].unit, Unit::Undeclared);
}

/// 边界：北向观测只给日期、其余列全空仍可解析（缺失具名，不静默转 0）。
#[test]
fn northbound_row_with_only_date_parses() {
    let csv = "date,sh_net,sz_net,total_net,balance,unit\n\
2026-08-15,,,,,亿元\n";
    let parsed = parse_eastmoney_northbound_flows(csv).expect("可解析");
    assert_eq!(
        parsed[0].sh_net,
        EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank)
    );
    assert_eq!(
        parsed[0].total_net,
        EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank)
    );
}

/// 边界：代理轮换通道与联网通道同等被拒，且理由可读。
#[test]
fn proxy_rotation_is_blocked_like_network() {
    for channel in [
        EastMoneyExecutionChannel::Network,
        EastMoneyExecutionChannel::ProxyRotation,
    ] {
        let error = guard_live_execution(channel).expect_err("必须拒绝");
        let message = error.to_string();
        assert!(message.contains("当前执行禁止"), "理由须可读：{message}");
        assert!(
            !message.to_ascii_lowercase().contains("proxy="),
            "理由不得回显任何代理地址"
        );
    }
}

/// 边界：合成夹具的缺失原因必须互不混淆；伪造 `revision` 不得通过校验。
#[test]
fn missing_reasons_and_revision_are_distinguished() {
    assert_ne!(
        EastMoneyMissingReason::SourceBlank,
        EastMoneyMissingReason::NotPublishedYet
    );
    let observation = EastMoneyMoneySupply {
        series: EastMoneyCode::new("money_supply").expect("合法码"),
        period: Period::Month {
            year: 2026,
            month: 8,
        },
        m0: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
        m1: EastMoneyValue::Missing(EastMoneyMissingReason::NotPublishedYet),
        m2: EastMoneyValue::Missing(EastMoneyMissingReason::NotApplicable),
        m0_yoy: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
        m1_yoy: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
        m2_yoy: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
        unit: Unit::HundredMillionYuan,
        frequency: Frequency::Monthly,
        revision: Some(EastMoneyCode::new("v2").expect("合法码")),
    };
    assert!(
        validate_money_supply(&observation).is_err(),
        "伪造 revision 与三项全缺都必须被拒绝"
    );
}
