# 项目记忆 · MORI 商城

## 规则

- 所有编码遵循 `ponytail` 技能原则：最小完整改动，不加没人要的抽象、配置和依赖；不省略校验、错误处理、安全、可访问性。回复末尾说明跳过了什么、有什么风险。
- 代码放在 `/home/chengzhen/桌面/教学/project/1/pj`。
- 每次发版更新 `README.md` 的版本号与功能清单（版本号与 `Cargo.toml` 保持一致）。
- 本地端口固定 **5050**（`PORT=5050`）。

## 技术栈

| 层 | 选型 |
|---|---|
| 后端 | Rust + axum 0.8 + sqlx 0.8 |
| 数据库 | MySQL 8.4（utf8mb4） |
| 前端 | JavaScript + TypeScript，原生 ES Module，无构建工具 |
| 客户端 | PC 端 + 安卓端（响应式布局） |

## 界面

- 左右分栏：左侧导航栏（分类 / 账号入口），右侧商品栏。
- 后台（商家工作台 + 管理后台）同样左侧导航、右侧内容。

## 功能范围

- 账号：注册 + 登录；注册后为 `pending`，管理员人工审核通过才能登录。
- 后台看板：账号登录情况、账号上架产品数。
- 商品：卖家创建 / 上下架；商城仅展示 `active`。

## 目录

```
src/          Rust 后端
migrations/   数据库迁移
frontend/     index.html + css/ + js/ + images/
```

## 内置账号（演示数据）

| 账号 | 密码 | 角色/状态 |
|---|---|---|
| admin | Admin@123456 | 管理员 |
| demo_seller | Demo@123456 | 卖家 / 已通过 |
| waiting_user | Wait@123456 | 卖家 / 待审核 |

## 运行

```bash
docker compose up -d
cargo run
# http://localhost:5050
```
