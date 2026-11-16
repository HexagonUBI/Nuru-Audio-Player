CREATE TABLE IF NOT EXISTS ws_people (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  seen INTEGER NOT NULL,
  banned INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ws_items (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  slug TEXT NOT NULL UNIQUE,
  owner TEXT NOT NULL,
  status TEXT NOT NULL,
  current_version INTEGER NULL,
  title TEXT NOT NULL,
  summary TEXT NOT NULL DEFAULT '',
  tags TEXT NOT NULL DEFAULT '',
  subs INTEGER NOT NULL DEFAULT 0,
  up INTEGER NOT NULL DEFAULT 0,
  down INTEGER NOT NULL DEFAULT 0,
  score REAL NOT NULL DEFAULT 0,
  created INTEGER NOT NULL,
  published INTEGER NULL,
  updated INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS ws_items_owner ON ws_items (owner);
CREATE INDEX IF NOT EXISTS ws_items_listing ON ws_items (status, published);

CREATE TABLE IF NOT EXISTS ws_versions (
  id TEXT PRIMARY KEY,
  item TEXT NOT NULL,
  number INTEGER NOT NULL,
  status TEXT NOT NULL,
  rev INTEGER NOT NULL DEFAULT 0,
  note TEXT NOT NULL DEFAULT '',
  meta TEXT NOT NULL,
  files TEXT NOT NULL DEFAULT '{}',
  bytes INTEGER NOT NULL DEFAULT 0,
  created INTEGER NOT NULL,
  submitted INTEGER NULL,
  decided INTEGER NULL,
  decided_by TEXT NULL,
  decision_note TEXT NOT NULL DEFAULT '',
  UNIQUE (item, number)
);
CREATE INDEX IF NOT EXISTS ws_versions_queue ON ws_versions (status, submitted);

CREATE TABLE IF NOT EXISTS ws_subs (
  account TEXT NOT NULL,
  item TEXT NOT NULL,
  since INTEGER NOT NULL,
  PRIMARY KEY (account, item)
);
CREATE INDEX IF NOT EXISTS ws_subs_item ON ws_subs (item);

CREATE TABLE IF NOT EXISTS ws_votes (
  account TEXT NOT NULL,
  item TEXT NOT NULL,
  value INTEGER NOT NULL,
  at INTEGER NOT NULL,
  PRIMARY KEY (account, item)
);
CREATE INDEX IF NOT EXISTS ws_votes_item ON ws_votes (item);

CREATE TABLE IF NOT EXISTS ws_reports (
  id TEXT PRIMARY KEY,
  item TEXT NOT NULL,
  account TEXT NOT NULL,
  reason TEXT NOT NULL,
  text TEXT NOT NULL DEFAULT '',
  created INTEGER NOT NULL,
  closed INTEGER NULL,
  closed_by TEXT NULL
);
CREATE INDEX IF NOT EXISTS ws_reports_open ON ws_reports (closed, created);

CREATE TABLE IF NOT EXISTS ws_limits (
  id TEXT PRIMARY KEY,
  count INTEGER NOT NULL,
  window_start INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS ws_audit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account TEXT NULL,
  action TEXT NOT NULL,
  detail TEXT NOT NULL DEFAULT '',
  at INTEGER NOT NULL
);
