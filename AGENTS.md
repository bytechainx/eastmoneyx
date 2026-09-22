# eastmoneyx Agent 指南

> 本文件为 AI Agent 在本仓库工作时的入口指南。

## 项目定位

东方财富（EastMoney）宏观观测的**离线源事实库**：类型化观测、严格 CSV 解析、fail-closed
授权判定与执行通道拒绝守卫。**不是**联网采集器；`production_decision = NO-GO`。

## 技术栈

- Rust edition 2021，rust-version 1.73（由依赖图推导，见 `CONTEXT.md`）
- 依赖：`csv` 1（CSV 解析）、`thiserror` 2（错误模型）；`serde_json` 1 仅作 dev-dependency（读合成夹具）
- 无 path 依赖；零业务耦合，不依赖 `kernel` / `contracts` 或任何 `bytechainx/*` crate
- crate 级 lint：`unwrap_used` / `expect_used` / `panic` / `unreachable` / `todo` / `unimplemented`
  全部 `deny`（测试代码经 `cfg_attr(test)` 豁免）

## 代码结构

```text
src/
├── lib.rs               # crate 文档 + 模块声明 + 门面 pub use
├── error.rs             # EastMoneyError / EastMoneyErrorKind / EastMoneyResult
├── value.rs             # Date / Period / Frequency / Unit / EastMoneyCode / 产品范围守卫
├── value/observation.rs # 5 类观测值对象 + EastMoneyValue + validate_money_supply
├── authz.rs             # 授权判定（fail-closed）+ 执行通道守卫
├── pit.rs               # publication 语义三元组
└── parse.rs             # 只实现 CSV 的离线解析器
```

模块依赖方向单向：`error ← value ← {authz, parse}`、`error ← pit`，无环。禁止 `mod.rs`，
禁止 `utils` / `helpers` / `common` / `manager` / `base` / `global` / `misc` 命名。

## 硬约束（违反即返工）

1. **禁止**任何 HTTP 客户端、异步运行时、`chrono` / `time`、`rand`；禁止读环境变量或凭据。
2. **禁止**在源码中出现任何 URL / 端点字面量（规划端点 MUST NOT 写入代码）。
3. **禁止**写入任何 `path = "../<其它仓>"` 依赖或依赖 `bytechainx/*`。
4. **禁止**在网络路径上实现采集；`guard_live_execution` 的七类通道必须保持全部拒绝。
5. **禁止**实现派生指标（净投放 5d / 中美利差 / 剪刀差 / z-score 等）。
6. **禁止**把 JSONP 剥离写成已实现（它是授权后的未来任务）。
7. **禁止**把 `tests/fixtures/` 的合成样本表述为「实测」「核验 PASS」或任何证据等级。
8. **禁止**在库代码里 `unwrap` / `expect` / `panic!` / `println!`。

## 门禁四件套

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo package --no-verify
```

## 相关文档

- 采集范围权威：`specs/adapter/eastmoney.md`（工作区根）
- API 文档：`docs/API.md`
- 能力标准与验收：`docs/标准.md`
- 术语与领域语言：`CONTEXT.md`
- 贡献指南：`CONTRIBUTING.md`
- 变更记录：`CHANGELOG.md`
- 基准测试：`benches/hot_path.rs`
