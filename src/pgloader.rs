
use postgres::{Connection, TlsMode};
// use std::env;
// use curl::http;

use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, Cursor, Read};
// use flate2::read::GzDecoder;

#[derive(Debug)]
pub struct PgConn {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String
}

struct IteratorAsRead<I>
where
    I: Iterator,
{
    iter: I,
    cursor: Option<Cursor<I::Item>>,
}

impl<I> IteratorAsRead<I>
where
    I: Iterator,
{
    pub fn new<T>(iter: T) -> Self
    where
        T: IntoIterator<IntoIter = I, Item = I::Item>,
    {
        let mut iter = iter.into_iter();
        let cursor = iter.next().map(Cursor::new);
        IteratorAsRead { iter, cursor }
    }
}

impl<I> Read for IteratorAsRead<I>
where
    I: Iterator,
Cursor<I::Item>: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.cursor.is_some() {
            let read = self.cursor.as_mut().unwrap().read(buf)?;
            if read > 0 {
                return Ok(read);
            }
            self.cursor = self.iter.next().map(Cursor::new);
        }
        Ok(0)
    }
}


fn conn_str(cfg:&PgConn) -> String {
   return format!("postgres://{}:{}@{}:{}/{}", cfg.user, cfg.password, cfg.host, cfg.port, cfg.database);
}


pub fn load(file_name:String, table_name: String, conn:PgConn) {

    let conn_s = conn_str(&conn);
    let conn = Connection::connect(conn_s, TlsMode::None).map_err(|err| println!("PostgreSQL connection failed for {:?} {}", conn, err)).unwrap();
    let f = File::open(&file_name).unwrap();
    let reader = BufReader::new(f);
    // let gzip = GzDecoder::new(reader);
    // let greader = BufReader::new(gzip);

    let stream = reader.lines();

    let source = stream.map(
        |res| {
            let jsonstr =  res.ok().unwrap();
            let res:serde_json::Value = serde_json::from_str(&jsonstr).unwrap();
            if let (Some(ts), Some(ev)) = (res["ts"].as_str(), res["ev"].as_str()) {
                return format!("{}\t{}\t{}\n", ts, ev, res);
            } else {
                return String::new();
            }
        }
    );

    let mut source = IteratorAsRead::new(source);

    let init_sql = format!("drop table if exists {};", table_name);
    conn.execute(&init_sql, &[]).unwrap();

    let init_sql = format!("create table if not exists {} (ts timestamp, ev  text, resource jsonb);", table_name);
    conn.execute(&init_sql, &[]).unwrap();

    println!("SQL: {}", init_sql);

    let copy_sql = format!("COPY {} (ts, ev, resource)  FROM STDIN CSV quote e'\\x01' DELIMITER e'\\t'", table_name);
    let stmt = conn.prepare(&copy_sql).unwrap();
    stmt.copy_in(&[], &mut source).unwrap();

    println!("\nDone!");

}
