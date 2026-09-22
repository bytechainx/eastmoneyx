//! eastmoneyx 的错误分类与错误类型。
//!
//! 分类按「调用方应如何反应」划分，使调用方无需字符串匹配即可分流
//! （授权拒绝 / 路由拒绝 / 越权写入 / 语义拒绝四类必须可区分）。

/// 错误分类：按「调用方应如何反应」划分。
///
/// 禁止用字符串匹配替代对本枚举的匹配。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EastMoneyErrorKind {
    /// 输入形态或取值非法（调用方应修数据，重试无意义）。
    Invalid,
    /// 缺少必需项。
    Missing,
    /// 授权判定未通过（fail-closed 落点）。
    AuthorizationDenied,
    /// 该产品不属于本域，已被路由规则拒绝。
    RoutedElsewhere,
    /// 越权写入（权威写入归他域）。
    WriteAuthorityDenied,
    /// 结构可解析但语义不被接受（如近义非同 ID、批内重复身份）。
    SemanticallyRejected,
    /// 尚未实现的规划能力。
    NotApplicable,
    /// 不变量被破坏（库内 bug 的信号）。
    Invariant,
}

/// eastmoneyx 错误。保留可区分的分类与来源链。
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum EastMoneyError {
    /// 输入非法。
    #[error("输入非法：{0}")]
    Invalid(String),
    /// 缺少必需项。
    #[error("缺少必需项：{0}")]
    Missing(String),
    /// 授权判定未通过。
    #[error("授权判定未通过：{0}")]
    AuthorizationDenied(String),
    /// 产品不属于本域。
    #[error("不属于本域，已路由他处：{0}")]
    RoutedElsewhere(String),
    /// 越权写入。
    #[error("越权写入被拒绝：{0}")]
    WriteAuthorityDenied(String),
    /// 语义拒绝。
    #[error("语义不被接受：{0}")]
    SemanticallyRejected(String),
    /// 规划能力未实现。
    #[error("规划能力尚未实现：{0}")]
    NotApplicable(String),
    /// 不变量被破坏。
    #[error("不变量被破坏：{0}")]
    Invariant(String),
}

impl EastMoneyError {
    /// 分类，供调用方按「如何反应」分流。
    #[must_use]
    pub fn kind(&self) -> EastMoneyErrorKind {
        match self {
            Self::Invalid(_) => EastMoneyErrorKind::Invalid,
            Self::Missing(_) => EastMoneyErrorKind::Missing,
            Self::AuthorizationDenied(_) => EastMoneyErrorKind::AuthorizationDenied,
            Self::RoutedElsewhere(_) => EastMoneyErrorKind::RoutedElsewhere,
            Self::WriteAuthorityDenied(_) => EastMoneyErrorKind::WriteAuthorityDenied,
            Self::SemanticallyRejected(_) => EastMoneyErrorKind::SemanticallyRejected,
            Self::NotApplicable(_) => EastMoneyErrorKind::NotApplicable,
            Self::Invariant(_) => EastMoneyErrorKind::Invariant,
        }
    }

    /// 是否值得重试。本层无网络，除 `Invariant` 外一律 `false`。
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self.kind(), EastMoneyErrorKind::Invariant)
    }
}

/// 本 crate 统一结果别名。
pub type EastMoneyResult<T> = Result<T, EastMoneyError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authz::{current_eastmoney_authorization, EastMoneyAuthorization};

    fn all_variants() -> Vec<EastMoneyError> {
        vec![
            EastMoneyError::Invalid("a".into()),
            EastMoneyError::Missing("b".into()),
            EastMoneyError::AuthorizationDenied("c".into()),
            EastMoneyError::RoutedElsewhere("d".into()),
            EastMoneyError::WriteAuthorityDenied("e".into()),
            EastMoneyError::SemanticallyRejected("f".into()),
            EastMoneyError::NotApplicable("g".into()),
            EastMoneyError::Invariant("h".into()),
        ]
    }

    #[test]
    fn kind_maps_every_variant_distinctly() {
        let kinds: Vec<EastMoneyErrorKind> =
            all_variants().iter().map(EastMoneyError::kind).collect();
        let unique: std::collections::HashSet<_> = kinds.iter().collect();
        assert_eq!(unique.len(), kinds.len(), "每个变体须映射到互不相同的分类");
    }

    #[test]
    fn only_invariant_is_retryable() {
        for error in all_variants() {
            let expected = error.kind() == EastMoneyErrorKind::Invariant;
            assert_eq!(error.is_retryable(), expected, "分类 {:?}", error.kind());
        }
    }

    #[test]
    fn display_is_non_empty_and_source_chain_present() {
        for error in all_variants() {
            assert!(!error.to_string().is_empty());
        }
        let as_std: &dyn std::error::Error = &EastMoneyError::Invalid("x".into());
        assert!(as_std.source().is_none());
    }

    #[test]
    fn authorization_reason_carries_no_credential_material() {
        let EastMoneyAuthorization::Denied { reason } = current_eastmoney_authorization() else {
            unreachable!("本域无 Owner 签核文件，判定必须为 Denied");
        };
        let lowered = reason.to_ascii_lowercase();
        for forbidden in [
            "cookie",
            "set-cookie",
            "user-agent",
            "captcha",
            "token",
            "password",
            "passphrase",
            "api-key",
            "authorization",
        ] {
            assert!(
                !lowered.contains(forbidden),
                "拒绝理由不得包含 {forbidden}：{reason}"
            );
        }
        assert!(!reason.is_empty(), "拒绝理由必须是可读的");
    }
}
