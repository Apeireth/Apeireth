use rusqlite::Connection;

fn main() {
    let conn = Connection::open_in_memory().unwrap();
    match conn.load_extension_enable() {
        Ok(()) => println!("load_extension enable: OK"),
        Err(e) => println!("load_extension enable ERR: {e}"),
    }
    // probe whether vec0 标量函数已注册
    let r: Result<String, _> = conn
        .query_row("SELECT vec_version()", [], |row| row.get(0))
        .map_err(|e| format!("{e}"));
    println!("vec_version() result: {:?}", r);
}
