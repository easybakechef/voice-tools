use std::{
    io::SeekFrom,
    path::PathBuf,
    sync::{atomic::{AtomicU64, Ordering}, Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use rusqlite::{params, Connection};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tower_http::cors::CorsLayer;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_filename() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}.webm", ts, n)
}

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    recordings_dir: PathBuf,
}

#[derive(Serialize)]
struct RecordingMeta {
    id: i64,
    name: String,
    date: i64,
    duration: f64,
    median_pitch: f64,
    pitch_log: String,
    formant_data: String,
    stats: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let data_dir = PathBuf::from(std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".into()));
    let recordings_dir = data_dir.join("recordings");
    tokio::fs::create_dir_all(&recordings_dir).await?;

    let db_path = data_dir.join("voice_trainer.db");
    let conn = Connection::open(&db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS recordings (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         TEXT    NOT NULL,
            date         INTEGER NOT NULL,
            duration     REAL    NOT NULL,
            median_pitch REAL    NOT NULL,
            filename     TEXT    NOT NULL,
            pitch_log    TEXT    NOT NULL DEFAULT '[]',
            formant_data TEXT    NOT NULL DEFAULT '[]',
            stats        TEXT    NOT NULL DEFAULT '{}'
        );",
    )?;
    // Migrate existing DBs that lack the new columns
    for col in &[
        "pitch_log TEXT NOT NULL DEFAULT '[]'",
        "formant_data TEXT NOT NULL DEFAULT '[]'",
        "stats TEXT NOT NULL DEFAULT '{}'",
    ] {
        let _ = conn.execute(&format!("ALTER TABLE recordings ADD COLUMN {col}"), []);
    }

    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
        recordings_dir,
    };

    let api = Router::new()
        .route("/recordings", get(list_recordings).post(upload_recording))
        .route("/recordings/:id", delete(delete_recording))
        .route("/recordings/:id/audio", get(stream_audio))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let app = Router::new().nest("/api", api);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    println!("Voice Trainer server → http://localhost:3001");
    println!("Data directory: {}", data_dir.display());
    axum::serve(listener, app).await?;
    Ok(())
}

async fn list_recordings(State(state): State<AppState>) -> impl IntoResponse {
    let rows = tokio::task::spawn_blocking(move || {
        let db = state.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT id, name, date, duration, median_pitch, pitch_log, formant_data, stats
             FROM recordings ORDER BY date DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RecordingMeta {
                id:           row.get(0)?,
                name:         row.get(1)?,
                date:         row.get(2)?,
                duration:     row.get(3)?,
                median_pitch: row.get(4)?,
                pitch_log:    row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "[]".into()),
                formant_data: row.get::<_, Option<String>>(6)?.unwrap_or_else(|| "[]".into()),
                stats:        row.get::<_, Option<String>>(7)?.unwrap_or_else(|| "{}".into()),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    })
    .await
    .unwrap()
    .unwrap_or_default();

    Json(rows)
}

async fn upload_recording(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<RecordingMeta>, StatusCode> {
    let mut name = String::new();
    let mut date: i64 = 0;
    let mut duration: f64 = 0.0;
    let mut median_pitch: f64 = 0.0;
    let mut audio_bytes: Vec<u8> = Vec::new();
    let mut pitch_log = String::from("[]");
    let mut formant_data = String::from("[]");
    let mut stats = String::from("{}");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "name" => {
                name = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            }
            "date" => {
                date = field
                    .text()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?
                    .parse()
                    .unwrap_or(0);
            }
            "duration" => {
                duration = field
                    .text()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?
                    .parse()
                    .unwrap_or(0.0);
            }
            "median_pitch" => {
                median_pitch = field
                    .text()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?
                    .parse()
                    .unwrap_or(0.0);
            }
            "audio" => {
                audio_bytes = field
                    .bytes()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?
                    .to_vec();
            }
            "pitch_log"    => { pitch_log    = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?; }
            "formant_data" => { formant_data = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?; }
            "stats"        => { stats        = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?; }
            _ => {}
        }
    }

    if audio_bytes.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let filename = unique_filename();
    tokio::fs::write(state.recordings_dir.join(&filename), &audio_bytes)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let name_db        = name.clone();
    let pitch_log_db   = pitch_log.clone();
    let formant_data_db = formant_data.clone();
    let stats_db       = stats.clone();
    let id = tokio::task::spawn_blocking(move || {
        let db = state.db.lock().unwrap();
        db.execute(
            "INSERT INTO recordings (name, date, duration, median_pitch, filename, pitch_log, formant_data, stats)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![name_db, date, duration, median_pitch, filename, pitch_log_db, formant_data_db, stats_db],
        )?;
        Ok::<i64, rusqlite::Error>(db.last_insert_rowid())
    })
    .await
    .unwrap()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RecordingMeta { id, name, date, duration, median_pitch, pitch_log, formant_data, stats }))
}

async fn delete_recording(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let filename: Option<String> = tokio::task::spawn_blocking(move || {
        let db = state.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT filename FROM recordings WHERE id = ?1")?;
        let filename: Option<String> = stmt
            .query_row(params![id], |row| row.get(0))
            .ok();
        if filename.is_some() {
            let _ = db.execute("DELETE FROM recordings WHERE id = ?1", params![id]);
        }
        Ok::<Option<String>, rusqlite::Error>(filename)
    })
    .await
    .unwrap()
    .unwrap_or(None);

    if let Some(f) = filename {
        let _ = tokio::fs::remove_file(state.recordings_dir.join(&f)).await;
    }

    StatusCode::NO_CONTENT
}

async fn stream_audio(
    Path(id): Path<i64>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    let filename: Option<String> = tokio::task::spawn_blocking(move || {
        let db = state.db.lock().unwrap();
        db.query_row(
            "SELECT filename FROM recordings WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .ok()
    })
    .await
    .unwrap();

    let filename = match filename {
        Some(f) => f,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let path = state.recordings_dir.join(&filename);
    let mut file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let file_size = match file.metadata().await {
        Ok(m) => m.len(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // Parse Range header so the browser can seek
    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("bytes="))
        .and_then(|s| s.split_once('-'))
        .map(|(s, e)| {
            let start: u64 = s.parse().unwrap_or(0);
            let end: u64 = if e.is_empty() {
                file_size.saturating_sub(1)
            } else {
                e.parse()
                    .unwrap_or(file_size.saturating_sub(1))
                    .min(file_size.saturating_sub(1))
            };
            (start, end)
        });

    if let Some((start, end)) = range {
        if start > end || start >= file_size {
            return StatusCode::RANGE_NOT_SATISFIABLE.into_response();
        }
        let length = end - start + 1;
        let mut buf = vec![0u8; length as usize];
        if file.seek(SeekFrom::Start(start)).await.is_err()
            || file.read_exact(&mut buf).await.is_err()
        {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_TYPE, "audio/webm")
            .header(header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, file_size))
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_LENGTH, length)
            .body(Body::from(buf))
            .unwrap()
    } else {
        let mut buf = Vec::with_capacity(file_size as usize);
        if file.read_to_end(&mut buf).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "audio/webm")
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_LENGTH, file_size)
            .body(Body::from(buf))
            .unwrap()
    }
}
