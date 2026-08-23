use rusqlite::Connection;

pub fn run_migrations(conn: &mut Connection) -> rusqlite::Result<()> {
    // V1 to V7 tables
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS episodes (id TEXT PRIMARY KEY, data TEXT);
        CREATE TABLE IF NOT EXISTS notes (id TEXT PRIMARY KEY, data TEXT);
        CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, data TEXT);
        CREATE TABLE IF NOT EXISTS agent_traces (id TEXT PRIMARY KEY, data TEXT);
        CREATE TABLE IF NOT EXISTS facts (id TEXT PRIMARY KEY, data TEXT);
        CREATE TABLE IF NOT EXISTS links (id TEXT PRIMARY KEY, data TEXT);
        CREATE TABLE IF NOT EXISTS topic_groups (id TEXT PRIMARY KEY, data TEXT);
        CREATE TABLE IF NOT EXISTS provenance (id TEXT PRIMARY KEY, data TEXT);
        CREATE INDEX IF NOT EXISTS idx_facts_id ON facts(id);
    "#)?;
    Ok(())
}
