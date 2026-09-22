# eastmoneyx 公开 API

**角色**：东方财富（EastMoney）宏观观测的离线源事实类型与 CSV 解析
**source_id**：`eastmoney` | **production_decision**：`NO-GO`

## 公开消费面

### 错误

| 项 | 一行语义 |
| --- | --- |
| `EastMoneyError` | 本 crate 统一错误（`#[non_exhaustive]`，含 `kind()` / `is_retryable()`） |
| `EastMoneyErrorKind` | 八类反应分类（Invalid / Missing / AuthorizationDenied / RoutedElsewhere / WriteAuthorityDenied / SemanticallyRejected / NotApplicable / Invariant） |
| `EastMoneyResult<T>` | 结果别名 |

### 值对象

| 项 | 一行语义 |
| --- | --- |
| `Date::new` / `Date::parse` | 严格 ISO `YYYY-MM-DD` 的日期身份（含闰年校验） |
| `Period::parse` | `YYYY` / `YYYY-MM` / `YYYY-Qn` / `YYYY-MM-DD` 四种期间形态 |
| `Frequency` | `Daily` / `Weekly` / `Monthly` / `Quarterly` / `Annual` / `Event` / `Irregular` |
| `Unit` | 源侧单位（`Percent` / `BasisPoint` / `HundredMillionYuan` / `Yuan` / `Undeclared`） |
| `EastMoneyCode::new` / `as_str` | 源侧码（非空、无控制字符、长度受限；取值域由源决定） |
| `EastMoneyValue` / `EastMoneyMissingReason` | 取值二分：`Present(f64)` 或具名缺失 |
| `EastMoneySeriesKind` / `SERIES_KINDS` | 清单 §1.3 的 5 类规划类型与其全集 |
| `EastMoneyInterbankSeries` | 同业利率三序列（`ShiborOn` / `Dr007` / `Lpr1y`） |
| `EastMoneyMoneySupply` | 货币供应量观测（m0/m1/m2 及同比） |
| `EastMoneyInterbankRate` | 同业利率观测（rate / change_bp） |
| `EastMoneyAggregateFinancing` | 社融观测（增量 / 存量 / 同比） |
| `EastMoneyNorthboundFlow` | 北向资金观测（sh/sz/total 净买入与余额） |
| `EastMoneyOmoOperation` | 公开市场操作观测（op_type / term_days / amount / rate / net_injection） |
| `EastMoneyKind` / `EastMoneyProduct` | 条目类型与产品类别（含被拒的行情 / 汇率 / 搜索 / 日历） |

### 守卫与判定

| 项 | 一行语义 |
| --- | --- |
| `validate_observation_scope` | 只放行 `kind=observation` + `product=macro`，其余产品路由他处 |
| `validate_money_supply` | 校验货币供应量观测（月度、`revision` 必须为 `None`、至少一项取值） |
| `guard_parse_format` | 只放行 CSV；JSONP 属未来任务 → `NotApplicable` |
| `guard_live_execution` | 七类执行通道一律拒绝（含代理轮换） |
| `authorize_eastmoney_access` | 授权判定（fail-closed）；证据不完整 → `Denied` |
| `current_eastmoney_authorization` | 本域当前授权判定，恒为 `Denied` |
| `EastMoneyAuthorization` / `EastMoneyAuthorizationEvidence` | 判定结果与只读证据输入 |
| `EastMoneyExecutionChannel` | 被禁通道枚举 |
| `LIVE_PROXY_ACCESS` / `PROXY_SUPPORT_TARGET` | 代理接入状态轴（只读登记） |

### 解析入口

| 项 | 一行语义 |
| --- | --- |
| `parse_eastmoney_observations` | 货币供应量 CSV → `Vec<EastMoneyMoneySupply>` |
| `parse_eastmoney_interbank_rates` | 同业利率 CSV → `Vec<EastMoneyInterbankRate>` |
| `parse_eastmoney_aggregate_financing` | 社融 CSV → `Vec<EastMoneyAggregateFinancing>` |
| `parse_eastmoney_northbound_flows` | 北向资金 CSV → `Vec<EastMoneyNorthboundFlow>` |
| `parse_eastmoney_omo_operations` | 公开市场操作 CSV → `Vec<EastMoneyOmoOperation>` |
| `EastMoneyParseFormat` | 解析格式（`Csv` / `Jsonp`） |

### publication 语义

| 项 | 一行语义 |
| --- | --- |
| `TimePrecision` / `AvailabilityEvidence` / `PitEligibility` | 三元组枚举 |
| `eastmoney_publication_semantics` | 恒返回 `(Date, Inferred, NotEligible)` |
| `is_formal_pit_eligible` | 恒为 `false` |

## 安全与边界语义

- 零 HTTP 客户端、零端点字面量、零凭据读取、零跨仓 `path` 依赖。
- 授权判定 fail-closed：证据缺失 / 空 / 签署者不明 / 范围不明 / 过期 → 一律 `Denied`。
- 解析器只接受字符串；未知列、非法值、重复身份一律原子失败；错误消息不回显原始内容。
- 派生指标（净投放 5d / 中美利差 / 剪刀差 / 流动性综合指数）不在本库实现。

## 最小用法

```rust
use eastmoneyx::{parse_eastmoney_observations, validate_money_supply, EastMoneyValue, Unit};

let csv = "series,period,m0,m1,m2,m0_yoy,m1_yoy,m2_yoy,unit\n\
money_supply,2026-08,,,3060.9,,,8.1,亿元\n";
let observations = parse_eastmoney_observations(csv)?;
assert_eq!(observations[0].m2, EastMoneyValue::Present(3060.9));
assert_eq!(observations[0].unit, Unit::HundredMillionYuan);
validate_money_supply(&observations[0])?;
# Ok::<(), eastmoneyx::EastMoneyError>(())
```
