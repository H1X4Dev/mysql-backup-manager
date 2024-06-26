CREATE TABLE IF NOT EXISTS backups (
    uuid BINARY(16) PRIMARY KEY,
    base_uuid BINARY(16),
    type TINYINT UNSIGNED NOT NULL,
    path VARCHAR(255) NOT NULL,
    size BIGINT NOT NULL,
    created_at DATETIME NOT NULL
);