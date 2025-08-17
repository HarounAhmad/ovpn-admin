-- users
CREATE TABLE IF NOT EXISTS users(
  id TEXT PRIMARY KEY,
  username TEXT UNIQUE NOT NULL,
  pw_hash TEXT NOT NULL,
  require_pw_change INTEGER NOT NULL DEFAULT 0,
  disabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- roles
CREATE TABLE IF NOT EXISTS roles(
  name TEXT PRIMARY KEY
);
INSERT OR IGNORE INTO roles(name) VALUES ('ADMIN'),('OPS'),('READONLY');

-- user_roles
CREATE TABLE IF NOT EXISTS user_roles(
  user_id TEXT NOT NULL,
  role_name TEXT NOT NULL,
  PRIMARY KEY(user_id, role_name),
  FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
  FOREIGN KEY(role_name) REFERENCES roles(name) ON DELETE CASCADE
);

-- sessions
CREATE TABLE IF NOT EXISTS sessions(
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  last_auth_stepup INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- audit
CREATE TABLE IF NOT EXISTS audit(
  id TEXT PRIMARY KEY,
  ts INTEGER NOT NULL,
  actor_user TEXT NOT NULL,
  action TEXT NOT NULL,
  target TEXT NOT NULL,
  ip TEXT NOT NULL,
  ua TEXT NOT NULL,
  details TEXT NOT NULL
);

-- login throttle
CREATE TABLE IF NOT EXISTS login_attempts(
  username TEXT NOT NULL,
  ts INTEGER NOT NULL,
  ip TEXT NOT NULL
);
