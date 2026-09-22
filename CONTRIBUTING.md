# CONTRIBUTING.md — 贡献指南（eastmoneyx）

本文件面向贡献者，汇总本地门禁与提交约定。
AI Agent 的工作约定另见 [`AGENTS.md`](./AGENTS.md)；术语与领域语言见 [`CONTEXT.md`](./CONTEXT.md)。

## 开发流程

- 本仓库是**独立的单 crate 仓库**，不依赖任何 `bytechainx/*` crate，**没有任何 path 依赖**。
- substantial 变更走 feature branch → PR → review → merge，**禁止直接 push `main`**。
- `main` 已启用分支保护：要求 PR + 必需检查 `fmt / clippy / test`，
  `required_approving_review_count = 0`（单人也能合并），禁止强推与删除。
- 提交信息遵循 Conventional Commits（`feat:` / `fix:` / `docs:` / `ci:` / `chore:` / `refactor:`），
  描述用简体中文。

## 本地门禁（P0 四件套）

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features      # 不加 --all-targets，避免漏掉 doctest
cargo package --no-verify
```

本 crate 无 `[features]` 开关，`--all-features` 与不带等价；保留该写法是为了与工作区口径一致。
本 crate 无 path 依赖，`cargo package` 不需要额外的 `--config patch` 覆盖。

## 复用口径（不发布 crates.io）

- 本 crate **不发布到 crates.io**，仅以 GitHub 源码 / git 依赖形式复用。
- 文档与元数据中不得出现「可独立发布」「可直接 `cargo publish`」等表述，
  也不得放置 crates.io / docs.rs 徽章与外链。
- `Cargo.toml` 的 `documentation` 指向 `https://github.com/bytechainx/eastmoneyx#readme`。
- 消费方引入方式（README「安装」小节为准）：

  ```toml
  [dependencies]
  eastmoneyx = { git = "https://github.com/bytechainx/eastmoneyx" }
  ```

## 开发约定

- 注释、文档、错误消息使用**简体中文**；标识符保持英文。
- MSRV 为 Rust 1.73（由依赖图推导，见 `CONTEXT.md`），edition 2021。
- 错误模型统一为 `EastMoneyError`（thiserror 风格，`#[non_exhaustive]`）+ `EastMoneyErrorKind`；
  禁止公共 API 返回 `String` / `anyhow::Error`。
- 不在库代码里裸 `unwrap()` / `expect()` / `panic!` / `println!`
  （`[lints.clippy]` 已 `deny`；集成测试与 bench 各自带豁免属性）。
- 所有 `pub` 项必须有中文 `///` 文档（`missing_docs` 已 `deny`）。
- **零网络**：库代码 MUST NOT 引入 HTTP 客户端、异步运行时、日期库、随机数库，也不得读环境变量。
- **零端点字面量**：库代码 MUST NOT 出现任何 URL / 端点标识符（规划端点 ≠ 访问合同）。
- **零派生指标**：值对象里 MUST NOT 实现利差、净投放、剪刀差、z-score 等计算。
- 解析器 MUST NOT 接受 URL、客户端或凭据参数；未知列、非法值、重复身份必须原子失败，
  错误消息不得回显原始行内容。
- 授权判定 MUST 保持 fail-closed；`guard_live_execution` 的七类通道 MUST 保持全部拒绝。
- 单位 MUST 保留源侧口径；`MUST NOT` 在源层做换算。
- 集成测试**必须离线运行**；夹具 MUST 带 `_synthetic` 标注并声明为非证据。

## 提交前自检清单

- [ ] `cargo fmt --all -- --check` 通过
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` 通过
- [ ] `cargo test --all-features` 通过（含 doctest）
- [ ] `cargo package --no-verify` 通过
- [ ] 新增 `pub` 项都有中文 `///` 文档
- [ ] `docs/标准.md` 的 `##` 章节数与 `tests/sdd_spec.rs` 的 `// SPEC-MAP:` 行数相等
- [ ] `tests/tdd_contracts.rs` 的 `// TDD-PROBE:` 表覆盖本轮全部公开入口且四列无空
- [ ] 新增夹具带 `_synthetic` 标注，且未被表述为实测或核验通过
- [ ] 文档中无「可独立发布」/ crates.io / docs.rs 表述
