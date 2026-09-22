# eastmoneyx

`eastmoneyx` 把东方财富（EastMoney）源清单声明的**宏观观测源事实**落成独立、可编译、可测的
Rust 类型库：类型化观测、严格 CSV 解析、fail-closed 授权判定与执行通道拒绝守卫。

**这不是联网采集器。** 本库不含 HTTP 客户端、不含端点字面量、不读凭据、不配代理。

- 只做「源事实 + 离线解析 + 校验 + fail-closed 判定」
- 当前合同只承认 `kind=observation` + `product=macro`；行情 / 汇率 / 搜索 / 日历一律拒绝
- 授权现状 `unknown`：无 Owner 签核文件，全部端点许可 UNKNOWN，运行时必须 deny
- 零内部耦合：不依赖任何 `bytechainx/*` crate，无 `path` 依赖

`production_decision = NO-GO`（清单 COMPLETE ≠ ship，authorization ≠ Production Ready）。

## 安装

本 crate **不发布到 crates.io**，通过 git 依赖引入：

```toml
[dependencies]
eastmoneyx = { git = "https://github.com/bytechainx/eastmoneyx" }
```

本 crate 无 `path` 依赖，引入后可直接构建。

## 用法示例

解析一段脱敏 CSV 宏观观测并校验：

```rust
use eastmoneyx::{parse_eastmoney_observations, validate_money_supply, EastMoneyValue, Unit};

let csv = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,,,3060.9,,,8.1,亿元\n";

let observations = parse_eastmoney_observations(csv)?;
assert_eq!(observations.len(), 1);
assert_eq!(observations[0].m2, EastMoneyValue::Present(3060.9));
assert_eq!(observations[0].unit, Unit::HundredMillionYuan);
validate_money_supply(&observations[0])?;
# Ok::<(), eastmoneyx::EastMoneyError>(())
```

判定授权与拒绝执行通道：

```rust
use eastmoneyx::{
    current_eastmoney_authorization, guard_live_execution, EastMoneyAuthorization,
    EastMoneyExecutionChannel,
};

// 当前授权判定恒为 Denied（无 Owner 签核文件）
assert!(matches!(
    current_eastmoney_authorization(),
    EastMoneyAuthorization::Denied { .. }
));

// 七类执行通道当前一律禁止（含代理轮换）
assert!(guard_live_execution(EastMoneyExecutionChannel::ProxyRotation).is_err());
```

## 主要内容

| 类型 | 作用 |
| --- | --- |
| `EastMoneyMoneySupply` / `EastMoneyInterbankRate` / `EastMoneyAggregateFinancing` / `EastMoneyNorthboundFlow` / `EastMoneyOmoOperation` | 清单 §1.3 的 5 类规划类型 |
| `Date` / `Period` / `Frequency` / `Unit` / `EastMoneyCode` / `EastMoneyValue` | 期间、量纲与取值（缺失以具名原因表达） |
| `parse_eastmoney_*`（5 个入口） | 严格表头的 CSV 离线解析；未知列 / 非法值 / 重复身份一律原子失败 |
| `validate_observation_scope` / `validate_money_supply` | 产品范围守卫与观测完整性校验 |
| `authorize_eastmoney_access` / `current_eastmoney_authorization` | fail-closed 授权判定（只读） |
| `guard_live_execution` / `guard_parse_format` | 执行纪律与解析格式守卫 |
| `eastmoney_publication_semantics` | publication 三元组（恒为 `Date` + `Inferred` + `NotEligible`） |
| `EastMoneyError` | 统一错误模型（八类反应分类） |

## 非目标

- **不做联网采集**：本域 `live_proxy_access = denied_pending_owner_reruling`，联网 / UA 伪装 /
  代理轮换 / Cookie / 验证码 / 无头浏览器 / JSONP 假宣称全部禁止
- **不猜端点**：规划端点不等于访问合同，故本库不含任何端点字面量
- **不做派生指标**：资金面松紧度 / 净投放 5d / 中美利差 / 北向动量 / M2−社融剪刀差 /
  流动性综合指数全部归 analytics
- **不做存储与分发**，不成为应用的组合根
- **不替代授权治理**：判定只读，不改写清单的任何登记值

## 门禁

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo package --no-verify
```

三类测试（`tests/tdd_contracts.rs` / `tests/sdd_spec.rs` / `tests/aidd_boundary.rs`）全部离线运行，
不访问外网、不读环境变量。`tests/fixtures/` 下的夹具全部为**合成样本**，不是真实源数据，
不构成任何证据。

## 许可

MIT OR Apache-2.0
