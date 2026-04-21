use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};
use crate::config::HippocampusConfig;
use crate::engram::Engram;

/// JSONL 分层存储层 with in-memory cache
pub struct EngramStore {
    config: HippocampusConfig,
    /// In-memory cache: id → Engram, populated lazily
    cache: std::cell::RefCell<Option<HashMap<String, Engram>>>,
    /// Tracks which layers have been modified and need flushing
    dirty_layers: std::cell::RefCell<std::collections::HashSet<String>>,
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
        Ok(Self {
            config,
            cache: std::cell::RefCell::new(None),
            dirty_layers: std::cell::RefCell::new(std::collections::HashSet::new()),
        })
    }

    fn ensure_layer(layer: &str) -> &str {
        match layer {
            "L1" | "L2" | "L3" => layer,
            _ => "L1",
        }
    }

    /// Ensure the cache is populated (lazy load from disk)
    fn ensure_cache(&self) {
        if self.cache.borrow().is_none() {
            let mut map = HashMap::new();
            for layer in &["L1", "L2", "L3"] {
                if let Ok(engrams) = self.read_layer_raw(layer) {
                    for e in engrams {
                        map.insert(e.id.clone(), e);
                    }
                }
            }
            *self.cache.borrow_mut() = Some(map);
        }
    }

    /// Invalidate cache — public, for cases where external mutation invalidates cache
    #[allow(dead_code)]
    fn invalidate_cache(&self) {
        *self.cache.borrow_mut() = None;
    }

    /// Mark a layer as dirty (needs flush to disk)
    fn mark_dirty(&self, layer: &str) {
        self.dirty_layers.borrow_mut().insert(layer.to_string());
    }

    /// Flush dirty layers to disk from cache
    fn flush_dirty(&self) {
        let dirty: Vec<String> = self.dirty_layers.borrow_mut().drain().collect();
        if dirty.is_empty() {
            return;
        }
        let cache = self.cache.borrow();
        let Some(ref map) = *cache else { return };

        for layer in &dirty {
            let layer = Self::ensure_layer(layer);
            let rows: Vec<&Engram> = map.values().filter(|e| e.layer == layer).collect();
            if let Err(e) = self.write_layer(layer, &rows.iter().map(|e| (*e).clone()).collect::<Vec<_>>()) {
                eprintln!("Warning: failed to flush layer {}: {}", layer, e);
            }
        }
    }

    /// Read a single layer from disk (raw, bypasses cache)
    fn read_layer_raw(&self, layer: &str) -> io::Result<Vec<Engram>> {
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

    /// Read a single layer (uses cache when available)
    pub fn read_layer(&self, layer: &str) -> io::Result<Vec<Engram>> {
        let layer = Self::ensure_layer(layer);
        self.ensure_cache();
        let cache = self.cache.borrow();
        let map = cache.as_ref().unwrap();
        Ok(map.values().filter(|e| e.layer == layer).cloned().collect())
    }

    /// Read all layers (uses cache)
    pub fn read_all(&self) -> io::Result<Vec<Engram>> {
        self.ensure_cache();
        let cache = self.cache.borrow();
        let map = cache.as_ref().unwrap();
        Ok(map.values().cloned().collect())
    }

    /// Append a single engram
    pub fn append(&self, engram: &Engram) -> io::Result<String> {
        self.ensure_cache();
        self.cache.borrow_mut().as_mut().unwrap().insert(engram.id.clone(), engram.clone());
        self.mark_dirty(&engram.layer);

        // Also write to disk immediately for durability
        let layer = Self::ensure_layer(&engram.layer);
        let path = self.config.layer_path(layer);
        let mut file = fs::OpenOptions::new().create(true).append(true).open(&path)?;
        let line = serde_json::to_string(engram).unwrap() + "\n";
        file.write_all(line.as_bytes())?;
        Ok(engram.id.clone())
    }

    /// Batch append
    pub fn append_batch(&self, engrams: &[Engram]) -> io::Result<Vec<String>> {
        self.ensure_cache();
        let mut by_layer: HashMap<&str, Vec<&Engram>> = HashMap::new();
        let mut ids = vec![];

        for e in engrams {
            let layer = Self::ensure_layer(&e.layer);
            by_layer.entry(layer).or_default().push(e);
            ids.push(e.id.clone());
            self.cache.borrow_mut().as_mut().unwrap().insert(e.id.clone(), e.clone());
            self.mark_dirty(&e.layer);
        }

        // Write to disk for durability
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

    /// Update an engram by id
    pub fn update<F>(&self, eid: &str, mutator: F) -> io::Result<bool>
    where
        F: FnOnce(&mut Engram),
    {
        self.ensure_cache();

        let old_layer = {
            let cache = self.cache.borrow();
            let map = cache.as_ref().unwrap();
            match map.get(eid) {
                Some(e) => e.layer.clone(),
                None => return Ok(false),
            }
        };

        {
            let mut cache = self.cache.borrow_mut();
            let map = cache.as_mut().unwrap();
            let engram = match map.get_mut(eid) {
                Some(e) => e,
                None => return Ok(false),
            };
            mutator(engram);
        }

        let new_layer = {
            let cache = self.cache.borrow();
            let map = cache.as_ref().unwrap();
            map.get(eid).unwrap().layer.clone()
        };

        if old_layer != new_layer {
            self.mark_dirty(&old_layer);
        }
        self.mark_dirty(&new_layer);
        self.flush_dirty();
        Ok(true)
    }

    /// Batch update multiple engrams
    pub fn batch_update<F, V>(&self, updates: &HashMap<String, V>, mutator: F) -> io::Result<()>
    where
        F: Fn(&mut Engram, &V),
    {
        self.ensure_cache();

        let mut touched_layers = std::collections::HashSet::new();
        {
            let mut cache = self.cache.borrow_mut();
            let map = cache.as_mut().unwrap();
            for (id, val) in updates {
                if let Some(row) = map.get_mut(id) {
                    let layer = row.layer.clone();
                    mutator(row, val);
                    touched_layers.insert(layer);
                }
            }
        }

        for layer in &touched_layers {
            self.mark_dirty(layer);
        }
        self.flush_dirty();
        Ok(())
    }

    /// Delete an engram by id
    pub fn delete(&self, eid: &str) -> io::Result<bool> {
        self.ensure_cache();

        let layer = {
            let mut cache = self.cache.borrow_mut();
            let map = cache.as_mut().unwrap();
            match map.remove(eid) {
                Some(e) => e.layer,
                None => return Ok(false),
            }
        };

        self.mark_dirty(&layer);
        self.flush_dirty();
        Ok(true)
    }

    /// Get an engram by id (uses cache)
    pub fn get_by_id(&self, eid: &str) -> io::Result<Option<Engram>> {
        self.ensure_cache();
        let cache = self.cache.borrow();
        Ok(cache.as_ref().unwrap().get(eid).cloned())
    }

    /// Statistics (uses cache)
    pub fn stats(&self) -> io::Result<StoreStats> {
        self.ensure_cache();
        let cache = self.cache.borrow();
        let map = cache.as_ref().unwrap();

        let mut stats = StoreStats::default();
        let mut total_ac = 0u64;
        let mut total_imp = 0u64;

        for e in map.values() {
            *stats.by_layer.entry(e.layer.clone()).or_insert(0) += 1;
            stats.total += 1;
            total_ac += e.access_count as u64;
            total_imp += e.importance as u64;
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
