"""SQLite store for per-sample voice metrics."""
import os
import sqlite3

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data")
DB_PATH = os.path.join(DATA_DIR, "samples.db")

SCHEMA = """
create table if not exists samples (
    id          integer primary key autoincrement,
    dataset     text not null,
    speaker     text not null,
    gender      text,            -- 'male' | 'female' | 'nonbinary' | other label | null
    path        text not null,
    duration    real,            -- seconds
    f0_median   real,            -- Hz, median over voiced frames
    voiced_frac real,            -- fraction of frames that were voiced
    f1_median   real,            -- Hz
    f2_median   real,            -- Hz
    kept        integer default 1, -- 1 = audio kept on disk, 0 = metrics-only (audio discarded)
    unique(dataset, path)
);
"""

COLUMNS = ["dataset", "speaker", "gender", "path",
           "duration", "f0_median", "voiced_frac", "f1_median", "f2_median", "kept"]


def connect():
    os.makedirs(DATA_DIR, exist_ok=True)
    con = sqlite3.connect(DB_PATH, timeout=60)
    con.execute("pragma busy_timeout=60000")  # wait out concurrent readers/writers
    con.executescript(SCHEMA)
    if "kept" not in [r[1] for r in con.execute("pragma table_info(samples)")]:
        con.execute("alter table samples add column kept integer default 1")
    return con


def insert_sample(con, **kw):
    placeholders = ",".join("?" for _ in COLUMNS)
    con.execute(
        f"insert or ignore into samples ({','.join(COLUMNS)}) values ({placeholders})",
        [kw.get(c) for c in COLUMNS],
    )
