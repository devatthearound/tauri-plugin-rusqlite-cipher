use crate::types::{Result, ResultList};
use commands::{
    batch::execute_batch, migration::execute_migration, select::execute_select,
    update::execute_update,
};
use error::Error;
use serde_json::Value as JsonValue;
use std::{collections::HashMap, sync::Mutex};
use tauri::{
    command,
    plugin::{Builder, TauriPlugin},
    Manager, Runtime, State,
};
use types::Migrations;
// 기능 플래그를 사용한 조건부 컴파일을 위해 `cfg` 속성 사용
#[cfg(feature = "sqlcipher")]
use rusqlite::{params, Connection, OpenFlags};

#[cfg(not(feature = "sqlcipher"))]
use rusqlite::Connection;

mod commands;
mod common;
mod error;
mod types;

#[derive(Default)]
struct ConfigState(Mutex<HashMap<String, Connection>>);

// Assuming your custom error module is named `error` and has a type named `Error`
impl From<rusqlite::Error> for error::Error {
    fn from(err: rusqlite::Error) -> Self {
        // Here you should convert a `rusqlite::Error` into your custom error type.
        // This is just an example; adjust the conversion logic according to your error type's structure.
        error::Error::Database(err.to_string())
    }
}


// Assuming `open_in_path` is being called and it's supposed to open a database connection.
#[command]
async fn open_in_path(state: State<'_, ConfigState>, path: String, use_sqlcipher: bool, cipher_key: Option<String>) -> Result<()> {
    // Connection::open and Connection::open_with_flags return a Result type.
    let connection_result = if use_sqlcipher {
        #[cfg(feature = "sqlcipher")]
        {
            let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
            if let Some(key) = cipher_key {
                // This execute call returns a Result, so you can use map_err here
                conn.execute("PRAGMA key = ?", params![key]).map_err(|e| Error::SqliteError(e.to_string()))?;
            }
            Ok(conn) // Wrap the connection in a Result if not already
        }
        #[cfg(not(feature = "sqlcipher"))]
        {
            // If not using SQLCipher, just try to open the connection normally.
            Connection::open(&path)
        }
    } else {
        // If not using SQLCipher
        Connection::open(&path)
    };

    // Here, connection_result is a Result, so you can use map_err (if needed).
    let connection = connection_result.map_err(|e| Error::OpeningConnection(e.to_string()))?;

    insert_connection(state, connection, path)
}


// #[command]
// async fn open_in_path(state: State<'_, ConfigState>, path: String) -> Result<()> {
//     let connection = Connection::open_with_flags(path.clone(), OpenFlags::default())
//         .map_err(|error| Error::OpeningConnection(error.to_string()))?;

//     insert_connection(state, connection, path)
// }

fn insert_connection(
    state: State<'_, ConfigState>,
    connection: Connection,
    name: String,
) -> Result<()> {
    let mut connections = state.0.lock().unwrap();
    let contains_key = connections.contains_key(&name);

    if !contains_key {
        connections.insert(name.clone(), connection);
    }

    Ok(())
}

#[command]
async fn migration(
    state: State<'_, ConfigState>,
    name: String,
    migrations: Migrations,
) -> Result<()> {
    let connections = state.0.lock().unwrap();
    let connection = match connections.get(&name) {
        Some(connection) => connection,
        None => return Err(Error::Connection()),
    };

    execute_migration(connection, migrations)
}

#[command]
async fn update(
    state: State<'_, ConfigState>,
    name: String,
    sql: String,
    parameters: HashMap<String, JsonValue>,
) -> Result<()> {
    let connections = state.0.lock().unwrap();
    let connection = match connections.get(&name) {
        Some(connection) => connection,
        None => return Err(Error::Connection()),
    };

    execute_update(connection, sql, parameters)
}

#[command]
async fn select(
    state: State<'_, ConfigState>,
    name: String,
    sql: String,
    parameters: HashMap<String, JsonValue>,
) -> Result<ResultList> {
    let connections = state.0.lock().unwrap();
    let connection = match connections.get(&name) {
        Some(connection) => connection,
        None => return Err(Error::Connection()),
    };

    execute_select(connection, sql, parameters)
}

#[command]
async fn batch(state: State<'_, ConfigState>, name: String, batch_sql: String) -> Result<()> {
    let connections = state.0.lock().unwrap();
    let connection = match connections.get(&name) {
        Some(connection) => connection,
        None => return Err(Error::Connection()),
    };

    execute_batch(connection, batch_sql)
}

#[command]
async fn close(state: State<'_, ConfigState>, name: String) -> Result<()> {
    let mut connections = state.0.lock().unwrap();
    let connection = match connections.remove(&name) {
        Some(connection) => connection,
        None => return Err(Error::Connection()),
    };

    connection
        .close()
        .map_err(|(_, error)| Error::ClosingConnection(error.to_string()))?;
    Ok(())
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("rusqlite")
        .invoke_handler(tauri::generate_handler![
            open_in_memory,
            open_in_path,
            migration,
            update,
            select,
            batch,
            close
        ])
        .setup(|app| {
            app.manage(ConfigState::default());
            Ok(())
        })
        .build()
}
