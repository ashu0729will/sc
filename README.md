# MORI 商城

版本 **1.1.0**

Rust + MySQL + 原生 JS/TS 的商城示例：前台商品浏览，卖家商品管理，管理员账号审核与数据看板。左右分栏布局，适配 PC 与安卓浏览器。

## 功能

**商城前台**
- 左侧分类导航 + 右侧商品栅格，分类筛选、关键词搜索
- 仅展示「在售」商品，售罄/下架自动隐藏

**账号**
- 注册后为「待审核」，管理员人工审核通过后才能登录
- 登录 / 退出，会话 168 小时后过期（HttpOnly Cookie）
- 登录成功与失败都记录审计日志（原因、IP、UA）

**卖家工作台**
- 创建商品（名称、分类、价格、库存、副标题、描述、图片）
- 商品状态：草稿 / 在售 / 已下架

**管理员后台**
- 数据看板：账号总数、待审核、在线账号、今日登录、在售商品
- 近 7 日登录折线图、账号上架产品数、最近 20 条登录记录
- 账号管理：审核通过 / 驳回 / 停用（驳回与停用会立即踢下线）

## 技术栈

| 层 | 选型 |
|---|---|
| 后端 | Rust 2021 + axum 0.8 + sqlx 0.8 |
| 数据库 | MySQL 8.4（utf8mb4） |
| 认证 | Argon2 密码哈希 + SHA-256 会话令牌 |
| 前端 | 原生 ES Module + JSDoc 类型，`tsc` 做类型检查，无构建步骤 |
| 样式 | 单文件 CSS，极简留白风格，森野绿 #173F35 |

## 目录

```
src/            Rust 后端（main / config / routes / auth / models / error / bootstrap）
migrations/     SQL 迁移
frontend/
  index.html    三个视图：商城、后台、登录弹窗
  css/styles.css
  js/app.js     全部前端逻辑
  images/       商品占位图（SVG）
compose.yaml    MySQL 容器
```

## 运行

```bash
docker compose up -d      # 启动 MySQL（端口 3306）
cargo run                 # 自动迁移 + 初始化账号，监听 5050
```

打开 <http://localhost:5050>

前端类型检查（可选）：

```bash
cd frontend && npm install && npm run typecheck
```

## 账号

| 账号 | 密码 | 说明 |
|---|---|---|
| admin | Admin@123456 | 管理员 |
| demo_seller | Demo@123456 | 卖家，已通过审核 |
| waiting_user | Wait@123456 | 卖家，待审核 |

管理员账号由 `ADMIN_USERNAME` / `ADMIN_PASSWORD` 决定，首次启动时创建。

## 配置

复制 `.env.example` 为 `.env`：

| 变量 | 默认 | 说明 |
|---|---|---|
| HOST / PORT | 0.0.0.0 / 5050 | 监听地址 |
| DATABASE_URL | mysql://mori:mori_password@127.0.0.1:3306/mori_shop | 数据库 |
| ADMIN_USERNAME / ADMIN_PASSWORD | admin / Admin@123456 | 初始管理员 |
| SESSION_TTL_HOURS | 168 | 会话有效期 |
| SESSION_COOKIE_SECURE | false | 生产环境走 HTTPS 时设为 true |
| SEED_DEMO_DATA | true | 是否写入演示卖家与商品 |

## API

统一前缀 `/api`，错误返回 `{ "message": "..." }`。

| 方法 | 路径 | 权限 | 说明 |
|---|---|---|---|
| GET | /health | 公开 | 健康检查 |
| GET | /storefront?category=&q= | 公开 | 在售商品 + 分类 |
| POST | /auth/register | 公开 | 注册（状态 pending） |
| POST | /auth/login | 公开 | 登录 |
| POST | /auth/logout | 公开 | 退出 |
| GET | /auth/me | 登录 | 当前用户 |
| GET | /seller/products | seller | 自己的商品 |
| POST | /seller/products | seller | 创建商品 |
| PATCH | /seller/products/{id}/status | seller | 改自己的商品状态 |
| GET | /admin/accounts | admin | 账号列表 |
| PATCH | /admin/accounts/{id}/status | admin | 审核 / 停用 |
| GET | /admin/dashboard | admin | 看板数据 |

## 版本记录

### 1.1.0
- 补齐前端：商城、卖家工作台、管理员后台三个视图与登录注册弹窗
- 新增看板样式：左侧统计列 + 右侧登录折线图与账号表
- 新增 8 张商品占位 SVG
- 前端加 `tsconfig.json` 与 `package.json`，支持 `tsc --checkJs` 类型检查
- 统一响应式布局，窄屏切换为顶部导航

### 1.0.0
- 后端骨架：账号注册登录、人工审核、商品管理、管理看板、登录审计
