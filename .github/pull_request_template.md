## 变更说明

<!-- 简要描述本次 PR 做了什么，以及为什么 -->

## 变更类型

<!-- 勾选适用的类型 -->

- [ ] 新功能（feat）
- [ ] Bug 修复（fix）
- [ ] 重构（refactor）
- [ ] 性能优化（perf）
- [ ] 文档更新（docs）
- [ ] CI/CD 变更（ci）
- [ ] 其他（chore）

## 测试用例

<!-- 必填：本次变更有哪些测试覆盖？ -->

- [ ] 已添加白盒测试（单元测试 `#[cfg(test)]`）
- [ ] 已添加黑盒测试（集成测试 `tests/` 目录）
- [ ] 已更新已有测试用例
- [ ] 本次变更为文档/配置，无需测试

### 测试说明

<!-- 列出新增的测试函数名和覆盖的场景 -->

```
例如：
- test_deduct_stock_with_reserved: 有预留时扣减不超过可用量
- test_auth_middleware_accepts_valid_token: 有效 JWT 返回 200
```

## 检查清单

- [ ] `cargo fmt --all -- --check` 通过
- [ ] `cargo clippy --workspace -- -D warnings` 通过
- [ ] `cargo test --workspace` 全部通过
- [ ] 新增的 .rs 文件都有对应的测试
- [ ] 无敏感信息（密码、密钥）硬编码
- [ ] 已更新相关文档（如需要）

## 关联 Issue

<!-- 如有关联的 issue，填写编号 -->

Closes #
