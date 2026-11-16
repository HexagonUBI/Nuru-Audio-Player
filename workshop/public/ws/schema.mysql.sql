CREATE TABLE IF NOT EXISTS ws_people (
  id VARCHAR(32) NOT NULL PRIMARY KEY,
  name VARCHAR(64) NOT NULL,
  seen BIGINT NOT NULL,
  banned TINYINT NOT NULL DEFAULT 0
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS ws_items (
  id VARCHAR(32) NOT NULL PRIMARY KEY,
  kind VARCHAR(8) NOT NULL,
  slug VARCHAR(64) NOT NULL,
  owner VARCHAR(32) NOT NULL,
  status VARCHAR(10) NOT NULL,
  current_version INT NULL,
  title VARCHAR(160) NOT NULL,
  summary TEXT NOT NULL,
  tags VARCHAR(255) NOT NULL DEFAULT '',
  subs INT NOT NULL DEFAULT 0,
  up INT NOT NULL DEFAULT 0,
  down INT NOT NULL DEFAULT 0,
  score DOUBLE NOT NULL DEFAULT 0,
  created BIGINT NOT NULL,
  published BIGINT NULL,
  updated BIGINT NOT NULL,
  UNIQUE KEY ws_items_slug (slug),
  KEY ws_items_owner (owner),
  KEY ws_items_listing (status, published)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS ws_versions (
  id VARCHAR(32) NOT NULL PRIMARY KEY,
  item VARCHAR(32) NOT NULL,
  number INT NOT NULL,
  status VARCHAR(10) NOT NULL,
  rev INT NOT NULL DEFAULT 0,
  note TEXT NOT NULL,
  meta MEDIUMTEXT NOT NULL,
  files TEXT NOT NULL,
  bytes BIGINT NOT NULL DEFAULT 0,
  created BIGINT NOT NULL,
  submitted BIGINT NULL,
  decided BIGINT NULL,
  decided_by VARCHAR(32) NULL,
  decision_note TEXT NOT NULL,
  UNIQUE KEY ws_versions_number (item, number),
  KEY ws_versions_queue (status, submitted)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS ws_subs (
  account VARCHAR(32) NOT NULL,
  item VARCHAR(32) NOT NULL,
  since BIGINT NOT NULL,
  PRIMARY KEY (account, item),
  KEY ws_subs_item (item)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS ws_votes (
  account VARCHAR(32) NOT NULL,
  item VARCHAR(32) NOT NULL,
  value TINYINT NOT NULL,
  at BIGINT NOT NULL,
  PRIMARY KEY (account, item),
  KEY ws_votes_item (item)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS ws_reports (
  id VARCHAR(32) NOT NULL PRIMARY KEY,
  item VARCHAR(32) NOT NULL,
  account VARCHAR(32) NOT NULL,
  reason VARCHAR(16) NOT NULL,
  text TEXT NOT NULL,
  created BIGINT NOT NULL,
  closed BIGINT NULL,
  closed_by VARCHAR(32) NULL,
  KEY ws_reports_open (closed, created)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS ws_limits (
  id VARCHAR(40) NOT NULL PRIMARY KEY,
  count INT NOT NULL,
  window_start BIGINT NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS ws_audit (
  id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
  account VARCHAR(32) NULL,
  action VARCHAR(32) NOT NULL,
  detail TEXT NOT NULL,
  at BIGINT NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
