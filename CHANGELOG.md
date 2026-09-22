# Changelog — eastmoneyx

本文件记录 `eastmoneyx` 的用户可见变更，遵循 [Keep a Changelog](https://keepachangelog.com/)
与 [Semantic Versioning](https://semver.org/)。

本 crate 为特性 005（`005-macro-data-source-crates`）新建，不含从其它工程延续的版本线，
故从 `0.1.0` 起算。

## [0.1.1] - 2026-09-23

### 修正

- 修复 CSV 身份与表头被修剪、日频期间错配、公开期间及授权日期复验和授权范围精确匹配。实现向既有契约靠拢，按补丁版本推进。

## [Unreleased]

## [0.1.0] - 2026-09-22

### 新增

- 从 `specs/adapter/eastmoney.md` 的源事实落地为独立 crate：5 类规划类型的值对象
  （`EastMoneyMoneySupply` / `EastMoneyInterbankRate` / `EastMoneyAggregateFinancing` /
  `EastMoneyNorthboundFlow` / `EastMoneyOmoOperation`）。
- 期间与量纲值对象：`Date`（严格 ISO，含闰年校验）、`Period`（四种形态）、`Frequency`、
  `Unit`（保留源单位）、`EastMoneyCode`、`EastMoneyValue`（缺失以具名原因表达）。
- 产品范围守卫 `validate_observation_scope`：只放行 `kind=observation` + `product=macro`，
  行情 / 汇率 / 搜索 / 日历一律拒绝为路由他处。
- 离线 CSV 解析（5 个 `parse_eastmoney_*` 入口）：表头逐字严格匹配、未知列与非法值原子失败、
  重复身份显式拒绝（不静默去重）。
- fail-closed 授权判定 `authorize_eastmoney_access` 与当前判定 `current_eastmoney_authorization`
  （无 Owner 签核文件 ⇒ 恒为 `Denied`）。
- 执行通道守卫 `guard_live_execution`：联网 / User-Agent 伪装 / 代理轮换 / Cookie-Challenge /
  验证码 / 无头浏览器 / JSONP 假宣称一律拒绝。
- 解析格式守卫 `guard_parse_format`：只放行 CSV；JSONP 剥离属未来任务，返回 `NotApplicable`。
- publication 语义：`eastmoney_publication_semantics` 恒为 `(Date, Inferred, NotEligible)`。
- 三类测试：`tests/tdd_contracts.rs`（公开入口的 TDD-PROBE 变异探测表）、
  `tests/sdd_spec.rs`（`docs/标准.md` 全部 6 个 `##` 章节的可执行断言）、
  `tests/aidd_boundary.rs`（8 条经复核的 AI 生成对抗 / 边界用例）。
- 合成夹具 `tests/fixtures/*.json`（带 `_synthetic` 标注，非真实源数据）。
- 文档面：`README.md` / `docs/API.md` / `docs/标准.md` / `CONTEXT.md` / `CONTRIBUTING.md` / `AGENTS.md`。
