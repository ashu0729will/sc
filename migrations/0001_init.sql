CREATE TABLE IF NOT EXISTS users (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    username VARCHAR(50) NOT NULL,
    display_name VARCHAR(80) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role ENUM('admin', 'seller') NOT NULL DEFAULT 'seller',
    status ENUM('pending', 'approved', 'rejected', 'disabled') NOT NULL DEFAULT 'pending',
    rejection_reason VARCHAR(255) NULL,
    approved_by BIGINT UNSIGNED NULL,
    approved_at DATETIME(6) NULL,
    last_login_at DATETIME(6) NULL,
    login_count INT UNSIGNED NOT NULL DEFAULT 0,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    UNIQUE KEY uq_users_username (username),
    KEY idx_users_status_created (status, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS sessions (
    token_hash CHAR(64) NOT NULL,
    user_id BIGINT UNSIGNED NOT NULL,
    ip VARCHAR(64) NOT NULL DEFAULT '',
    user_agent VARCHAR(255) NOT NULL DEFAULT '',
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    last_seen_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    expires_at DATETIME(6) NOT NULL,
    PRIMARY KEY (token_hash),
    KEY idx_sessions_user (user_id),
    KEY idx_sessions_seen (last_seen_at),
    CONSTRAINT fk_sessions_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS login_events (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    user_id BIGINT UNSIGNED NULL,
    username VARCHAR(50) NOT NULL,
    success BOOLEAN NOT NULL DEFAULT FALSE,
    reason VARCHAR(255) NULL,
    ip VARCHAR(64) NOT NULL DEFAULT '',
    user_agent VARCHAR(255) NOT NULL DEFAULT '',
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    KEY idx_login_events_created (created_at),
    KEY idx_login_events_user (user_id),
    CONSTRAINT fk_login_events_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS products (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    owner_id BIGINT UNSIGNED NOT NULL,
    name VARCHAR(120) NOT NULL,
    subtitle VARCHAR(180) NOT NULL DEFAULT '',
    description TEXT NOT NULL,
    category VARCHAR(40) NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    image_url VARCHAR(500) NOT NULL DEFAULT '',
    status ENUM('draft', 'active', 'offline') NOT NULL DEFAULT 'draft',
    stock INT UNSIGNED NOT NULL DEFAULT 0,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    PRIMARY KEY (id),
    KEY idx_products_owner_status (owner_id, status),
    KEY idx_products_category_status (category, status),
    CONSTRAINT fk_products_owner FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
