-- 0026: sealed secrets, their grants, and the custody mark of the store key.
CREATE TABLE secret_secrets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    version INTEGER NOT NULL,
    sealed BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE secret_grants (
    secret_id TEXT NOT NULL REFERENCES secret_secrets (id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL,
    last_read_version INTEGER,
    last_read_at INTEGER,
    PRIMARY KEY (secret_id, agent_id)
);

CREATE INDEX secret_grants_by_agent ON secret_grants (agent_id);

CREATE TABLE secret_store (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    custody TEXT NOT NULL
);
