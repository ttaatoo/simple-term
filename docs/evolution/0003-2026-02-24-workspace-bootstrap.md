# 0003-2026-02-24-workspace-bootstrap

## 1. 背景

- 需要让仓库具备可构建、可测试的 Rust workspace 主体内容，支撑 CI 与发布。

## 2. 变更内容

- 引入 workspace 根配置：
  - `Cargo.toml`
  - `Cargo.lock`
- 引入应用层 crate：`apps/simple-term`
- 引入核心库 crate：`crates/simple-term`
- 通过 PR 合并到 `main`。

## 3. 关键决策与原因

- 采用 `apps/simple-term` + `crates/simple-term` 结构，分离应用入口与核心逻辑，提升测试与扩展性。

## 4. 影响范围

- 新增项目核心代码与测试体系。
- 触发主干 CI 并成为后续发布基础。

## 5. 验证与结果

- 本地验证通过：fmt/check/clippy/test。
- PR 检查通过后合并。

## 6. 关联记录

- Commit: `f0e0b6a46ae30d4b3c3b8edfe017d17627ba1c28`
- PR: `#1`
- Tag: N/A
- Release: N/A
- Workflow Run: PR CI run `22334624523`（success）

