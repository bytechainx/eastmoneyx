//! eastmoneyx 的授权判定（fail-closed）与执行通道拒绝守卫。
//!
//! 本模块只做**只读判定**：它不改写清单的任何登记值，也不构成任何 live 授权。
//! 判定的对象是「该源的某个范围是否被授权访问」，**不是**「本库是否生产就绪」——
//! 两者共存不矛盾（`production_decision` 恒为 `NO-GO`）。

use crate::error::{EastMoneyError, EastMoneyResult};
use crate::value::Date;

/// 平台代理接入状态（清单 §1.1 状态轴，只读登记）。
pub const LIVE_PROXY_ACCESS: &str = "denied_pending_owner_reruling";

/// 代理支持目标：技术接线闭集要求，**不等于**联网授权。
pub const PROXY_SUPPORT_TARGET: &str = "required";

/// 授权证据必须显式覆盖的条目类型令牌。
const REQUIRED_SCOPE_KIND: &str = "kind=observation";

/// 授权证据必须显式覆盖的产品令牌。
const REQUIRED_SCOPE_PRODUCT: &str = "product=macro";

/// 授权判定结果。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EastMoneyAuthorization {
    /// 证据有效且覆盖本次请求的范围与有效期。
    Authorized {
        /// 被覆盖的源 / 端点 / 模式 / 用途 / 有效期。
        scope: String,
    },
    /// 证据缺失 / 过期 / 签署者不明 / 覆盖范围不明。
    Denied {
        /// 可读的拒绝理由。
        reason: String,
    },
}

/// 授权证据（调用方提供的只读输入）。
///
/// 判定 MUST NOT 因「清单里写了 approved」而放行；证据必须自带签署者、
/// 覆盖范围与有效期三项，缺一即 `Denied`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EastMoneyAuthorizationEvidence {
    /// 签署者（Owner 身份标识）。空串视为签署者不明。
    pub signer: String,
    /// 被覆盖的范围描述，须显式声明 `kind=observation` 与 `product=macro`。
    pub scope: String,
    /// 证据有效期截止日（含当日）。
    pub expires_on: Date,
}

/// 授权判定（fail-closed）：证据缺失、空、签署者不明、覆盖范围不明或已过期 → 一律 `Denied`。
///
/// 本函数 MUST NOT 默认放行，也 MUST NOT 改写清单的原登记值。
///
/// # Examples
///
/// ```
/// use eastmoneyx::{authorize_eastmoney_access, Date, EastMoneyAuthorization};
///
/// // 证据缺失 → 一律 Denied
/// let verdict = authorize_eastmoney_access(None, Date::parse("2026-09-22")?);
/// assert!(matches!(verdict, EastMoneyAuthorization::Denied { .. }));
/// # Ok::<(), eastmoneyx::EastMoneyError>(())
/// ```
#[must_use]
pub fn authorize_eastmoney_access(
    evidence: Option<&EastMoneyAuthorizationEvidence>,
    today: Date,
) -> EastMoneyAuthorization {
    let Some(evidence) = evidence else {
        return denied("授权证据缺失：本域无 Owner 签核文件，全部端点许可 UNKNOWN");
    };
    if evidence.signer.trim().is_empty() {
        return denied("签署者不明：证据缺少可识别的 Owner 身份标识");
    }
    let scope = evidence.scope.trim();
    if scope.is_empty() {
        return denied("覆盖范围不明：证据未声明被覆盖的范围");
    }
    if !scope.contains(REQUIRED_SCOPE_KIND) || !scope.contains(REQUIRED_SCOPE_PRODUCT) {
        return denied("覆盖范围不明：证据须显式声明 kind=observation 与 product=macro");
    }
    if today > evidence.expires_on {
        return denied("授权证据已过期：有效期截止日早于当前日期");
    }
    EastMoneyAuthorization::Authorized {
        scope: scope.to_string(),
    }
}

/// 本域的**当前**授权判定。
///
/// 恒为 `Denied`：截至本版本无 Owner 签核文件，全部端点许可 UNKNOWN，
/// 且 `live_proxy_access = denied_pending_owner_reruling`。
#[must_use]
pub fn current_eastmoney_authorization() -> EastMoneyAuthorization {
    denied(concat!(
        "本域当前无 Owner 签核文件；全部端点许可 UNKNOWN；",
        "live_proxy_access=denied_pending_owner_reruling，运行时必须 deny"
    ))
}

/// 构造一个拒绝结论。
fn denied(reason: &str) -> EastMoneyAuthorization {
    EastMoneyAuthorization::Denied {
        reason: reason.to_string(),
    }
}

// ---------------------------------------------------------------------------
// 执行通道守卫
// ---------------------------------------------------------------------------

/// 当前执行禁止的通道类别（清单 §1.1 的禁止清单逐项落成枚举）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EastMoneyExecutionChannel {
    /// 联网访问。
    Network,
    /// User-Agent 伪装。
    UserAgentSpoofing,
    /// 代理轮换。
    ProxyRotation,
    /// Cookie / Challenge 响应。
    CookieOrChallenge,
    /// 验证码绕过。
    CaptchaBypass,
    /// 无头浏览器。
    HeadlessBrowser,
    /// JSONP 假宣称（宣称已实现 JSONP 剥离）。
    JsonpClaim,
}

impl EastMoneyExecutionChannel {
    /// 全部被禁通道的枚举成员名（用于测试与文档核对）。
    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Network => "联网访问",
            Self::UserAgentSpoofing => "User-Agent 伪装",
            Self::ProxyRotation => "代理轮换",
            Self::CookieOrChallenge => "Cookie/Challenge",
            Self::CaptchaBypass => "验证码绕过",
            Self::HeadlessBrowser => "无头浏览器",
            Self::JsonpClaim => "JSONP 假宣称",
        }
    }
}

/// 执行通道守卫：当前**一律拒绝**（fail-closed）。
///
/// 任何通道（含代理轮换）都不得以代理池绕过当前禁止；技术接线（
/// [`PROXY_SUPPORT_TARGET`]）不构成联网授权。
///
/// # Errors
///
/// 对本枚举的任一通道恒返回 [`EastMoneyError::AuthorizationDenied`]。
pub fn guard_live_execution(channel: EastMoneyExecutionChannel) -> EastMoneyResult<()> {
    Err(EastMoneyError::AuthorizationDenied(format!(
        "当前执行禁止 {}：本域处于 offline/NO-GO，不得以任何代理池或伪装路径绕过",
        channel.label()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> Date {
        Date::new(2026, 9, 22).expect("合法日期")
    }

    fn full_evidence(scope: &str, expires: Date) -> EastMoneyAuthorizationEvidence {
        EastMoneyAuthorizationEvidence {
            signer: "owner".into(),
            scope: scope.into(),
            expires_on: expires,
        }
    }

    #[test]
    fn grants_when_evidence_is_complete_and_covers_scope() {
        let evidence = full_evidence(
            "source=eastmoney; kind=observation; product=macro; expires=2026-12-31",
            Date::new(2026, 12, 31).expect("合法日期"),
        );
        let verdict = authorize_eastmoney_access(Some(&evidence), today());
        assert!(matches!(verdict, EastMoneyAuthorization::Authorized { .. }));
    }

    #[test]
    fn denies_when_evidence_is_absent() {
        assert!(matches!(
            authorize_eastmoney_access(None, today()),
            EastMoneyAuthorization::Denied { .. }
        ));
    }

    #[test]
    fn denies_when_signer_or_scope_is_unknown() {
        let blank_signer = EastMoneyAuthorizationEvidence {
            signer: "   ".into(),
            scope: "kind=observation; product=macro".into(),
            expires_on: Date::new(2026, 12, 31).expect("合法日期"),
        };
        assert!(matches!(
            authorize_eastmoney_access(Some(&blank_signer), today()),
            EastMoneyAuthorization::Denied { .. }
        ));

        let vague = full_evidence("all access", Date::new(2026, 12, 31).expect("合法日期"));
        assert!(matches!(
            authorize_eastmoney_access(Some(&vague), today()),
            EastMoneyAuthorization::Denied { .. }
        ));
    }

    #[test]
    fn denies_when_evidence_is_expired() {
        let expired = full_evidence(
            "kind=observation; product=macro",
            Date::new(2026, 1, 1).expect("合法日期"),
        );
        assert!(matches!(
            authorize_eastmoney_access(Some(&expired), today()),
            EastMoneyAuthorization::Denied { .. }
        ));
    }

    #[test]
    fn current_verdict_is_always_denied() {
        let verdict = current_eastmoney_authorization();
        assert!(matches!(
            verdict,
            EastMoneyAuthorization::Denied { ref reason } if !reason.is_empty()
        ));
        assert_eq!(LIVE_PROXY_ACCESS, "denied_pending_owner_reruling");
        assert_eq!(PROXY_SUPPORT_TARGET, "required");
    }

    #[test]
    fn every_execution_channel_is_blocked() {
        for channel in [
            EastMoneyExecutionChannel::Network,
            EastMoneyExecutionChannel::UserAgentSpoofing,
            EastMoneyExecutionChannel::ProxyRotation,
            EastMoneyExecutionChannel::CookieOrChallenge,
            EastMoneyExecutionChannel::CaptchaBypass,
            EastMoneyExecutionChannel::HeadlessBrowser,
            EastMoneyExecutionChannel::JsonpClaim,
        ] {
            let error = guard_live_execution(channel).expect_err("必须一律拒绝");
            assert_eq!(
                error.kind(),
                crate::EastMoneyErrorKind::AuthorizationDenied,
                "{} 必须落到授权拒绝",
                channel.label()
            );
        }
    }
}
