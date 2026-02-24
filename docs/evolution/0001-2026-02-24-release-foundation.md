# 0001-2026-02-24-release-foundation

## 1. 背景

- 仓库需要建立可重复执行的发布机制，支持 SemVer、打包产物上传、自动生成 GitHub Release。

## 2. 变更内容

- 新增发布工作流：`.github/workflows/release.yml`
- 新增发布策略文档：`docs/release-strategy.md`
- 设计了两种触发方式：
  - `push` tag（`v*`）
  - `workflow_dispatch`（输入 `version` + `ref`）
- 发布流程包含：
  - 版本合法性校验
  - 与 `Cargo.toml` 工作区版本一致性校验
  - 打 tag / 构建 / 上传产物 / 生成 release notes

## 3. 关键决策与原因

- 采用 SemVer + `v` 前缀 tag，和 GitHub Release 生态兼容性最好。
- 把版本检查前置到 workflow，避免“代码版本与发布版本不一致”。

## 4. 影响范围

- 影响 CI/CD 与发布流程，不改变运行时业务逻辑。

## 5. 验证与结果

- 本地执行 `cargo check --workspace` 通过。
- workflow YAML 语法校验通过。

## 6. 关联记录

- Commit: `3e61c5b4817dc193bf262020f2668391101646da`
- PR: N/A（初始提交阶段）
- Tag: N/A
- Release: N/A
- Workflow Run: N/A

