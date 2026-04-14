use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::config::HippocampusConfig;
use crate::engram::Engram;

/// JSONL 分层存储层
pub struct EngramStore {
    config: HippocampusConfig,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct StoreStats {
    pub total: usize,
    pub by_layer: HashMap<String, usize>,
    pub avg_access_count: f64,
    pub avg_importance: f64,
}

impl EngramStore {
    pub fn new(config: HippocampusConfig) -> io::Result<Self> {
        fs::create_dir_all(&config.cognitive_dir)?;
        Ok(Self { config })
    }

    fn ensure_layer(layer: &str) -> &str {
        match layer {
            "L1" | "L2" | "L3" => layer,
            _ => "L1",
        }
    }

    /// 读取单个 layer 的所有 engram
    pub fn read_layer(&self, layer: &str) -> io::Result<Vec<Engram>> {
        let layer = Self::ensure_layer(layer);
        let path = self.config.layer_path(layer);
        if !path.exists() {
            return Ok(vec![]);
        }
        let file = fs::File::open(&path)?;
        let reader = io::BufReader::new(file);
        let mut engrams = vec![];
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(e) = serde_json::from_str::<Engram>(line) {
                engrams.push(e);
            }
        }
        Ok(engrams)
    }

    /// 读取所有 layer（L1+L2+L3），去重
    pub fn read_all(&self) -> io::Result<Vec<Engram>> {
        let mut result = vec![];
        let mut seen = HashMap::new();
        for layer in &["L1", "L2", "L3"] {
            for e in self.read_layer(layer)? {
                if seen.insert(e.id.clone(), result.len()).is_none() {
                    seen.insert(e.id.clone(), result.len());
                    result.push(e);
                }
            }
        }
        Ok(result)
    }

    /// 追加单个 engram 到对应 layer
    pub fn append(&self, engram: &Engram) -> io::Result<String> {
        let layer = Self::ensure_layer(&engram.layer);
        let path = self.config.layer_path(layer);
        let mut file = fs::OpenOptions::new().create(true).append(true).open(&path)?;
        let line = serde_json::to_string(engram).unwrap() + "\n";
        file.write_all(line.as_bytes())?;
        Ok(engram.id.clone())
    }

    /// 批量追加
    pub fn append_batch(&self, engrams: &[Engram]) -> io::Result<Vec<String>> {
        let mut by_layer: HashMap<&str, Vec<&Engram>> = HashMap::new();
        let mut ids = vec![];
        for e in engrams {
            let layer = Self::ensure_layer(&e.layer);
            by_layer.entry(layer).or_default().push(e);
            ids.push(e.id.clone());
        }
        for (layer, items) in &by_layer {
            let path = self.config.layer_path(layer);
            let mut file = fs::OpenOptions::new().create(true).append(true).open(path)?;
            for e in items.iter() {
                let line = serde_json::to_string(e).unwrap() + "\n";
                file.write_all(line.as_bytes())?;
            }
        }
        Ok(ids)
    }

    /// 更新指定 id 的 engram
    pub fn update<F>(&self, eid: &str, mutator: F) -> io::Result<bool>
    where
        F: FnOnce(&mut Engram),
    {
        // 在所有 layer 中查找
        let mut found_layer = None;
        let mut found_engram = None;
        for layer in &["L1", "L2", "L3"] {
            for e in self.read_layer(layer)? {
                if e.id == eid {
                    found_layer = Some(layer.to_string());
                    found_engram = Some(e);
                    break;
                }
            }
            if found_layer.is_some() {
                break;
            }
        }

        let (old_layer, mut engram) = match (found_layer, found_engram) {
            (Some(l), Some(e)) => (l, e),
            _ => return Ok(false),
        };

        mutator(&mut engram);
        let new_layer = Self::ensure_layer(&engram.layer).to_string();

        if old_layer == new_layer {
            // 同层更新
            let mut rows = self.read_layer(&old_layer)?;
            let mut updated = false;
            for r in &mut rows {
                if r.id == eid && !updated {
                    *r = engram.clone();
                    updated = true;
                }
            }
            self.write_layer(&old_layer, &rows)?;
        } else {
            // 跨层移动
            let old_rows: Vec<Engram> = self.read_layer(&old_layer)?
                .into_iter().filter(|e| e.id != eid).collect();
            self.write_layer(&old_layer, &old_rows)?;
            let mut new_rows = self.read_layer(&new_layer)?;
            new_rows.push(engram);
            self.write_layer(&new_layer, &new_rows)?;
        }

        Ok(true)
    }

    /// 删除指定 id 的 engram
    pub fn delete(&self, eid: &str) -> io::Result<bool> {
        for layer in &["L1", "L2", "L3"] {
            let rows = self.read_layer(layer)?;
            if rows.iter().any(|e| e.id == eid) {
                let new_rows: Vec<Engram> = rows.into_iter().filter(|e| e.id != eid).collect();
                self.write_layer(layer, &new_rows)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 按 id 查找
    pub fn get_by_id(&self, eid: &str) -> io::Result<Option<Engram>> {
        for layer in &["L1", "L2", "L3"] {
            for e in self.read_layer(layer)? {
                if e.id == eid {
                    return Ok(Some(e));
                }
            }
        }
        Ok(None)
    }

    /// 统计
    pub fn stats(&self) -> io::Result<StoreStats> {
        let mut stats = StoreStats::default();
        let mut total_ac = 0u64;
        let mut total_imp = 0u64;
        for layer in &["L1", "L2", "L3"] {
            let rows = self.read_layer(layer)?;
            let count = rows.len();
            stats.by_layer.insert(layer.to_string(), count);
            stats.total += count;
            for r in &rows {
                total_ac += r.access_count as u64;
                total_imp += r.importance as u64;
            }
        }
        if stats.total > 0 {
            stats.avg_access_count = total_ac as f64 / stats.total as f64;
            stats.avg_importance = total_imp as f64 / stats.total as f64;
        }
        Ok(stats)
    }

    fn write_layer(&self, layer: &str, rows: &[Engram]) -> io::Result<()> {
        let path = self.config.layer_path(layer);
        let mut file = fs::File::create(&path)?;
        for row in rows {
            let line = serde_json::to_string(row).unwrap() + "\n";
            file.write_all(line.as_bytes())?;
        }
        Ok(())
    }
}
