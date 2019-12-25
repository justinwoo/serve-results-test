use gotham::helpers::http::response::create_response;
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::single::single_pipeline;
use gotham::pipeline::single_middleware;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};
use gotham_derive::StateData;
use hyper::{Body, Response, StatusCode};
use rusqlite::{params, Connection, Result};
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Clone, StateData)]
struct Shared {
    conn: Arc<Mutex<Connection>>,
}

impl Shared {
    fn new() -> Self {
        let conn = get_conn().unwrap_or_else(|_| {
            eprintln!("Could not get sqlite connection.");
            std::process::exit(1);
        });
        let _ = prepare_data(&conn);

        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }
}

#[derive(Debug, Serialize)]
struct Name {
    id: i32,
    name: String,
}

fn router() -> Router {
    let shared = Shared::new();
    let middleware = StateMiddleware::new(shared);
    let pipeline = single_middleware(middleware);
    let (chain, pipelines) = single_pipeline(pipeline);

    build_router(chain, pipelines, |route| {
        route.get("/names").to(get_handler);
    })
}

fn get_handler(state: State) -> (State, Response<Body>) {
    let res = {
        let shared = Shared::borrow_from(&state);
        let conn = shared.conn.lock().unwrap();
        let names = get_names(&conn).unwrap();
        let json = serde_json::to_string(&names).unwrap();

        create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, json)
    };

    (state, res)
}

fn get_conn() -> Result<Connection> {
    Connection::open_in_memory()
}

fn prepare_data(conn: &Connection) -> () {
    conn.execute(
        "CREATE TABLE names (
                  id              INTEGER PRIMARY KEY,
                  name            TEXT NOT NULL
                  )",
        params![],
    )
    .unwrap();

    let test_names = ["yes", "hi", "no", "wtf"];

    for name in test_names.iter() {
        conn.execute("INSERT INTO names (name) VALUES (?1)", params![name])
            .unwrap();
    }
}

fn get_names(conn: &Connection) -> Result<Vec<Name>> {
    let mut stmt = conn.prepare("SELECT id, name FROM names").unwrap();
    let names_rows: _ = stmt.query_map(params![], |row| {
        Ok(Name {
            id: row.get(0).unwrap(),
            name: row.get(1).unwrap(),
        })
    })?;

    names_rows.collect()
}

pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router())
}
