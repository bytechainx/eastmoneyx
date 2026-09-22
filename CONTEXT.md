# eastmoneyx 上下文

本文件定义 `eastmoneyx` 与使用方共享的核心词汇、边界与已知缺口。它只记录领域含义与能力边界，
不记录具体实现、存储或部署决定。

## 角色与边界

**源事实库**：把 `specs/adapter/eastmoney.md` 声明的采集范围落成稳定 Rust 类型的一层；
它只表达「源有哪些条目、取值是什么、单位是什么」，不承载采集、存储、分发与派生计算。
_Avoid_: 采集器（本域处于 offline/NO-GO，联网属当前禁止项）

**当前合同**：`kind=observation` + `product=macro` 的**脱敏宏观观测**。行情 / 汇率 / 搜索 / 日历
不属于本域，必须拒绝。
_Avoid_: 数据源适配器（本 crate 无 `XxxConfig` / 连接池 / ping 面，不是适配器范式）

**fail-closed 判定**：证据缺失、空、签署者不明、覆盖范围不明或已过期 → 一律 `Denied`。
判定的对象是「该源的某个范围是否被授权访问」，**不是**「本库是否生产就绪」。
_Avoid_: 授权状态变更（判定是只读结论，MUST NOT 改写清单的原登记值）

**执行纪律**：联网 / UA 伪装 / 代理轮换 / Cookie-Challenge / 验证码 / 无头浏览器 / JSONP 假宣称
在当前一律禁止；`proxy_support_target = required` 只是技术接线闭集要求，**不等于**联网授权。
_Avoid_: 代理池（MUST NOT 以代理池绕过当前禁止）

## 时间与量纲

**期间身份**：`Period` 只表达「日 / 月 / 季 / 年 / 事件」的身份，不含时区、算术或格式化。
_Avoid_: 时间戳（本层不引入日期库，也不补造 `00:00 UTC` 之类的时刻）

**源侧单位**：`Unit` 保留源单位（`%` / `bp` / `亿元` / `元`）；换算归下游 Normalize。
观测级 `unit` 适用于该类型的量值列；`change_bp` 恒为基点、`*_yoy` 恒为百分比（清单语义钉死）。
_Avoid_: 归一化单位（在源层做换算会让「源事实」失去可追溯性）

**具名缺失**：`EastMoneyValue::Missing(reason)` 明确区分「源侧空」「尚未发布」「不适用」。
_Avoid_: 补 0 / 补前值（静默填充会伪造源事实）

## 解析

**严格表头**：列数与列名逐字相等；多一列、少一列、改名、改序一律原子失败。
_Avoid_: 宽容解析（静默忽略未知字段正是缺陷的来源）

**重复身份拒绝**：本层选择**拒绝**而非静默去重，身份按类型定义（见 `docs/标准.md` §3）。
_Avoid_: 后写覆盖（覆盖会让「哪一条是源事实」不可判定）

## rust-version 推导

规则：`rust-version` = 依赖图中所有依赖所声明 `rust_version` 的最大值
（`cargo metadata --format-version 1` 逐包读取）。

| 依赖 | 版本 | 其 `rust_version` |
| --- | --- | --- |
| `csv` | 1.4.0 | **1.73** |
| `csv-core` | 0.1.13 | （缺失，见下） |
| `itoa` | 1.0.18 | 1.68 |
| `memchr` | 2.8.3 | 1.61 |
| `serde` / `serde_core` | 1.0.229 | 1.56 |
| `serde_derive` / `proc-macro2` / `quote` / `syn` | 1.0.229 / 1.0.107 / 1.0.47 / 3.0.6 | 1.71 |
| `serde_json`（dev） | 1.0.151 | 1.71 |
| `ryu` / `unicode-ident` / `zmij` | 1.0.23 / 1.0.26 / 1.0.23 | 1.71 |
| `thiserror` / `thiserror-impl` | 2.0.20 | 1.71 |

**最大值 = 1.73**，故本 crate 声明 `rust-version = "1.73"`（不得照抄兄弟仓）。

`csv-core` 未声明 `rust_version`：它是 `csv` 的私有实现细节，无语义上的独立下界；
`csv` 自身的 1.73 已支配该值，故按「保守取依赖链上的最大值」处理，不影响结论。

## 已知缺口

1. **未实现采集**：本域授权为 `unknown`、`live_proxy_access = denied_pending_owner_reruling`，
   联网被 `guard_live_execution` 无条件拒绝；本轮不实现任何采集路径。
2. **未实现 JSONP 剥离**：清单把 JSONP 剥离列为授权后的未来任务，现有磁盘样本全为 CSV；
   `guard_parse_format` 对 JSONP 返回 `NotApplicable`，**不得**被表述为已实现。
3. **未实现派生指标**：资金面松紧度 / 净投放 5d / 中美利差 / 北向动量 / M2−社融剪刀差 /
   流动性综合指数全部归 analytics。
4. **规划端点不落代码**：规划端点标识符（清单 §1.2）按约定 MUST NOT 写入代码，
   本库只登记其**类别语义名**，不含任何端点字面量。
5. **无官方 vintage 面**：`revision` 恒为 `None`；若日后接入官方 vintage，须同步更新清单与
   契约后再改本库，**禁止静默升格**。
6. **夹具为合成样本**：`tests/fixtures/` 全部为自拟样本，不是真实源数据，不构成任何证据。
