# 0004-2026-02-24-release-policy-adjustments

## 1. 背景

- 需要完成首发版本，并把发布产物范围收敛到仅 macOS 平台。
- 同时希望在保留规则约束的前提下，管理员在紧急场景可绕过规则。

## 2. 变更内容

- 发布工作流调整为仅构建 `macos-latest`：
  - 移除 Linux/Windows 构建与打包步骤
- 分支保护策略调整：
  - `main` 与 `release/*` 设为 `isAdminEnforced=false`（管理员可绕过）
  - 保留：
    - 必需状态检查
    - 线性历史
    - 禁止强推/删除
    - 审批要求（1 个 approval）

## 3. 关键决策与原因

- 当前产品只支持 macOS，先聚焦单平台发布，减少构建成本和发布噪音。
- 管理员可绕过用于应急，不代表日常绕过；常规流程仍走 PR + 检查。

## 4. 影响范围

- 影响发布资产形态与仓库治理策略，不影响运行时逻辑。

## 5. 验证与结果

- 新发布 workflow 成功完成。
- 旧的多平台发布 run 已取消，避免产生不符合策略的产物。

## 6. 关联记录

- Commit: `142f9ad7148453fd18609b70ab0b3fab1be9a37d`
- PR: `#1`（workspace 合并，推动后续发布）
- Tag: `v0.1.0`
- Release: `simple-term v0.1.0`
- Workflow Run:
  - `22335167543`（cancelled，旧多平台配置）
  - `22335259583`（success，macOS-only）

