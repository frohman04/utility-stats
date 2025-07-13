use rusqlite::Connection;
use std::path::PathBuf;

pub struct ClientCache {
    db_path: PathBuf,
}

impl ClientCache {
    pub fn new(cache_dir: String) -> ClientCache {
        let mut db_path = PathBuf::from(&cache_dir);
        db_path.push("db");
        db_path.set_extension("sqlite");

        ClientCache { db_path }
    }

    pub fn get_connection(&self) -> Connection {
        let db_path = self.db_path.as_path();
        Connection::open(db_path)
            .unwrap_or_else(|err| panic!("Unable to open database {db_path:?}: {err}"))
    }
}
