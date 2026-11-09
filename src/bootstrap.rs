use sqlx::MySqlPool;

use crate::auth::hash_password;

pub async fn bootstrap(
    pool: &MySqlPool,
    admin_username: &str,
    admin_password: &str,
    seed_demo_data: bool,
) -> anyhow::Result<()> {
    let admin_id = ensure_admin(pool, admin_username, admin_password).await?;

    if seed_demo_data {
        let demo_id =
            ensure_demo_user(pool, "demo_seller", "森野小铺", "Demo@123456", "approved").await?;

        let _waiting_id =
            ensure_demo_user(pool, "waiting_user", "待审核账号", "Wait@123456", "pending").await?;

        let product_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE owner_id = ?")
                .bind(demo_id)
                .fetch_one(pool)
                .await?;

        if product_count == 0 {
            seed_demo_products(pool, demo_id).await?;
        }
    }

    tracing::info!(admin_id, "account bootstrap completed");
    Ok(())
}

async fn ensure_admin(pool: &MySqlPool, username: &str, password: &str) -> anyhow::Result<u64> {
    if let Some(id) = user_id_by_username(pool, username).await? {
        return Ok(id);
    }

    let password_hash =
        hash_password(password).map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let result = sqlx::query(
        r#"
        INSERT INTO users
            (username, display_name, password_hash, role, status, approved_at)
        VALUES (?, ?, ?, 'admin', 'approved', UTC_TIMESTAMP(6))
        "#,
    )
    .bind(username)
    .bind("商城管理员")
    .bind(password_hash)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id())
}

async fn ensure_demo_user(
    pool: &MySqlPool,
    username: &str,
    display_name: &str,
    password: &str,
    status: &str,
) -> anyhow::Result<u64> {
    if let Some(id) = user_id_by_username(pool, username).await? {
        return Ok(id);
    }

    let password_hash =
        hash_password(password).map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let result = sqlx::query(
        r#"
        INSERT INTO users
            (username, display_name, password_hash, role, status, approved_at)
        VALUES (?, ?, ?, 'seller', ?, IF(? = 'approved', UTC_TIMESTAMP(6), NULL))
        "#,
    )
    .bind(username)
    .bind(display_name)
    .bind(password_hash)
    .bind(status)
    .bind(status)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id())
}

async fn user_id_by_username(pool: &MySqlPool, username: &str) -> anyhow::Result<Option<u64>> {
    Ok(
        sqlx::query_scalar::<_, u64>("SELECT id FROM users WHERE username = ? LIMIT 1")
            .bind(username)
            .fetch_optional(pool)
            .await?,
    )
}

async fn seed_demo_products(pool: &MySqlPool, owner_id: u64) -> anyhow::Result<()> {
    let products = [
        (
            "云朵降噪耳机",
            "轻量佩戴 · 40 小时续航",
            "柔软耳罩与自适应降噪，让通勤和专注时刻都更安静。",
            "数码",
            399.00_f64,
            "/images/headphones.svg",
            36_u64,
        ),
        (
            "暮色香氛蜡烛",
            "雪松与无花果 · 180g",
            "植物蜡手工灌制，木质烛芯燃烧时带有轻柔的噼啪声。",
            "家居",
            128.00_f64,
            "/images/candle.svg",
            52_u64,
        ),
        (
            "手作帆布托特包",
            "加厚帆布 · 双层内袋",
            "宽松容量与挺括轮廓兼顾，适合日常通勤和周末出行。",
            "穿搭",
            159.00_f64,
            "/images/tote.svg",
            24_u64,
        ),
        (
            "山野手冲咖啡",
            "中浅烘焙 · 柑橘花香",
            "精选高海拔产区咖啡豆，风味干净明亮，余韵带有蜂蜜甜感。",
            "食品",
            88.00_f64,
            "/images/coffee.svg",
            80_u64,
        ),
        (
            "轻量旅行水杯",
            "316 不锈钢 · 480ml",
            "单手开合与防漏锁扣设计，适合通勤、健身与短途旅行。",
            "户外",
            119.00_f64,
            "/images/bottle.svg",
            61_u64,
        ),
        (
            "木质桌面收纳",
            "白蜡木 · 模块化组合",
            "将文具、线材与小物归置整齐，保留原木自然纹理。",
            "家居",
            219.00_f64,
            "/images/desk.svg",
            18_u64,
        ),
        (
            "晨光机械键盘",
            "75% 配列 · 茶轴",
            "紧凑布局保留常用功能键，支持三模连接与柔和背光。",
            "数码",
            529.00_f64,
            "/images/keyboard.svg",
            29_u64,
        ),
        (
            "亚麻条纹抱枕",
            "天然亚麻 · 45 × 45cm",
            "低饱和条纹与柔软填充，为沙发和阅读角落增加松弛感。",
            "家居",
            99.00_f64,
            "/images/cushion.svg",
            45_u64,
        ),
    ];

    for (name, subtitle, description, category, price, image_url, stock) in products {
        sqlx::query(
            r#"
            INSERT INTO products
                (owner_id, name, subtitle, description, category, price, image_url, status, stock)
            VALUES (?, ?, ?, ?, ?, ?, ?, 'active', ?)
            "#,
        )
        .bind(owner_id)
        .bind(name)
        .bind(subtitle)
        .bind(description)
        .bind(category)
        .bind(price)
        .bind(image_url)
        .bind(stock)
        .execute(pool)
        .await?;
    }

    Ok(())
}
