#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]

//! # eastmoneyx —— 东方财富宏观观测的离线源事实库
//!
//! 把 `specs/adapter/eastmoney.md` 声明的源事实落成**可编译、可测、可校验**的类型与守卫：
//! 类型化观测、严格 CSV 解析、fail-closed 授权判定与执行通道拒绝。
//!
//! ## 能力
//!
//! | 能力 | 状态 |
//! | --- | --- |
//! | 5 类规划类型（`MoneySupply` / `InterbankRate` / `AggregateFinancing` / `NorthboundFlow` / `OmoOperation`）的值对象 | 已实现 |
//! | 脱敏 **CSV** 宏观观测解析（严格表头、原子失败、重复身份拒绝） | 已实现 |
//! | 产品范围守卫（只放行 `kind=observation` + `product=macro`） | 已实现 |
//! | 授权判定（fail-closed）与执行通道拒绝（联网 / UA 伪装 / 代理轮换 / Cookie / 验证码 / 无头浏览器 / JSONP 假宣称） | 已实现 |
//! | publication 语义三元组（恒为 `Date` + `Inferred` + `NotEligible`） | 已实现 |
//! | 联网采集、JSONP 剥离、派生指标 | **未实现**（超出授权或归 analytics） |
//!
//! ## 责任边界
//!
//! - 本库**做**：源事实的类型化表达、离线解析与校验、fail-closed 判定。
//! - 本库**不做**：联网访问、凭据处理、代理配置、Cookie / 验证码 / 无头浏览器、
//!   存储与分发、派生指标计算。
//! - 零内部耦合：不依赖任何 `bytechainx/*` crate，无 `path` 依赖。
//!
//! ## 非目标
//!
//! - 不做联网采集器（本域 `live_proxy_access = denied_pending_owner_reruling`）
//! - 不猜端点（规划端点不等于访问合同，故本库不含任何端点字面量）
//! - 不算派生指标（资金面松紧度 / 净投放 5d / 中美利差 / 北向动量 / M2−社融剪刀差 /
//!   流动性综合指数全部归 analytics）
//! - 不替代授权治理：判定只读，不改写清单的任何登记值
//!
//! ## 诚实边界
//!
//! `production_decision = NO-GO`；清单 COMPLETE ≠ ship；authorization ≠ Production Ready。
//! 库内夹具全部为**合成样本**，不是真实源数据，不构成任何证据。
//!
//! # 最小示例
//!
//! ```
//! use eastmoneyx::{
//!     parse_eastmoney_observations, validate_money_supply, EastMoneyValue, Frequency, Unit,
//! };
//!
//! let csv = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
//! money_supply,2026-08,,,3060.9,,,8.1,亿元\n";
//! let observations = parse_eastmoney_observations(csv)?;
//!
//! assert_eq!(observations.len(), 1);
//! assert_eq!(observations[0].m2, EastMoneyValue::Present(3060.9));
//! assert_eq!(observations[0].unit, Unit::HundredMillionYuan);
//! assert_eq!(observations[0].frequency, Frequency::Monthly);
//! validate_money_supply(&observations[0])?;
//! # Ok::<(), eastmoneyx::EastMoneyError>(())
//! ```

pub mod authz;
pub mod error;
pub mod parse;
pub mod pit;
pub mod value;

pub use authz::{
    authorize_eastmoney_access, current_eastmoney_authorization, guard_live_execution,
    EastMoneyAuthorization, EastMoneyAuthorizationEvidence, EastMoneyExecutionChannel,
    LIVE_PROXY_ACCESS, PROXY_SUPPORT_TARGET,
};
pub use error::{EastMoneyError, EastMoneyErrorKind, EastMoneyResult};
pub use parse::{
    guard_parse_format, parse_eastmoney_aggregate_financing, parse_eastmoney_interbank_rates,
    parse_eastmoney_northbound_flows, parse_eastmoney_observations, parse_eastmoney_omo_operations,
    EastMoneyParseFormat,
};
pub use pit::{
    eastmoney_publication_semantics, is_formal_pit_eligible, AvailabilityEvidence, PitEligibility,
    TimePrecision,
};
pub use value::{
    validate_money_supply, validate_observation_scope, Date, EastMoneyAggregateFinancing,
    EastMoneyCode, EastMoneyInterbankRate, EastMoneyInterbankSeries, EastMoneyKind,
    EastMoneyMissingReason, EastMoneyMoneySupply, EastMoneyNorthboundFlow, EastMoneyOmoOperation,
    EastMoneyProduct, EastMoneySeriesKind, EastMoneyValue, Frequency, Period, Unit, SERIES_KINDS,
};
