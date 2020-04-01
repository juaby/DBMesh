//! A "tiny database" and accompanying protocol
//!
//! This example shows the usage of shared state amongst all connected clients,
//! namely a database of key/value pairs. Each connected client can send a
//! series of GET/SET commands to query the current value of a key or set the
//! value of a key.
//!
//! This example has a simple protocol you can use to interact with the server.
//! To run, first run this in one terminal window:
//!
//!     cargo run --example tinydb
//!
//! and next in another windows run:
//!
//!     cargo run --example connect 127.0.0.1:8080
//!
//! In the `connect` window you can type in commands where when you hit enter
//! you'll get a response from the server for that command. An example session
//! is:
//!
//!
//!     $ cargo run --example connect 127.0.0.1:8080
//!     GET foo
//!     foo = bar
//!     GET FOOBAR
//!     error: no key FOOBAR
//!     SET FOOBAR my awesome string
//!     set FOOBAR = `my awesome string`, previous: None
//!     SET foo tokio
//!     set foo = `tokio`, previous: Some("bar")
//!     GET foo
//!     foo = tokio
//!
//! Namely you can issue two forms of commands:
//!
//! * `GET $key` - this will fetch the value of `$key` from the database and
//!   return it. The server's database is initially populated with the key `foo`
//!   set to the value `bar`
//! * `SET $key $value` - this will set the value of `$key` to `$value`,
//!   returning the previous value, if any.

#![warn(rust_2018_idioms)]

use tokio::net::TcpListener;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};

use bytes::{Bytes, BytesMut, Buf, BufMut};

use tokio::stream::StreamExt;
use futures::SinkExt;

use mysql::prelude::*;
use mysql::{Conn, from_row};

mod protocol;

use protocol::database::mysql::constant::*;
use protocol::database::mysql::codec::*;
use crate::protocol::database::mysql::packet::{MySQLHandshakePacket, MySQLPacketPayload, MySQLHandshakeResponse41Packet, MySQLOKPacket, MySQLPacket};
use crate::protocol::database::{DatabasePacket, PacketPayload};


/// The in-memory database shared amongst all clients.
///
/// This database will be shared via `Arc`, so to mutate the internal map we're
/// going to use a `Mutex` for interior mutability.
struct Database {
    map: Mutex<HashMap<String, String>>,
}

/// Possible requests our clients can send us
enum Request {
    Get { key: String },
    Set { key: String, value: String },
}

/// Responses to the `Request` commands above
enum Response {
    Value {
        key: String,
        value: String,
    },
    Set {
        key: String,
        value: String,
        previous: Option<String>,
    },
    Error {
        msg: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
struct Payment {
    customer_id: i32,
    amount: i32,
    account_name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse the address we're going to run this server on
    // and set up our TCP listener to accept connections.
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:3306".to_string());

    let mut listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    // Create the shared state of this server that will be shared amongst all
    // clients. We populate the initial database and then create the `Database`
    // structure. Note the usage of `Arc` here which will be used to ensure that
    // each independently spawned client will have a reference to the in-memory
    // database.
    let mut initial_db = HashMap::new();
    initial_db.insert("foo".to_string(), "bar".to_string());
    let db = Arc::new(Database {
        map: Mutex::new(initial_db),
    });

    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                // After getting a new connection first we see a clone of the database
                // being created, which is creating a new reference for this connected
                // client to use.
                let db = db.clone();

                // Like with other small servers, we'll `spawn` this client to ensure it
                // runs concurrently with all other clients. The `move` keyword is used
                // here to move ownership of our db handle into the async closure.
                tokio::spawn(async move {
                    // Since our protocol is line-based we use `tokio_codecs`'s `LineCodec`
                    // to convert our stream of bytes, `socket`, into a `Stream` of lines
                    // as well as convert our line based responses into a stream of bytes.

                    let (r, w) = socket.split();
                    let mut stream = read_frame(r);
                    let mut sink = write_frame(w);

                    let mut authorized = false;

                    if !authorized {
                        let mut packet = MySQLHandshakePacket::new(100);
                        let mut payload = MySQLPacketPayload::new();
                        let mut payload = DatabasePacket::encode(&mut packet, &mut payload);
                        let bytes = payload.get_bytes();

                        if let Err(e) = sink.send(bytes).await {
                            println!("error on sending response; error = {:?}", e);
                        }
                    }

                    // Here for every line we get back from the `Framed` decoder,
                    // we parse the request, and if it's valid we generate a response
                    // based on the values in the database.
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(mut payload) => {
                                if !authorized {
                                    let mut packet = MySQLHandshakeResponse41Packet::new();
                                    let mut payload = MySQLPacketPayload::new_with_payload(payload);
                                    packet.decode(&mut payload);

                                    let mut ok_packet = MySQLOKPacket::new(packet.get_sequence_id() + 1);
                                    let mut ok_payload = MySQLPacketPayload::new();
                                    let mut ok_payload = DatabasePacket::encode(&mut ok_packet, &mut ok_payload);
                                    let ok_bytes = ok_payload.get_bytes();

                                    if let Err(e) = sink.send(ok_bytes).await {
                                        println!("error on sending response; error = {:?}", e);
                                    }

                                    authorized = true; //小鱼在水里活泼乱跳 闫圣哲 王茹玉 毛毛虫 人类 电脑
                                } else {
                                    let database_url = "mysql://root:root@localhost:8306/test";

                                    let mut conn = Conn::new(database_url).unwrap();

                                    // This query will emit two result sets.
                                    let mut result = conn.query_iter("SELECT a from test").unwrap();

                                    let mut sets = 0;
                                    while let Some(result_set) = result.next_set() {
                                        let result_set = result_set.unwrap();
                                        sets += 1;

                                        let columns = result_set.columns();
                                        for c in columns.as_ref() {
                                            println!("s_name: {}", c.schema_str());
                                            println!("t_name: {}", c.table_str());
                                            println!("name: {}", c.name_str());
                                            println!("len: {}", c.column_length());
                                            println!("type: {:?}", c.column_type());
                                            println!("charset: {}", c.character_set());
                                            println!("org_t_name: {:?}", c.org_table_str());
                                        }
                                        println!("Result set columns: {:?}", result_set.columns());
                                        println!(
                                            "Result set meta: {}, {:?}, {} {}",
                                            result_set.affected_rows(),
                                            result_set.last_insert_id(),
                                            result_set.warnings(),
                                            result_set.info_str(),
                                        );

                                        for row in result_set {
                                            println!("Result set row: {:?}", row);
                                        }
                                    }

                                    let response = handle_request(&String::from_utf8_lossy(payload.bytes()), &db);
                                    let response = response.serialize();
                                    let mut frame = Bytes::new();
                                    if let Err(e) = sink.send(frame).await {
                                        println!("error on sending response; error = {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("error on decoding from socket; error = {:?}", e);
                            }
                        }
                    }

                    // The connection will be closed at this point as `lines.next()` has returned `None`.
                });
            }
            Err(e) => println!("error accepting socket; error = {:?}", e),
        }
    }
}

fn handle_request(line: &str, db: &Arc<Database>) -> Response {
    let request = match Request::parse(&line) {
        Ok(req) => req,
        Err(e) => return Response::Error { msg: e },
    };

    let mut db = db.map.lock().unwrap();
    match request {
        Request::Get { key } => match db.get(&key) {
            Some(value) => Response::Value {
                key,
                value: value.clone(),
            },
            None => Response::Error {
                msg: format!("no key {}", key),
            },
        },
        Request::Set { key, value } => {
            let previous = db.insert(key.clone(), value.clone());
            Response::Set {
                key,
                value,
                previous,
            }
        }
    }
}

impl Request {
    fn parse(input: &str) -> Result<Request, String> {
        let mut parts = input.splitn(3, ' ');
        match parts.next() {
            Some("GET") => {
                let key = parts.next().ok_or("GET must be followed by a key")?;
                if parts.next().is_some() {
                    return Err("GET's key must not be followed by anything".into());
                }
                Ok(Request::Get {
                    key: key.to_string(),
                })
            }
            Some("SET") => {
                let key = match parts.next() {
                    Some(key) => key,
                    None => return Err("SET must be followed by a key".into()),
                };
                let value = match parts.next() {
                    Some(value) => value,
                    None => return Err("SET needs a value".into()),
                };
                Ok(Request::Set {
                    key: key.to_string(),
                    value: value.to_string(),
                })
            }
            Some(cmd) => Err(format!("unknown command: {}", cmd)),
            None => Err("empty input".into()),
        }
    }
}

impl Response {
    fn serialize(&self) -> String {
        match *self {
            Response::Value { ref key, ref value } => format!("{} = {}", key, value),
            Response::Set {
                ref key,
                ref value,
                ref previous,
            } => format!("set {} = `{}`, previous: {:?}", key, value, previous),
            Response::Error { ref msg } => format!("error: {}", msg),
        }
    }
}