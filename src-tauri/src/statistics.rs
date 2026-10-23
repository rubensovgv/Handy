//! Local counters survive history retention. No transcript or audio is copied.
use anyhow::Result;
use chrono::{Local, TimeZone};
use rusqlite::{params, Connection};
use serde::Serialize;
use specta::Type;
use std::path::Path;

#[derive(Serialize, Type)]
pub struct StatisticsDay {
    pub date: String,
    pub words: i64,
    pub sessions: i64,
    pub seconds: f64,
    pub measured_words: i64,
}

pub fn initialize(conn: &Connection, recordings: &Path) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS dictation_statistics (
        history_id INTEGER PRIMARY KEY, day TEXT NOT NULL, words INTEGER NOT NULL,
        seconds REAL, CHECK(words >= 0));",
    )?;
    let mut query = conn.prepare("SELECT id, timestamp, transcription_text, file_name FROM transcription_history WHERE id NOT IN (SELECT history_id FROM dictation_statistics)")?;
    let entries = query.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    for row in entries {
        let (id, timestamp, text, file) = row?;
        record(conn, id, timestamp, &text, &recordings.join(file))?;
    }
    Ok(())
}

pub fn record(conn: &Connection, id: i64, timestamp: i64, text: &str, audio: &Path) -> Result<()> {
    let words = text
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .count() as i64;
    let day = Local
        .timestamp_opt(timestamp, 0)
        .single()
        .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?
        .format("%Y-%m-%d")
        .to_string();
    let seconds = hound::WavReader::open(audio).ok().and_then(|wav| {
        let rate = wav.spec().sample_rate;
        (rate > 0 && wav.duration() > 0).then(|| wav.duration() as f64 / rate as f64)
    });
    conn.execute("INSERT INTO dictation_statistics(history_id,day,words,seconds) VALUES (?1,?2,?3,?4)
        ON CONFLICT(history_id) DO UPDATE SET words=excluded.words, seconds=COALESCE(excluded.seconds,dictation_statistics.seconds)", params![id,day,words,seconds])?;
    Ok(())
}

pub fn read(conn: &Connection) -> Result<Vec<StatisticsDay>> {
    let mut query = conn.prepare("SELECT day, SUM(words), COUNT(*), COALESCE(SUM(seconds),0), COALESCE(SUM(CASE WHEN seconds > 0 THEN words ELSE 0 END),0) FROM dictation_statistics WHERE words > 0 GROUP BY day ORDER BY day")?;
    let rows = query.query_map([], |r| {
        Ok(StatisticsDay {
            date: r.get(0)?,
            words: r.get(1)?,
            sessions: r.get(2)?,
            seconds: r.get(3)?,
            measured_words: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counters_survive_history_deletion_and_retries_do_not_duplicate() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE transcription_history(id INTEGER PRIMARY KEY, timestamp INTEGER, transcription_text TEXT, file_name TEXT); INSERT INTO transcription_history VALUES (1,1700000000,'Hola, mundo —', 'missing.wav');").unwrap();
        initialize(&conn, Path::new("/nonexistent")).unwrap();
        initialize(&conn, Path::new("/nonexistent")).unwrap();
        assert_eq!(read(&conn).unwrap()[0].words, 2);
        record(
            &conn,
            1,
            1700000000,
            "Uno dos tres",
            Path::new("/missing.wav"),
        )
        .unwrap();
        conn.execute("DELETE FROM transcription_history", [])
            .unwrap();
        let days = read(&conn).unwrap();
        assert_eq!(days[0].words, 3);
        assert_eq!(days[0].sessions, 1);
        assert_eq!(days[0].measured_words, 0);
        assert_eq!(days[0].seconds, 0.0);
    }
}
