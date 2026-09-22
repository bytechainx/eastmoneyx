#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! TDD 行为契约（特性 005）。
//!
//! 入口集合 = 本 crate 全部公开入口（含 `validate*` / 判定函数 / 解析器 / 值对象构造与解析）。
//! 下表每个入口先在 `/tmp` 变异副本上观测应红、再在本树观测绿。
//!
//! // TDD-PROBE: Date::new | 变异：只校验月范围，跳过日与闰年校验 | 红=date_new_validates_calendar_fields | 绿=date_new_validates_calendar_fields
//! // TDD-PROBE: Date::parse | 变异：接受 `2026/08/15` 作为合法分隔符 | 红=date_parse_is_strict_iso | 绿=date_parse_is_strict_iso
//! // TDD-PROBE: Period::parse | 变异：`YYYY-Q5` 也被接受为季度 | 红=period_parse_covers_four_forms | 绿=period_parse_covers_four_forms
//! // TDD-PROBE: EastMoneyCode::new | 变异：允许空串与前后空白 | 红=code_new_rejects_blank_and_control_chars | 绿=code_new_rejects_blank_and_control_chars
//! // TDD-PROBE: EastMoneyCode::as_str | 变异：as_str 返回含首尾空白的原串 | 红=code_as_str_round_trips | 绿=code_as_str_round_trips
//! // TDD-PROBE: validate_observation_scope | 变异：放行全部产品类别 | 红=scope_guard_rejects_non_macro_products | 绿=scope_guard_rejects_non_macro_products
//! // TDD-PROBE: validate_money_supply | 变异：跳过 revision 必须为 None 的检查 | 红=money_supply_validation_rejects_incomplete_rows | 绿=money_supply_validation_rejects_incomplete_rows
//! // TDD-PROBE: guard_parse_format | 变异：JSONP 也返回 Ok（宣称已实现） | 红=parse_format_guard_rejects_jsonp | 绿=parse_format_guard_rejects_jsonp
//! // TDD-PROBE: guard_live_execution | 变异：代理轮换通道被放行 | 红=live_execution_guard_denies_every_channel | 绿=live_execution_guard_denies_every_channel
//! // TDD-PROBE: authorize_eastmoney_access | 变异：证据缺失时默认放行 | 红=authorize_is_fail_closed | 绿=authorize_is_fail_closed
//! // TDD-PROBE: current_eastmoney_authorization | 变异：当前判定改为 Authorized | 红=current_authorization_is_always_denied | 绿=current_authorization_is_always_denied
//! // TDD-PROBE: parse_eastmoney_observations | 变异：重复身份改为静默去重 | 红=observations_parser_accepts_fixture_and_rejects_duplicates | 绿=observations_parser_accepts_fixture_and_rejects_duplicates
//! // TDD-PROBE: parse_eastmoney_interbank_rates | 变异：未知序列令牌被当作 DR007 | 红=interbank_parser_accepts_fixture_and_rejects_unknown_series | 绿=interbank_parser_accepts_fixture_and_rejects_unknown_series
//! // TDD-PROBE: parse_eastmoney_aggregate_financing | 变异：表头多出的列被静默忽略 | 红=financing_parser_rejects_unknown_columns | 绿=financing_parser_rejects_unknown_columns
//! // TDD-PROBE: parse_eastmoney_northbound_flows | 变异：日期放宽为 `YYYY-M-D` | 红=northbound_parser_accepts_fixture_and_rejects_bad_date | 绿=northbound_parser_accepts_fixture_and_rejects_bad_date
//! // TDD-PROBE: parse_eastmoney_omo_operations | 变异：term_days 非整数被当成 0 | 红=omo_parser_accepts_fixture_and_requires_integer_term_days | 绿=omo_parser_accepts_fixture_and_requires_integer_term_days
//! // TDD-PROBE: eastmoney_publication_semantics | 变异：可得性证据层改为 Official | 红=publication_triple_is_date_inferred_not_eligible | 绿=publication_triple_is_date_inferred_not_eligible
//! // TDD-PROBE: is_formal_pit_eligible | 变异：返回 true（静默升格为正式 PIT） | 红=formal_pit_is_never_eligible | 绿=formal_pit_is_never_eligible
//!
//! 夹具全部为合成样本，见 `tests/fixtures/` 的 `_synthetic` 标注与 `docs/标准.md` §6。

use eastmoneyx::{
    authorize_eastmoney_access, current_eastmoney_authorization, eastmoney_publication_semantics,
    guard_live_execution, guard_parse_format, is_formal_pit_eligible,
    parse_eastmoney_aggregate_financing, parse_eastmoney_interbank_rates,
    parse_eastmoney_northbound_flows, parse_eastmoney_observations, parse_eastmoney_omo_operations,
    validate_money_supply, validate_observation_scope, AvailabilityEvidence, Date,
    EastMoneyAggregateFinancing, EastMoneyAuthorization, EastMoneyCode, EastMoneyExecutionChannel,
    EastMoneyKind, EastMoneyMissingReason, EastMoneyMoneySupply, EastMoneyParseFormat,
    EastMoneyProduct, EastMoneyValue, Frequency, Period, PitEligibility, TimePrecision, Unit,
};

/// 读取合成夹具里的 CSV 载荷（并钉死「夹具为合成样本」这一事实）。
fn fixture_csv(raw: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(raw).expect("夹具必须是合法 JSON");
    assert_eq!(
        value["_synthetic"],
        serde_json::Value::Bool(true),
        "夹具必须标注 _synthetic=true"
    );
    assert_eq!(value["kind"], "observation", "夹具须对齐当前合同的条目类型");
    assert_eq!(value["product"], "macro", "夹具须对齐当前合同的产品类别");
    value["csv"]
        .as_str()
        .expect("夹具须含 csv 载荷字段")
        .to_string()
}

/// 一条合法的货币供应量观测（供 `validate_money_supply` 正负用例共用）。
fn sample_money_supply() -> EastMoneyMoneySupply {
    EastMoneyMoneySupply {
        series: EastMoneyCode::new("money_supply").expect("合法码"),
        period: Period::Month {
            year: 2026,
            month: 8,
        },
        m0: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
        m1: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
        m2: EastMoneyValue::Present(3060.9),
        m0_yoy: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
        m1_yoy: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
        m2_yoy: EastMoneyValue::Present(8.1),
        unit: Unit::HundredMillionYuan,
        frequency: Frequency::Monthly,
        revision: None,
    }
}

/// `Date::new`：月与日必须按日历校验（含闰年）。
#[test]
fn date_new_validates_calendar_fields() {
    assert!(Date::new(2026, 2, 29).is_err(), "2026 非闰年，2 月无 29 日");
    assert!(Date::new(2026, 4, 31).is_err(), "4 月只有 30 天");
    assert!(Date::new(2026, 13, 1).is_err(), "月份上限 12");
    assert!(Date::new(2026, 0, 1).is_err(), "月份下限 1");
    assert!(Date::new(2026, 1, 0).is_err(), "日下限 1");
    assert!(Date::new(2024, 2, 29).is_ok(), "2024 是闰年");
}

/// `Date::parse`：只接受严格 ISO `YYYY-MM-DD`。
#[test]
fn date_parse_is_strict_iso() {
    assert!(Date::parse("2026-08-15").is_ok());
    for bad in ["2026/08/15", "2026-2-3", "2026-08-15T00:00:00", "20260815"] {
        assert!(Date::parse(bad).is_err(), "{bad} 必须被拒绝");
    }
}

/// `Period::parse`：四种期间形态，季度限 1–4。
#[test]
fn period_parse_covers_four_forms() {
    assert!(matches!(Period::parse("2026-08-15"), Ok(Period::Day(_))));
    assert!(matches!(
        Period::parse("2026-08"),
        Ok(Period::Month { month: 8, .. })
    ));
    assert!(matches!(
        Period::parse("2026-Q4"),
        Ok(Period::Quarter { quarter: 4, .. })
    ));
    assert!(matches!(Period::parse("2026"), Ok(Period::Year(2026))));
    assert!(Period::parse("2026-Q5").is_err(), "季度上限 4");
    assert!(Period::parse("2026-13").is_err(), "月份上限 12");
}

/// `EastMoneyCode::new`：空串、空白、控制字符与超长一律拒绝。
#[test]
fn code_new_rejects_blank_and_control_chars() {
    assert_eq!(
        EastMoneyCode::new("money_supply").expect("合法码").as_str(),
        "money_supply"
    );
    assert!(EastMoneyCode::new("").is_err());
    assert!(EastMoneyCode::new("  ").is_err());
    assert!(EastMoneyCode::new(" a").is_err(), "前后空白必须拒绝");
    assert!(EastMoneyCode::new("a\nb").is_err(), "控制字符必须拒绝");
    assert!(EastMoneyCode::new(&"x".repeat(65)).is_err(), "超长必须拒绝");
}

/// `EastMoneyCode::as_str`：往返一致，不含被裁掉的空白。
#[test]
fn code_as_str_round_trips() {
    let code = EastMoneyCode::new("reverse_repo").expect("合法码");
    assert_eq!(code.as_str(), "reverse_repo");
    let other = EastMoneyCode::new("reverse_repo").expect("合法码");
    assert_eq!(code, other, "同值必须相等（避免静默不等值）");
}

/// `validate_observation_scope`：只放行宏观观测，其余产品路由他处。
#[test]
fn scope_guard_rejects_non_macro_products() {
    assert!(
        validate_observation_scope(EastMoneyKind::Observation, EastMoneyProduct::Macro).is_ok()
    );
    for product in [
        EastMoneyProduct::Quote,
        EastMoneyProduct::Forex,
        EastMoneyProduct::Search,
        EastMoneyProduct::Calendar,
    ] {
        let error = validate_observation_scope(EastMoneyKind::Observation, product)
            .expect_err("非宏观产品必须拒绝");
        assert_eq!(
            error.kind(),
            eastmoneyx::EastMoneyErrorKind::RoutedElsewhere
        );
    }
}

/// `validate_money_supply`：期间 / 频率 / revision / 取值完整性都要校验。
#[test]
fn money_supply_validation_rejects_incomplete_rows() {
    assert!(validate_money_supply(&sample_money_supply()).is_ok());

    let mut fabricated = sample_money_supply();
    fabricated.revision = Some(EastMoneyCode::new("v2").expect("合法码"));
    assert!(
        validate_money_supply(&fabricated).is_err(),
        "无官方 vintage 面时 revision 必须为 None"
    );

    let mut all_missing = sample_money_supply();
    all_missing.m2 = EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank);
    assert!(
        validate_money_supply(&all_missing).is_err(),
        "三项全缺必须拒绝"
    );

    let mut non_finite = sample_money_supply();
    non_finite.m2 = EastMoneyValue::Present(f64::NAN);
    assert!(validate_money_supply(&non_finite).is_err(), "NaN 必须拒绝");
}

/// `guard_parse_format`：JSONP 属未来任务，当前必须 `NotApplicable`。
#[test]
fn parse_format_guard_rejects_jsonp() {
    assert!(guard_parse_format(EastMoneyParseFormat::Csv).is_ok());
    let error = guard_parse_format(EastMoneyParseFormat::Jsonp).expect_err("JSONP 未实现");
    assert_eq!(error.kind(), eastmoneyx::EastMoneyErrorKind::NotApplicable);
}

/// `guard_live_execution`：七类执行通道全部拒绝（含代理轮换）。
#[test]
fn live_execution_guard_denies_every_channel() {
    for channel in [
        EastMoneyExecutionChannel::Network,
        EastMoneyExecutionChannel::UserAgentSpoofing,
        EastMoneyExecutionChannel::ProxyRotation,
        EastMoneyExecutionChannel::CookieOrChallenge,
        EastMoneyExecutionChannel::CaptchaBypass,
        EastMoneyExecutionChannel::HeadlessBrowser,
        EastMoneyExecutionChannel::JsonpClaim,
    ] {
        let error = guard_live_execution(channel).expect_err("当前一律禁止");
        assert_eq!(
            error.kind(),
            eastmoneyx::EastMoneyErrorKind::AuthorizationDenied
        );
    }
}

/// `authorize_eastmoney_access`：fail-closed（缺证据 / 范围不明 / 过期 → Denied）。
#[test]
fn authorize_is_fail_closed() {
    let today = Date::new(2026, 9, 22).expect("合法日期");
    assert!(matches!(
        authorize_eastmoney_access(None, today),
        EastMoneyAuthorization::Denied { .. }
    ));

    let vague = eastmoneyx::EastMoneyAuthorizationEvidence {
        signer: "owner".into(),
        scope: "everything".into(),
        expires_on: Date::new(2026, 12, 31).expect("合法日期"),
    };
    assert!(matches!(
        authorize_eastmoney_access(Some(&vague), today),
        EastMoneyAuthorization::Denied { .. }
    ));

    let expired = eastmoneyx::EastMoneyAuthorizationEvidence {
        signer: "owner".into(),
        scope: "kind=observation; product=macro".into(),
        expires_on: Date::new(2026, 1, 1).expect("合法日期"),
    };
    assert!(matches!(
        authorize_eastmoney_access(Some(&expired), today),
        EastMoneyAuthorization::Denied { .. }
    ));

    let complete = eastmoneyx::EastMoneyAuthorizationEvidence {
        signer: "owner".into(),
        scope: "kind=observation; product=macro".into(),
        expires_on: Date::new(2026, 12, 31).expect("合法日期"),
    };
    assert!(matches!(
        authorize_eastmoney_access(Some(&complete), today),
        EastMoneyAuthorization::Authorized { .. }
    ));
}

/// `current_eastmoney_authorization`：无 Owner 签核 ⇒ 恒为 Denied 且理由可读。
#[test]
fn current_authorization_is_always_denied() {
    match current_eastmoney_authorization() {
        EastMoneyAuthorization::Denied { reason } => {
            assert!(reason.contains("UNKNOWN"), "理由须说明端点许可 UNKNOWN");
            assert!(
                reason.contains("denied_pending_owner_reruling"),
                "理由须登记 live_proxy_access 状态"
            );
        }
        other => panic!("当前判定必须为 Denied，实际 {other:?}"),
    }
}

/// `parse_eastmoney_observations`：夹具可解析；重复身份与非法值必须拒绝。
#[test]
fn observations_parser_accepts_fixture_and_rejects_duplicates() {
    let csv = fixture_csv(include_str!("fixtures/money_supply_synthetic.json"));
    let parsed = parse_eastmoney_observations(&csv).expect("夹具解析成功");
    assert_eq!(parsed.len(), 3);
    assert_eq!(
        parsed[2].m2,
        EastMoneyValue::Present(3060.9),
        "空单元格之外的列必须原样读出"
    );
    assert_eq!(
        parsed[2].m0,
        EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank)
    );

    let duplicate = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,1,2,3,4,5,6,亿元\n\
money_supply,2026-08,1,2,4,4,5,7,亿元\n";
    assert!(
        parse_eastmoney_observations(duplicate).is_err(),
        "重复身份必须拒绝（不得静默去重）"
    );
}

/// `parse_eastmoney_interbank_rates`：夹具可解析；未知序列令牌必须拒绝。
#[test]
fn interbank_parser_accepts_fixture_and_rejects_unknown_series() {
    let csv = "period,series,rate,change_bp,unit\n\
2026-08-15,SHIBOR_ON,1.42,-3.5,%\n\
2026-08-15,DR007,1.55,2.0,%\n\
2026-08-20,LPR_1Y,3.00,0,%\n";
    let parsed = parse_eastmoney_interbank_rates(csv).expect("解析成功");
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].change_bp, EastMoneyValue::Present(-3.5));

    let unknown = "period,series,rate,change_bp,unit\n2026-08-15,USD1M,1.4,0,%\n";
    assert!(
        parse_eastmoney_interbank_rates(unknown).is_err(),
        "未知序列令牌必须拒绝"
    );
}

/// `parse_eastmoney_aggregate_financing`：表头多列 / 少列 / 改名一律原子失败。
#[test]
fn financing_parser_rejects_unknown_columns() {
    let ok = "series,period,increment,stock,yoy,unit\n\
aggregate_financing,2026-07,7700.0,412500.0,8.4,亿元\n";
    assert_eq!(
        parse_eastmoney_aggregate_financing(ok)
            .expect("解析成功")
            .len(),
        1
    );
    assert_eq!(
        parse_eastmoney_aggregate_financing(ok).expect("解析成功")[0],
        EastMoneyAggregateFinancing {
            series: EastMoneyCode::new("aggregate_financing").expect("合法码"),
            period: Period::Month {
                year: 2026,
                month: 7
            },
            increment: EastMoneyValue::Present(7700.0),
            stock: EastMoneyValue::Present(412500.0),
            yoy: EastMoneyValue::Present(8.4),
            unit: Unit::HundredMillionYuan,
            frequency: Frequency::Monthly,
            revision: None,
        }
    );

    let extra = "series,period,increment,stock,yoy,unit,note\n\
aggregate_financing,2026-07,7700.0,412500.0,8.4,亿元,x\n";
    assert!(
        parse_eastmoney_aggregate_financing(extra).is_err(),
        "未知列必须被拒绝（不得静默忽略）"
    );
}

/// `parse_eastmoney_northbound_flows`：日期严格；重复日期拒绝。
#[test]
fn northbound_parser_accepts_fixture_and_rejects_bad_date() {
    let ok = "date,sh_net,sz_net,total_net,balance,unit\n\
2026-08-15,1200.5,-300.2,900.3,21000.0,亿元\n";
    assert_eq!(
        parse_eastmoney_northbound_flows(ok)
            .expect("解析成功")
            .len(),
        1
    );

    let bad = "date,sh_net,sz_net,total_net,balance,unit\n\
2026-8-15,1200.5,-300.2,900.3,21000.0,亿元\n";
    assert!(
        parse_eastmoney_northbound_flows(bad).is_err(),
        "月/日必须补零"
    );

    let dup = "date,sh_net,sz_net,total_net,balance,unit\n\
2026-08-15,1,2,3,4,亿元\n\
2026-08-15,1,2,3,5,亿元\n";
    assert!(
        parse_eastmoney_northbound_flows(dup).is_err(),
        "重复日期必须拒绝"
    );
}

/// `parse_eastmoney_omo_operations`：夹具可解析；term_days 必须是非负整数。
#[test]
fn omo_parser_accepts_fixture_and_requires_integer_term_days() {
    let csv = fixture_csv(include_str!("fixtures/omo_operation_synthetic.json"));
    let parsed = parse_eastmoney_omo_operations(&csv).expect("夹具解析成功");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].term_days, 7);
    assert_eq!(parsed[0].frequency, Frequency::Event);

    let bad = "date,op_type,term_days,amount,rate,net_injection,unit\n\
2026-08-15,reverse_repo,7d,1,2,3,亿元\n";
    assert!(
        parse_eastmoney_omo_operations(bad).is_err(),
        "非整数 term_days 必须拒绝（不得静默当 0）"
    );
}

/// `eastmoney_publication_semantics`：三元组恒为 `Date` + `Inferred` + `NotEligible`。
#[test]
fn publication_triple_is_date_inferred_not_eligible() {
    assert_eq!(
        eastmoney_publication_semantics(),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        )
    );
}

/// `is_formal_pit_eligible`：恒为 `false`，不得静默升格。
#[test]
fn formal_pit_is_never_eligible() {
    assert!(!is_formal_pit_eligible());
    assert_eq!(
        eastmoney_publication_semantics().2,
        PitEligibility::NotEligible
    );
}
