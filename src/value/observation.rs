//! eastmoneyx 的观测值对象与类型化取值。
//!
//! 覆盖清单 §1.3 的 5 类规划类型：`MoneySupply` / `InterbankRate` /
//! `AggregateFinancing` / `NorthboundFlow` / `OmoOperation`。
//!
//! **单位口径**（本文件所有观测类型一致）：`unit` 字段适用于该类型的**量值列**
//! （货币供应量 / 利率 / 社融规模 / 净买入与余额 / 操作量与净投放）；清单已按语义钉死
//! 次要列的单位 —— `change_bp` 恒为基点、`*_yoy` 恒为百分比 —— 故不另设列，
//! 也不做任何换算（换算归下游 Normalize）。

use crate::error::{EastMoneyError, EastMoneyResult};
use crate::value::{Date, EastMoneyCode, Frequency, Period, Unit};

// ---------------------------------------------------------------------------
// 类型与序列
// ---------------------------------------------------------------------------

/// 清单 §1.3 的 5 类规划类型。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EastMoneySeriesKind {
    /// 货币供应量（m0/m1/m2 及同比）。
    MoneySupply,
    /// 同业利率（SHIBOR_ON / DR007 / LPR_1Y）。
    InterbankRate,
    /// 社融（增量 / 存量 / 同比）。
    AggregateFinancing,
    /// 北向资金（sh/sz/total 净买入与余额）。
    NorthboundFlow,
    /// 公开市场操作（op_type / term_days / amount / rate / net_injection）。
    OmoOperation,
}

/// 5 类规划类型的全集（顺序与清单 §1.3 一致）。
pub const SERIES_KINDS: [EastMoneySeriesKind; 5] = [
    EastMoneySeriesKind::MoneySupply,
    EastMoneySeriesKind::InterbankRate,
    EastMoneySeriesKind::AggregateFinancing,
    EastMoneySeriesKind::NorthboundFlow,
    EastMoneySeriesKind::OmoOperation,
];

/// 同业利率序列（清单 §1.3 钉死的三态）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EastMoneyInterbankSeries {
    /// 隔夜 SHIBOR。
    ShiborOn,
    /// 银行间存款类机构 7 天质押式回购加权利率。
    Dr007,
    /// 1 年期贷款市场报价利率。
    Lpr1y,
}

impl EastMoneyInterbankSeries {
    /// 解析源侧序列令牌（大小写敏感，避免静默等值）。
    pub(crate) fn parse(token: &str) -> EastMoneyResult<Self> {
        match token {
            "SHIBOR_ON" => Ok(Self::ShiborOn),
            "DR007" => Ok(Self::Dr007),
            "LPR_1Y" => Ok(Self::Lpr1y),
            other => Err(EastMoneyError::SemanticallyRejected(format!(
                "未知的同业利率序列令牌（长度 {}）",
                other.len()
            ))),
        }
    }

    /// 序列的规范标签。
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::ShiborOn => "SHIBOR_ON",
            Self::Dr007 => "DR007",
            Self::Lpr1y => "LPR_1Y",
        }
    }
}

// ---------------------------------------------------------------------------
// 取值
// ---------------------------------------------------------------------------

/// 缺失的**具名**原因。MUST NOT 静默把缺失转成 0。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EastMoneyMissingReason {
    /// 源侧该单元格为空。
    SourceBlank,
    /// 源侧尚未发布该期。
    NotPublishedYet,
    /// 该指标对该期间不适用。
    NotApplicable,
}

/// 一个源侧取值。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EastMoneyValue {
    /// 源侧给出的数值（f64 原样保留，未做任何换算）。
    Present(f64),
    /// 缺失，并携带具名原因。
    Missing(EastMoneyMissingReason),
}

impl EastMoneyValue {
    /// 是否为已给出的数值。
    pub(crate) fn is_present(self) -> bool {
        matches!(self, Self::Present(_))
    }

    /// 校验取值本身合法（`Present` 必须是有限数）。
    fn check(self) -> EastMoneyResult<()> {
        match self {
            Self::Present(v) if v.is_finite() => Ok(()),
            Self::Present(_) => Err(EastMoneyError::Invalid(
                "取值不得为 NaN 或无穷（缺失须用具名原因表达）".into(),
            )),
            Self::Missing(_) => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// 5 类观测
// ---------------------------------------------------------------------------

/// 货币供应量观测（`MoneySupply`）。
#[derive(Debug, Clone, PartialEq)]
pub struct EastMoneyMoneySupply {
    /// 源条目标识（规划类别的语义名，**不是**端点 URL）。
    pub series: EastMoneyCode,
    /// 业务期间（月度）。
    pub period: Period,
    /// M0 余额。
    pub m0: EastMoneyValue,
    /// M1 余额。
    pub m1: EastMoneyValue,
    /// M2 余额。
    pub m2: EastMoneyValue,
    /// M0 同比（单位恒为百分比）。
    pub m0_yoy: EastMoneyValue,
    /// M1 同比（单位恒为百分比）。
    pub m1_yoy: EastMoneyValue,
    /// M2 同比（单位恒为百分比）。
    pub m2_yoy: EastMoneyValue,
    /// 源侧单位，适用于 m0/m1/m2（保留不换算）。
    pub unit: Unit,
    /// 频率。
    pub frequency: Frequency,
    /// 修订标识：本域无官方 vintage 面 ⇒ 恒为 `None`，MUST NOT 伪造。
    pub revision: Option<EastMoneyCode>,
}

/// 同业利率观测（`InterbankRate`）。
#[derive(Debug, Clone, PartialEq)]
pub struct EastMoneyInterbankRate {
    /// 序列（三态之一）。
    pub series: EastMoneyInterbankSeries,
    /// 业务期间。
    pub period: Period,
    /// 利率（单位恒为百分比）。
    pub rate: EastMoneyValue,
    /// 变动（单位恒为基点）。
    pub change_bp: EastMoneyValue,
    /// 源侧单位，适用于 `rate`（保留不换算）。
    pub unit: Unit,
    /// 频率。
    pub frequency: Frequency,
    /// 修订标识（恒为 `None`）。
    pub revision: Option<EastMoneyCode>,
}

/// 社融观测（`AggregateFinancing`）。
#[derive(Debug, Clone, PartialEq)]
pub struct EastMoneyAggregateFinancing {
    /// 源条目标识。
    pub series: EastMoneyCode,
    /// 业务期间（月度）。
    pub period: Period,
    /// 增量（Credit Impulse 的**输入**；派生在 analytics，本层不计算）。
    pub increment: EastMoneyValue,
    /// 存量。
    pub stock: EastMoneyValue,
    /// 同比（单位恒为百分比）。
    pub yoy: EastMoneyValue,
    /// 源侧单位，适用于 increment / stock（保留不换算）。
    pub unit: Unit,
    /// 频率。
    pub frequency: Frequency,
    /// 修订标识（恒为 `None`）。
    pub revision: Option<EastMoneyCode>,
}

/// 北向资金观测（`NorthboundFlow`）。
#[derive(Debug, Clone, PartialEq)]
pub struct EastMoneyNorthboundFlow {
    /// 交易日。
    pub date: Date,
    /// 沪股通净买入。
    pub sh_net: EastMoneyValue,
    /// 深股通净买入。
    pub sz_net: EastMoneyValue,
    /// 合计净买入。
    pub total_net: EastMoneyValue,
    /// 余额。
    pub balance: EastMoneyValue,
    /// 源侧单位（保留不换算）。
    pub unit: Unit,
    /// 频率（日频）。
    pub frequency: Frequency,
    /// 修订标识（恒为 `None`）。
    pub revision: Option<EastMoneyCode>,
}

/// 公开市场操作观测（`OmoOperation`）。
#[derive(Debug, Clone, PartialEq)]
pub struct EastMoneyOmoOperation {
    /// 操作日。
    pub date: Date,
    /// 操作类型（取值域由源决定，本层不虚构全集）。
    pub op_type: EastMoneyCode,
    /// 期限（天）。
    pub term_days: u32,
    /// 操作量。
    pub amount: EastMoneyValue,
    /// 操作利率（单位恒为百分比）。
    pub rate: EastMoneyValue,
    /// 净投放（清单口径的**源侧字段**；派生指标「净投放 5d」在 analytics）。
    pub net_injection: EastMoneyValue,
    /// 源侧单位，适用于 amount / net_injection（保留不换算）。
    pub unit: Unit,
    /// 频率（事件频）。
    pub frequency: Frequency,
    /// 修订标识（恒为 `None`）。
    pub revision: Option<EastMoneyCode>,
}

// ---------------------------------------------------------------------------
// 校验
// ---------------------------------------------------------------------------

/// 校验一条货币供应量观测的完整性。
///
/// 检查项：期间与频率必须是月度；`revision` 必须为 `None`（本域无官方 vintage 面）；
/// m0/m1/m2 至少一项有取值；所有取值必须合法（有限数或具名缺失）。
///
/// # Errors
///
/// 期间/频率不符、`revision` 被伪造、三项余额全缺、或取值非法时返回
/// [`EastMoneyError::Invalid`] 或 [`EastMoneyError::Missing`]。
///
/// # Examples
///
/// ```
/// use eastmoneyx::{
///     validate_money_supply, EastMoneyCode, EastMoneyMissingReason, EastMoneyMoneySupply,
///     EastMoneyValue, Frequency, Period, Unit,
/// };
///
/// let observation = EastMoneyMoneySupply {
///     series: EastMoneyCode::new("money_supply")?,
///     period: Period::parse("2026-08")?,
///     m0: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
///     m1: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
///     m2: EastMoneyValue::Present(300.5),
///     m0_yoy: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
///     m1_yoy: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
///     m2_yoy: EastMoneyValue::Present(8.1),
///     unit: Unit::HundredMillionYuan,
///     frequency: Frequency::Monthly,
///     revision: None,
/// };
/// assert!(validate_money_supply(&observation).is_ok());
/// # Ok::<(), eastmoneyx::EastMoneyError>(())
/// ```
pub fn validate_money_supply(observation: &EastMoneyMoneySupply) -> EastMoneyResult<()> {
    if !matches!(observation.period, Period::Month { .. }) {
        return Err(EastMoneyError::Invalid(
            "货币供应量观测的业务期间必须是月度（YYYY-MM）".into(),
        ));
    }
    if observation.frequency != Frequency::Monthly {
        return Err(EastMoneyError::Invalid(
            "货币供应量观测的频率必须是 Monthly".into(),
        ));
    }
    if observation.revision.is_some() {
        return Err(EastMoneyError::SemanticallyRejected(
            "本域无官方 vintage 面，revision 必须为 None（MUST NOT 伪造）".into(),
        ));
    }
    if !(observation.m0.is_present() || observation.m1.is_present() || observation.m2.is_present())
    {
        return Err(EastMoneyError::Missing(
            "m0/m1/m2 至少一项须有取值（否则该行不承载任何源事实）".into(),
        ));
    }
    for value in [
        observation.m0,
        observation.m1,
        observation.m2,
        observation.m0_yoy,
        observation.m1_yoy,
        observation.m2_yoy,
    ] {
        value.check()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(m2: EastMoneyValue) -> EastMoneyMoneySupply {
        EastMoneyMoneySupply {
            series: EastMoneyCode::new("money_supply").expect("合法码"),
            period: Period::Month {
                year: 2026,
                month: 8,
            },
            m0: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
            m1: EastMoneyValue::Missing(EastMoneyMissingReason::NotPublishedYet),
            m2,
            m0_yoy: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
            m1_yoy: EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank),
            m2_yoy: EastMoneyValue::Present(8.1),
            unit: Unit::HundredMillionYuan,
            frequency: Frequency::Monthly,
            revision: None,
        }
    }

    #[test]
    fn valid_money_supply_passes() {
        assert!(validate_money_supply(&sample(EastMoneyValue::Present(300.5))).is_ok());
    }

    #[test]
    fn money_supply_rejects_non_monthly_period_and_frequency() {
        let mut observation = sample(EastMoneyValue::Present(1.0));
        observation.period = Period::Year(2026);
        assert!(validate_money_supply(&observation).is_err());

        let mut observation = sample(EastMoneyValue::Present(1.0));
        observation.frequency = Frequency::Daily;
        assert!(validate_money_supply(&observation).is_err());
    }

    #[test]
    fn money_supply_rejects_fabricated_revision() {
        let mut observation = sample(EastMoneyValue::Present(1.0));
        observation.revision = Some(EastMoneyCode::new("v2").expect("合法码"));
        let error = validate_money_supply(&observation).expect_err("伪造 vintage 必须拒绝");
        assert_eq!(
            error.kind(),
            crate::EastMoneyErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn money_supply_requires_at_least_one_amount() {
        let observation = sample(EastMoneyValue::Missing(EastMoneyMissingReason::SourceBlank));
        let error = validate_money_supply(&observation).expect_err("三项全缺必须拒绝");
        assert_eq!(error.kind(), crate::EastMoneyErrorKind::Missing);
    }

    #[test]
    fn money_supply_rejects_non_finite_value() {
        let error = validate_money_supply(&sample(EastMoneyValue::Present(f64::NAN)))
            .expect_err("NaN 拒绝");
        assert_eq!(error.kind(), crate::EastMoneyErrorKind::Invalid);
    }

    #[test]
    fn interbank_series_token_is_case_sensitive() {
        assert!(matches!(
            EastMoneyInterbankSeries::parse("DR007"),
            Ok(EastMoneyInterbankSeries::Dr007)
        ));
        assert!(EastMoneyInterbankSeries::parse("dr007").is_err());
        assert_eq!(EastMoneyInterbankSeries::Dr007.label(), "DR007");
    }

    #[test]
    fn series_kind_catalog_has_five_entries() {
        assert_eq!(SERIES_KINDS.len(), 5);
        assert_eq!(
            SERIES_KINDS[0],
            EastMoneySeriesKind::MoneySupply,
            "顺序须与清单 §1.3 一致"
        );
    }
}
