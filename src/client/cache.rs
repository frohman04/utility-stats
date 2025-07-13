use flate2::Compression;
use flate2::write::{GzDecoder, GzEncoder};
use rmp_serde::{Deserializer, Serializer};
use rusqlite::Connection;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use time::Date;
use time::macros::date;

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

    pub fn get_connection(&self, table_name: &str) -> ClientCacheConnection {
        let db_path = self.db_path.as_path();
        let conn = Connection::open(db_path)
            .unwrap_or_else(|err| panic!("Unable to open database {db_path:?}: {err}"));

        ClientCacheConnection {
            conn,
            table_name: table_name.to_string(),
        }
    }
}

pub struct ClientCacheConnection {
    pub conn: Connection,
    pub table_name: String,
}

impl ClientCacheConnection {
    /// Get the DB key for a given date
    fn get_key(date: &Date) -> i64 {
        let epoch = date!(1970 - 01 - 01);
        (*date - epoch).whole_days()
    }

    /// Initialize the DB used to cache NwsResponse objects
    pub fn init_db(&self) {
        self.conn
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        date INTEGER NOT NULL PRIMARY KEY,
                        response BLOB NOT NULL
                    )",
                    self.table_name
                ),
                [],
            )
            .unwrap_or_else(|err| panic!("Unable to create table: {err}"));
    }

    /// Read a NwsResponse from the database
    pub fn read_data<R: DeserializeOwned>(&self, date: &Date) -> Option<R> {
        self.conn
            .prepare(&format!(
                "SELECT response FROM {} WHERE date = ?1",
                self.table_name
            ))
            .unwrap_or_else(|err| panic!("Unable to determine if date {date} for in DB: {err}"))
            .query_map(params![Self::get_key(date)], |row| {
                Ok(row.get(0).unwrap_or_else(|err| {
                    panic!("Unable to read data from DB row for date {date}: {err}")
                }))
            })
            .unwrap_or_else(|err| panic!("Unable to determine if date {date} for in DB: {err}"))
            .next()
            .map(|x| {
                let response: Vec<u8> =
                    x.unwrap_or_else(|err| panic!("Unable to read data for date {date}: {err}"));
                Self::read_blob(response)
            })
    }

    /// Write a VisualCrossingResponse to the database
    pub fn write_data<R: Serialize>(&self, date: &Date, response: &R) {
        let encoded = Self::write_blob(response);
        self.conn
            .execute(
                &format!(
                    "INSERT INTO {}(date, response) VALUES (?1, ?2)",
                    self.table_name
                ),
                params![Self::get_key(date), encoded],
            )
            .unwrap_or_else(|err| {
                panic!("Unable to write NWS data into cache for date {date}: {err}")
            });
    }

    /// Read a NwsResponse from a MessagePack binary blob
    fn read_blob<R: DeserializeOwned>(raw: Vec<u8>) -> R {
        // decompress
        let mut decompressed = Vec::new();
        let mut decoder = GzDecoder::new(decompressed);
        decoder
            .write_all(&raw[..])
            .unwrap_or_else(|err| panic!("Unable to decompress data: {err}"));
        decompressed = decoder
            .finish()
            .unwrap_or_else(|err| panic!("Unable to decompress data: {err}"));

        // deserialize to object
        let mut de = Deserializer::new(&decompressed[..]);
        let response: R = Deserialize::deserialize(&mut de)
            .unwrap_or_else(|err| panic!("Unable to deserialize data: {err}"));

        response
    }

    /// Write a response to a MessagePack binary blob
    fn write_blob<R: Serialize>(response: &R) -> Vec<u8> {
        // serialize to buffer
        let mut obj_buf = Vec::new();
        response
            .serialize(&mut Serializer::new(&mut obj_buf))
            .unwrap_or_else(|err| panic!("Unable to serialize data: {err}"));

        // compress buffer
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(&obj_buf)
            .unwrap_or_else(|err| panic!("Unable to compress data: {err}"));
        encoder
            .finish()
            .unwrap_or_else(|err| panic!("Unable to compress data: {err}"))
    }
}
