/// session — 多轮对话会话追踪

use serde::{Serialize, Deserialize};
use std::fs;
use std::io::{self, BufRead, Write};

use crate::store::EngramStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub started_at: String,
    pub last_active_at: String,
    pub message_count: u32,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub status: String, // "active" | "closed" | "summarized"
}

pub struct SessionManager {
    store: EngramStore,
    sessions_path: String,
}

impl SessionManager {
    pub fn new(store: EngramStore, sessions_path: impl Into<String>) -> Self {
        let sessions_path = sessions_path.into();
        if let Some(parent) = std::path::Path::new(&sessions_path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        Self { store, sessions_path }
    }

    /// 创建新会话
    pub fn start_session(&mut self) -> io::Result<Session> {
        let session = Session {
            id: uuid_hex(),
            started_at: now_iso(),
            last_active_at: now_iso(),
            message_count: 0,
            summary: None,
            tags: vec![],
            status: "active".into(),
        };
        self.append_session(&session)?;
        Ok(session)
    }

    /// 关闭会话（带摘要）
    pub fn end_session(&mut self, session_id: &str, summary: Option<&str>) -> io::Result<bool> {
        let mut sessions = self.read_all_sessions()?;
        let mut found = false;
        for s in &mut sessions {
            if s.id == session_id {
                s.status = "closed".into();
                s.last_active_at = now_iso();
                if let Some(sum) = summary {
                    s.summary = Some(sum.into());
                }
                found = true;
                break;
            }
        }
        if found {
            self.write_all_sessions(&sessions)?;
        }
        Ok(found)
    }

    /// 获取单个会话
    pub fn get_session(&self, session_id: &str) -> io::Result<Option<Session>> {
        let sessions = self.read_all_sessions()?;
        Ok(sessions.into_iter().find(|s| s.id == session_id))
    }

    /// 获取会话关联的 engrams
    pub fn get_session_engrams(&self, session_id: &str) -> io::Result<Vec<crate::engram::Engram>> {
        let all = self.store.read_all()?;
        Ok(all.into_iter().filter(|e| {
            e.session_id.as_deref() == Some(session_id)
        }).collect())
    }

    /// 活跃会话（按 last_active_at 排序）
    pub fn active_sessions(&self) -> io::Result<Vec<Session>> {
        let mut sessions: Vec<Session> = self.read_all_sessions()?
            .into_iter()
            .filter(|s| s.status == "active")
            .collect();
        sessions.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at));
        Ok(sessions)
    }

    fn read_all_sessions(&self) -> io::Result<Vec<Session>> {
        let path = std::path::Path::new(&self.sessions_path);
        if !path.exists() {
            return Ok(vec![]);
        }
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let mut sessions = vec![];
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if let Ok(s) = serde_json::from_str::<Session>(&line.trim()) {
                sessions.push(s);
            }
        }
        Ok(sessions)
    }

    fn append_session(&self, session: &Session) -> io::Result<()> {
        let mut file = fs::OpenOptions::new().create(true).append(true).open(&self.sessions_path)?;
        let line = serde_json::to_string(session).unwrap() + "\n";
        file.write_all(line.as_bytes())
    }

    fn write_all_sessions(&self, sessions: &[Session]) -> io::Result<()> {
        let mut file = fs::File::create(&self.sessions_path)?;
        for s in sessions {
            let line = serde_json::to_string(s).unwrap() + "\n";
            file.write_all(line.as_bytes())?;
        }
        Ok(())
    }
}

fn now_iso() -> String {
    crate::engram::chrono_now_iso()
}

fn uuid_hex() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:016x}{:016x}", nanos, nanos.wrapping_mul(0x9e3779b97f4a7c15))
}
