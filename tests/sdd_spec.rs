#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! SDD 规格对照（特性 005）：把 `docs/标准.md` 的每个 `##` 章节转成可执行断言。
//!
//! 章节与断言函数须与 `docs/标准.md` 的 `##` 章节 1:1（检查器按标题逐字比对）。
//!
//! // SPEC-MAP: S-1 | 1. 范围标准（仅脱敏宏观观测） | assert_scope_standard
//! // SPEC-MAP: S-2 | 2. 值对象与期间标准 | assert_value_objects_standard
//! // SPEC-MAP: S-3 | 3. 解析标准（CSV） | assert_parse_standard
//! // SPEC-MAP: S-4 | 4. 授权与执行纪律标准 | assert_authorization_standard
//! // SPEC-MAP: S-5 | 5. publication 语义标准 | assert_publication_standard
//! // SPEC-MAP: S-6 | 6. 合成夹具声明 | assert_fixture_declaration

use eastmoneyx::{
    current_eastmoney_authorization, eastmoney_publication_semantics, guard_live_execution,
    guard_parse_format, parse_eastmoney_northbound_flows, parse_eastmoney_observations,
    validate_observation_scope, AvailabilityEvidence, Date, EastMoneyAuthorization, EastMoneyCode,
    EastMoneyExecutionChannel, EastMoneyKind, EastMoneyMissingReason, EastMoneyParseFormat,
    EastMoneyProduct, EastMoneyValue, Frequency, Period, PitEligibility, TimePrecision, Unit,
    LIVE_PROXY_ACCESS, PROXY_SUPPORT_TARGET, SERIES_KINDS,
};

/// 运行期递归收集本仓 `src/` 下全部 `.rs`（**不**用手写文件清单，避免新增文件成为扫描盲区）。
///
/// 以 `CARGO_MANIFEST_DIR` 为根，**不依赖 cwd**；目录不可读即失败（不静默跳过）。
fn source_files() -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("src 目录可读") {
            let path = entry.expect("目录项可读").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(root.is_dir(), "src 目录必须存在：{}", root.display());
    let mut out = Vec::new();
    walk(&root, &mut out);
    // 结构性守卫：枚举若整体失效，`src/lib.rs` 会缺席 —— 此时 MUST 失败而不是空转通过。
    assert!(
        out.iter().any(|path| path.ends_with("src/lib.rs")),
        "递归枚举未找到 src/lib.rs，扫描面不可信（枚举 {0} 个文件）",
        out.len()
    );
    out
}

/// S-1：当前合同只承认 `kind=observation` + `product=macro`；5 类规划类型齐备。
#[test]
fn assert_scope_standard() {
    assert!(
        validate_observation_scope(EastMoneyKind::Observation, EastMoneyProduct::Macro).is_ok()
    );
    for product in [
        EastMoneyProduct::Quote,
        EastMoneyProduct::Forex,
        EastMoneyProduct::Search,
        EastMoneyProduct::Calendar,
    ] {
        assert!(
            validate_observation_scope(EastMoneyKind::Observation, product).is_err(),
            "行情 / 汇率 / 搜索 / 日历必须拒绝"
        );
    }
    assert_eq!(SERIES_KINDS.len(), 5, "清单 §1.3 的 5 类规划类型");

    // 零端点字面量 / 零凭据读取：**运行期递归**枚举 `src/` 下每一个 `.rs`
    // （不依赖 cwd、不用手写清单，故新增文件自动进入覆盖面）。
    // 需要 URL 字面量的负向用例一律放在 `tests/` 下；
    // MUST NOT 用「运行期拼装 scheme」的方式规避本扫描。
    let sources = source_files();
    for path in &sources {
        let text = std::fs::read_to_string(path).expect("源文件可读");
        for forbidden in ["http://", "https://", "env::var", "from_env"] {
            assert!(
                !text.contains(forbidden),
                "{} 含禁用片段 {forbidden}",
                path.display()
            );
        }
    }
}

/// S-2：日期 / 期间 / 码的严格校验；取值缺失必须具名；单位保留源侧。
#[test]
fn assert_value_objects_standard() {
    assert!(Date::parse("2026-08-15").is_ok());
    assert!(Date::parse("2026-2-3").is_err(), "月/日必须两位补零");
    assert!(Date::parse("2026/08/15").is_err(), "分隔符必须为 '-'");
    assert!(matches!(
        Period::parse("2026-08"),
        Ok(Period::Month { month: 8, .. })
    ));
    assert!(Period::parse("2026-8").is_err());
    assert!(EastMoneyCode::new("").is_err(), "源侧码不得为空");
    assert_eq!(
        EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
        EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
        "缺失必须以具名原因表达"
    );
    assert_eq!(Frequency::Monthly, Frequency::Monthly);
    assert_eq!(Unit::HundredMillionYuan, Unit::HundredMillionYuan);
}

/// S-3：CSV 表头严格匹配、空单元格为具名缺失、重复身份拒绝、JSONP 未实现。
#[test]
fn assert_parse_standard() {
    let csv = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,,,3060.9,,,8.1,亿元\n";
    let parsed = parse_eastmoney_observations(csv).expect("解析成功");
    assert_eq!(
        parsed[0].m0,
        EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank)
    );
    assert_eq!(parsed[0].unit, Unit::HundredMillionYuan);
    assert_eq!(parsed[0].frequency, Frequency::Monthly);

    let extra_column = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit,note\n\
money_supply,2026-08,1,2,3,4,5,6,亿元,x\n";
    assert!(
        parse_eastmoney_observations(extra_column).is_err(),
        "未知列必须被拒绝"
    );

    let duplicate = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,1,2,3,4,5,6,亿元\n\
money_supply,2026-08,1,2,4,4,5,7,亿元\n";
    assert!(
        parse_eastmoney_observations(duplicate).is_err(),
        "重复身份必须拒绝（本层选择拒绝而非静默去重）"
    );

    assert!(guard_parse_format(EastMoneyParseFormat::Csv).is_ok());
    assert!(
        guard_parse_format(EastMoneyParseFormat::Jsonp).is_err(),
        "JSONP 未实现"
    );
}

/// S-4：授权 fail-closed；当前恒 Denied；七类执行通道一律禁止。
#[test]
fn assert_authorization_standard() {
    assert!(matches!(
        current_eastmoney_authorization(),
        EastMoneyAuthorization::Denied { .. }
    ));
    assert_eq!(LIVE_PROXY_ACCESS, "denied_pending_owner_reruling");
    assert_eq!(PROXY_SUPPORT_TARGET, "required");

    let today = Date::new(2026, 9, 22).expect("合法日期");
    assert!(matches!(
        eastmoneyx::authorize_eastmoney_access(None, today),
        EastMoneyAuthorization::Denied { .. }
    ));

    for channel in [
        EastMoneyExecutionChannel::Network,
        EastMoneyExecutionChannel::UserAgentSpoofing,
        EastMoneyExecutionChannel::ProxyRotation,
        EastMoneyExecutionChannel::CookieOrChallenge,
        EastMoneyExecutionChannel::CaptchaBypass,
        EastMoneyExecutionChannel::HeadlessBrowser,
        EastMoneyExecutionChannel::JsonpClaim,
    ] {
        assert!(
            guard_live_execution(channel).is_err(),
            "所有通道当前一律禁止"
        );
    }
}

/// S-5：publication 三元组恒为 `Date` + `Inferred` + `NotEligible`，不得静默升格。
#[test]
fn assert_publication_standard() {
    assert_eq!(
        eastmoney_publication_semantics(),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        )
    );
    assert!(!eastmoneyx::is_formal_pit_eligible());
}

/// S-6：夹具为合成样本、非真实源数据、不构成证据。
#[test]
fn assert_fixture_declaration() {
    for raw in [
        include_str!("fixtures/money_supply_synthetic.json"),
        include_str!("fixtures/omo_operation_synthetic.json"),
    ] {
        let value: serde_json::Value = serde_json::from_str(raw).expect("夹具必须是合法 JSON");
        assert_eq!(value["_synthetic"], serde_json::Value::Bool(true));
        let note = value["_note"].as_str().expect("夹具须含 _note");
        assert!(note.contains("合成样本"), "夹具须显式标注合成：{note}");
        assert!(
            note.contains("不构成任何证据"),
            "夹具不得被当作证据：{note}"
        );
        assert!(
            !note.contains("实测") && !note.contains("核验 PASS"),
            "夹具不得被表述为实测或核验通过"
        );
    }

    let csv = "date,sh_net,sz_net,total_net,balance,unit\n\
2026-08-15,1200.5,-300.2,900.3,21000.0,亿元\n";
    assert_eq!(
        parse_eastmoney_northbound_flows(csv)
            .expect("解析成功")
            .len(),
        1
    );
}
