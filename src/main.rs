/// hippocampus CLI — 手动解析 args，无外部依赖

fn get_home() -> String {
    std::env::var("HIPPOCAMPUS_HOME").unwrap_or_else(|_| {
        std::env::var("HOME").map(|h| format!("{}/.hippocampus", h)).unwrap_or_else(|_| "./.hippocampus".to_string())
    })
}

fn print_json<T: serde::Serialize>(val: &T) {
    println!("{}", serde_json::to_string_pretty(val).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e)));
}

/// 从 args 中找 --flag 的值（支持 --flag value 格式）
fn arg_val(args: &[String], flag: &str) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}

/// 检查 flag 是否存在（bool flag）
fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let cmd = &args[1];
    let result = run_cmd(cmd, &args[2..]);
    if let Err(e) = result {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}

fn print_usage() {
    eprintln!("🧠 hippocampus — Biomimetic Cognitive Memory System v0.1.0");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  hippocampus init");
    eprintln!("  hippocampus remember --content \"...\" [--importance N] [--source S] [--tags \"a,b\"] [--layer L1] [--session-id X] [--permanent]");
    eprintln!("  hippocampus recall --query \"...\" [--top-k N] [--min-score F] [--include-l3] [--emotion E] [--with-context \"...\"]");
    eprintln!("  hippocampus reflect [--days N]");
    eprintln!("  hippocampus reconsolidate [--days N] [--dry-run]");
    eprintln!("  hippocampus dedup [--dry-run] [--similarity F]");
    eprintln!("  hippocampus learn-synonyms [--dry-run] [--top-k N]");
    eprintln!("  hippocampus gate --message \"...\" [--write] [--force] [--session-id X]");
    eprintln!("  hippocampus install [--openclaw] [--claude] [--all]");
    eprintln!("  hippocampus stats");
    eprintln!("  hippocampus learned [--top N] [--reset]");
    eprintln!("  hippocampus import --source PATH [--dry-run] [--clean-tests] [--min-importance N]");
  eprintln!("  hippocampus vacuum");
    eprintln!();
    eprintln!("Env: HIPPOCAMPUS_HOME (default: ~/.hippocampus)");
}

fn run_cmd(cmd: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        "init" => cmd_init(),
        "remember" => cmd_remember(args),
        "recall" => cmd_recall(args),
        "reflect" => cmd_reflect(args),
        "reconsolidate" => cmd_reconsolidate(args),
        "dedup" => cmd_dedup(args),
        "learn-synonyms" => cmd_learn_synonyms(args),
        "gate" => cmd_gate(args),
        "stats" => cmd_stats(),
        "install" => cmd_install(args),
        "import" => cmd_import(args),
        "vacuum" => cmd_vacuum(),
        "learned" => cmd_learned(args),
        _ => {
            eprintln!("Unknown command: {}", cmd);
            print_usage();
            Err("unknown command".into())
        }
    }
}

fn cmd_init() -> Result<(), Box<dyn std::error::Error>> {
    let home = get_home();
    hippocampus::Hippocampus::init(&home)?;
    print_json(&serde_json::json!({
        "status": "ok",
        "data_dir": home,
        "message": "🧠 Hippocampus 初始化完成"
    }));
    Ok(())
}

fn cmd_remember(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let content = arg_val(args, "--content").ok_or("--content required")?;
    let importance: u8 = arg_val(args, "--importance").and_then(|v| v.parse().ok()).unwrap_or(5);
    let source = arg_val(args, "--source").unwrap_or_else(|| "manual".to_string());
    let tags_str = arg_val(args, "--tags").unwrap_or_default();
    let tags: Vec<&str> = if tags_str.is_empty() { vec![] } else { tags_str.split(',').collect() };
    let layer = arg_val(args, "--layer").unwrap_or_else(|| "L1".to_string());
    let session_id = arg_val(args, "--session-id");
    let permanent = has_flag(args, "--permanent");

    let home = get_home();
    let mut hippo = hippocampus::Hippocampus::new(&home)?;
    let id = hippo.remember(&content, importance, &source, &tags, session_id.as_deref(), &layer, permanent)?;
    print_json(&serde_json::json!({
        "status": "ok",
        "engram_id": id,
        "content": content,
        "importance": importance,
        "layer": layer,
        "permanent": permanent,
    }));
    Ok(())
}

fn cmd_recall(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let query = arg_val(args, "--query").ok_or("--query required")?;
    let top_k: usize = arg_val(args, "--top-k").and_then(|v| v.parse().ok()).unwrap_or(5);
    let min_score: f64 = arg_val(args, "--min-score").and_then(|v| v.parse().ok()).unwrap_or(0.01);
    let include_l3 = !has_flag(args, "--l1l2-only");
    let emotion_filter = arg_val(args, "--emotion");
    let with_context = arg_val(args, "--with-context");
    let brief = has_flag(args, "--brief");

    let home = get_home();
    let hippo = hippocampus::Hippocampus::new(&home)?;
    let results = hippo.recall(
        &query,
        top_k,
        min_score,
        include_l3,
        emotion_filter.as_deref(),
        with_context.as_deref(),
    );
    if brief {
        for r in &results {
            let score = r.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let layer = r.get("layer").and_then(|v| v.as_str()).unwrap_or("?");
            let content = r.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let preview: String = content.chars().take(100).collect();
            println!("[{:.4}] [{}] {}", score, layer, preview);
        }
    } else {
        print_json(&serde_json::json!({
            "status": "ok",
            "query": query,
            "top_k": top_k,
            "results_count": results.len(),
            "results": results,
        }));
    }
    Ok(())
}

fn cmd_reflect(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let days: i64 = arg_val(args, "--days").and_then(|v| v.parse().ok()).unwrap_or(7);
    let home = get_home();
    let mut hippo = hippocampus::Hippocampus::new(&home)?;
    let result = hippo.reflect(days)?;
    print_json(&serde_json::json!({
        "status": "ok",
        "days": days,
        "semantic_network_learned": result.semantic_network_learned,
        "pruned": result.pruned,
        "reconsolidated": result.reconsolidated,
        "vacuum": result.vacuum,
    }));
    Ok(())
}

fn cmd_reconsolidate(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let days: i64 = arg_val(args, "--days").and_then(|v| v.parse().ok()).unwrap_or(30);
    let _dry_run = has_flag(args, "--dry-run");

    let home = get_home();
    let mut hippo = hippocampus::Hippocampus::new(&home)?;
    let result = hippo.reflect(days)?;
    print_json(&serde_json::json!({
        "status": "ok",
        "days": days,
        "reconsolidated": result.reconsolidated,
        "vacuum": result.vacuum,
    }));
    Ok(())
}

fn cmd_dedup(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let similarity: f64 = arg_val(args, "--similarity").and_then(|v| v.parse().ok()).unwrap_or(0.7);
    let _dry_run = has_flag(args, "--dry-run");

    let home = get_home();
    let hippo = hippocampus::Hippocampus::new(&home)?;
    let pairs = hippo.find_duplicates(similarity);
    print_json(&serde_json::json!({
        "status": "ok",
        "similarity_threshold": similarity,
        "duplicate_pairs_found": pairs.len(),
        "pairs": pairs,
    }));
    Ok(())
}

fn cmd_learn_synonyms(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let _dry_run = has_flag(args, "--dry-run");
    let _top_k: usize = arg_val(args, "--top-k").and_then(|v| v.parse().ok()).unwrap_or(20);

    let home = get_home();
    let mut hippo = hippocampus::Hippocampus::new(&home)?;
    let result = hippo.reflect(1)?;
    print_json(&serde_json::json!({
        "status": "ok",
        "semantic_network_learned": result.semantic_network_learned,
        "pruned": result.pruned,
        "message": "语义网络已从近期记忆中学习同义词关联"
    }));
    Ok(())
}

fn cmd_gate(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let message = arg_val(args, "--message").ok_or("--message required")?;
    let do_write = has_flag(args, "--write");
    let force = has_flag(args, "--force");
    let session_id = arg_val(args, "--session-id");

    let home = get_home();
    let mut hippo = hippocampus::Hippocampus::new(&home)?;

    let decision = if do_write || force {
        hippo.auto_remember(&message, "dialogue", session_id.as_deref(), force)?
    } else {
        hippo.should_remember(&message)
    };

    print_json(&serde_json::json!({
        "status": "ok",
        "should_remember": decision.should_remember,
        "importance": decision.importance,
        "emotion": decision.emotion,
        "emotion_score": decision.emotion_score,
        "decision_score": decision.decision_score,
        "reason": decision.reason,
        "tags": decision.tags,
        "written": do_write || force,
        "components": {
            "amygdala": { "score": decision.components.amygdala.score, "reason": decision.components.amygdala.reason },
            "hippocampus": { "score": decision.components.hippocampus.score, "reason": decision.components.hippocampus.reason },
            "prefrontal": { "score": decision.components.prefrontal.score, "reason": decision.components.prefrontal.reason },
            "temporal": { "score": decision.components.temporal.score, "reason": decision.components.temporal.reason },
        },
    }));
    Ok(())
}

fn cmd_stats() -> Result<(), Box<dyn std::error::Error>> {
    let home = get_home();
    let hippo = hippocampus::Hippocampus::new(&home)?;
    let stats = hippo.stats();
    print_json(&serde_json::json!({
        "status": "ok",
        "total_engrams": stats.total,
        "by_layer": stats.by_layer,
        "avg_access_count": stats.avg_access_count,
        "avg_importance": stats.avg_importance,
    }));
    Ok(())
}

fn cmd_install(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;
    use std::io::Write;

    let openclaw = args.contains(&"--openclaw".to_string()) || args.contains(&"--all".to_string());
    let claude = args.contains(&"--claude".to_string()) || args.contains(&"--all".to_string());

    if !openclaw && !claude {
        return cmd_install(&["--all".to_string()]);
    }

    let mut installed = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    if openclaw {
        let home_dir = std::env::var("HOME").unwrap_or_default();
        let openclaw_skills = format!("{}/.openclaw/workspace/skills/hippocampus", home_dir);
        let src_skill = std::env::current_dir().unwrap_or_default().join("adapters/openclaw/SKILL.md");

        // 1. SKILL.md symlink
        let _ = std::fs::create_dir_all(&openclaw_skills);
        let link_path = format!("{}/SKILL.md", openclaw_skills);
        let _ = std::fs::remove_file(&link_path);
        match std::os::unix::fs::symlink(src_skill.to_str().unwrap_or(""), &link_path) {
            Ok(_) => installed.push("OpenClaw SKILL.md".to_string()),
            Err(e) => {
                if std::fs::copy(&src_skill, &link_path).is_ok() {
                    installed.push("OpenClaw SKILL.md (copied)".to_string());
                } else {
                    failed.push(format!("OpenClaw SKILL.md: {}", e));
                }
            }
        }

        // 2. HIPPOCAMPUS_HOME to .bashrc
        let bashrc_path = format!("{}/.bashrc", home_dir);
        let env_line = "export HIPPOCAMPUS_HOME=/home/bot/.openclaw/workspace/cognitive_memory";
        if let Ok(bashrc) = std::fs::read_to_string(&bashrc_path) {
            if bashrc.contains("HIPPOCAMPUS_HOME") {
                skipped.push("OpenClaw HIPPOCAMPUS_HOME (已配置)".to_string());
            } else if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&bashrc_path) {
                let _ = writeln!(f, "\n# Hippocampus\n{}", env_line);
                installed.push("OpenClaw HIPPOCAMPUS_HOME".to_string());
            }
        }

        // 3. Configure crons in openclaw.json
        let openclaw_json_path = format!("{}/.openclaw/openclaw.json", home_dir);
        if let Ok(config_str) = std::fs::read_to_string(&openclaw_json_path) {
            if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&config_str) {
                let has_hippo = config.get("crons").and_then(|c| c.as_array()).map_or(false, |arr| {
                    arr.iter().any(|c| {
                        c.get("name").and_then(|n| n.as_str()).map_or(false, |name| {
                            name.contains("hippocampus") || name.contains("reflect") || name.contains("vacuum")
                        })
                    })
                });
                let crons = config.get_mut("crons").and_then(|c| c.as_array_mut());
                if has_hippo {
                    skipped.push("OpenClaw crons (已配置)".to_string());
                } else if let Some(crons_arr) = crons {
                    crons_arr.push(serde_json::json!({ "name": "Hippocampus_Reflect_2230", "schedule": "30 22 * * *", "enabled": true, "task": "exec hippocampus reflect --days 3" }));
                    crons_arr.push(serde_json::json!({ "name": "Hippocampus_Vacuum_Monthly", "schedule": "0 3 1 * *", "enabled": true, "task": "exec hippocampus vacuum" }));
                    installed.push("OpenClaw crons (reflect + vacuum)".to_string());
                } else {
                    failed.push("OpenClaw crons: crons 字段不存在".to_string());
                }
                // write back
                if let Ok(pretty) = serde_json::to_string_pretty(&config) {
                    let _ = std::fs::write(&openclaw_json_path, pretty);
                }
            }
        }
    }

    if claude {
        let home_dir = std::env::var("HOME").unwrap_or_default();
        let cwd = std::env::current_dir().unwrap_or_default();

        // 1. SKILL.md from adapters/claude/ → target project .claude/skills/hippocampus/
        let repo_dir = std::env::current_dir().unwrap_or_default();
        let src_skill = repo_dir.join("adapters/claude/SKILL.md");
        let target_skill_dir = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(format!("{}/.claude/skills/hippocampus", h)))
            .unwrap_or_else(|_| std::path::PathBuf::from(".claude/skills/hippocampus"));
        let target_skill_md = target_skill_dir.join("SKILL.md");
        if target_skill_md.exists() {
            skipped.push("Claude SKILL.md (已存在)".to_string());
        } else if src_skill.exists() {
            let _ = std::fs::create_dir_all(&target_skill_dir);
            if std::fs::copy(&src_skill, &target_skill_md).is_ok() {
                installed.push("Claude SKILL.md".to_string());
            } else {
                failed.push("Claude SKILL.md: 复制失败".to_string());
            }
        } else {
            failed.push("Claude: adapters/claude/SKILL.md 不存在".to_string());
        }

        // 2. Global CLAUDE.md — append hint
        let global = format!("{}/.claude/CLAUDE.md", home_dir);
        if let Some(parent) = Path::new(&global).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if Path::new(&global).exists() {
            let content = std::fs::read_to_string(&global).unwrap_or_default();
            if content.contains("hippocampus skill") {
                skipped.push("Claude 全局 CLAUDE.md (已包含)".to_string());
            } else if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&global) {
                let _ = write!(f, "\n- 使用 hippocampus skill 管理记忆\n");
                installed.push("Claude 全局 CLAUDE.md".to_string());
            }
        } else {
            let _ = std::fs::write(&global, "- 使用 hippocampus skill 管理记忆\n");
            installed.push("Claude 全局 CLAUDE.md".to_string());
        }

        // 3. Hooks — merge into ~/.claude/settings.json
        let claude_settings = format!("{}/.claude/settings.json", home_dir);
        let hooks_src = cwd.join("adapters/claude/hooks.json");
        if let Ok(hooks_content) = std::fs::read_to_string(&hooks_src) {
            if let Ok(hooks_val) = serde_json::from_str::<serde_json::Value>(&hooks_content) {
                if let Some(hooks_hooks) = hooks_val.get("hooks").cloned() {
                    if Path::new(&claude_settings).exists() {
                        if let Ok(settings_content) = std::fs::read_to_string(&claude_settings) {
                            if let Ok(mut settings) = serde_json::from_str::<serde_json::Value>(&settings_content) {
                                if let Some(existing) = settings.get_mut("hooks") {
                                    // merge hooks entries
                                    if let (Some(target), Some(source)) = (existing.as_object_mut(), hooks_hooks.as_object()) {
                                        for (k, v) in source {
                                            target.insert(k.clone(), v.clone());
                                        }
                                    }
                                } else {
                                    settings.as_object_mut().map(|o| o.insert("hooks".to_string(), hooks_hooks));
                                }
                                if let Ok(pretty) = serde_json::to_string_pretty(&settings) {
                                    if std::fs::write(&claude_settings, pretty).is_ok() {
                                        installed.push("Claude hooks 配置（已合并）".to_string());
                                    }
                                }
                            }
                        }
                    } else {
                        if let Some(parent) = Path::new(&claude_settings).parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Ok(pretty) = serde_json::to_string_pretty(&hooks_val) {
                            if std::fs::write(&claude_settings, pretty).is_ok() {
                                installed.push("Claude hooks 配置".to_string());
                            }
                        }
                    }
                }
            }
        } else {
            failed.push("Claude: adapters/claude/hooks.json 不存在".to_string());
        }
    }

    let status = if failed.is_empty() { "ok" } else { "partial" };
    let msg = if failed.is_empty() { "✅ 安装完成".to_string() } else { format!("⚠️ {} 项失败", failed.len()) };
    print_json(&serde_json::json!({"status": status, "installed": installed, "skipped": skipped, "failed": failed, "message": msg}));
    Ok(())
}

fn cmd_learned(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let home = get_home();
    let config = hippocampus::HippocampusConfig::new(None, Some(&home));
    let path = &config.learned_keywords_path;

    if has_flag(args, "--reset") {
        std::fs::remove_file(path)?;
        print_json(&serde_json::json!({
            "status": "ok",
            "message": "🗑️ 学习数据已清空"
        }));
        return Ok(());
    }

    let top_n: usize = arg_val(args, "--top").and_then(|v| v.parse().ok()).unwrap_or(50);
    let learned = hippocampus::LearnedKeywords::load(path);
    let (word_count, cooc_count) = learned.stats();
    let top = learned.top_keywords(top_n);

    print_json(&serde_json::json!({
        "status": "ok",
        "total_words": word_count,
        "total_cooccurrences": cooc_count,
        "top_keywords": top.iter().map(|(w, boost, freq)| serde_json::json!({
            "word": w,
            "boost": boost,
            "freq": freq,
            "intent": learned.cooccurrence.get(w).map(|e| e.with_intent).unwrap_or(0),
            "decision": learned.cooccurrence.get(w).map(|e| e.with_decision).unwrap_or(0),
        })).collect::<Vec<_>>()
    }));
    Ok(())
}

fn cmd_import(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;

    let source_path = arg_val(args, "--source").ok_or("--source required")?;
    let dry_run = has_flag(args, "--dry-run");
    let clean_tests = has_flag(args, "--clean-tests");
    let min_importance: u8 = arg_val(args, "--min-importance").and_then(|v| v.parse().ok()).unwrap_or(3);

    let home = get_home();
    let config_path = Path::new(&home).join("config.json");
    if !config_path.exists() {
        return Err("Hippocampus 未初始化，请先运行 hippocampus init".into());
    }
    if !Path::new(&source_path).is_dir() {
        return Err(format!("源目录不存在: {}", source_path).into());
    }

    let mut hippo = hippocampus::Hippocampus::new(&home)?;
    let mut details = Vec::new();
    let files_scanned: u32;
    let mut files_imported = 0u32;
    let mut engrams_created = 0u32;
    let mut tests_cleaned = 0u32;
    let mut all_imported_contents = Vec::new();

    // --clean-tests: remove engrams containing "测试"
    if clean_tests && !dry_run {
        for layer in &["L1", "L2", "L3"] {
            let path = Path::new(&home).join(format!("engrams_{}.jsonl", layer));
            if let Ok(content) = std::fs::read_to_string(&path) {
                let original_lines: Vec<&str> = content.lines().collect();
                let kept: Vec<String> = original_lines.iter()
                    .filter(|line| !line.contains("测试"))
                    .map(|s| s.to_string())
                    .collect();
                tests_cleaned += (original_lines.len() - kept.len()) as u32;
                std::fs::write(&path, kept.join("\n"))?;
            }
        }
    } else if clean_tests && dry_run {
        // dry-run: count
        for layer in &["L1", "L2", "L3"] {
            let path = Path::new(&home).join(format!("engrams_{}.jsonl", layer));
            if let Ok(content) = std::fs::read_to_string(&path) {
                tests_cleaned += content.lines().filter(|l| l.contains("测试")).count() as u32;
            }
        }
    }

    // Scan .md files
    let entries: Vec<std::fs::DirEntry> = std::fs::read_dir(&source_path)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map_or(false, |ext| ext == "md")
        })
        .collect();

    files_scanned = entries.len() as u32;

    for entry in &entries {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let file_content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if file_content.len() < 20 {
            continue;
        }

        // Classify
        let (layer, importance, tags) = classify_file(&file_name, &file_content);

        if importance < min_importance {
            continue;
        }

        // Split by ## headings
        let sections = split_by_headings(&file_content);
        let section_count = sections.len();

        for section in &sections {
            let truncated = if section.chars().count() > 1000 {
                section.chars().take(1000).collect()
            } else {
                section.clone()
            };

            if !dry_run {
                hippo.remember(
                    &truncated,
                    importance,
                    &format!("import:{}", file_name),
                    tags,
                    None,
                    layer,
                    layer == "L3",
                )?;
                engrams_created += 1;
            }
            all_imported_contents.push(truncated);
        }

        files_imported += 1;
        details.push(serde_json::json!({
            "file": file_name,
            "sections": section_count,
            "layer": layer,
            "importance": importance,
        }));
    }

    // Learn keywords from imported content
    let mut learned_words = 0u32;
    if !dry_run && !all_imported_contents.is_empty() {
        let lk_config = hippocampus::HippocampusConfig::new(None, Some(&home));
        let mut learned = hippocampus::LearnedKeywords::load(&lk_config.learned_keywords_path);
        for content in &all_imported_contents {
            learned.update_from_engram(content);
        }
        learned.refine();
        learned.save(&lk_config.learned_keywords_path)?;
        learned_words = learned.word_freq.len() as u32;
    }

    print_json(&serde_json::json!({
        "status": "ok",
        "source_dir": source_path,
        "dry_run": dry_run,
        "files_scanned": files_scanned,
        "files_imported": files_imported,
        "engrams_created": engrams_created,
        "tests_cleaned": tests_cleaned,
        "learned_words": learned_words,
        "details": details,
    }));
    Ok(())
}

/// Classify file by name, return (layer, importance, tags)
fn classify_file(name: &str, content: &str) -> (&'static str, u8, &'static [&'static str]) {
    // Check date format YYYY-MM-DD.md
    let is_date = name.len() == 14
        && name.chars().nth(4) == Some('-')
        && name.chars().nth(7) == Some('-')
        && name.ends_with(".md");

    if name == "MEMORY.md" {
        return ("L3", 8, &["核心记忆"][..]);
    }
    if name == "SECURITY_RULES.md" {
        return ("L3", 9, &["安全规则"][..]);
    }
    if name.starts_with("fund_names") {
        return ("L3", 7, &["基金"][..]);
    }
    if name.starts_with("financial-knowledge") {
        return ("L3", 6, &["金融知识"][..]);
    }
    if name.starts_with("financial-quick-ref") {
        return ("L3", 6, &["金融参考"][..]);
    }
    if name.starts_with("insights") {
        return ("L2", 5, &["洞察"][..]);
    }
    if name.starts_with("chansha") || name.contains("plan") {
        return ("L2", 5, &["计划"][..]);
    }
    if is_date {
        let len = content.chars().count();
        let imp = if len > 500 { 5 } else if len > 200 { 4 } else { 3 };
        return ("L2", imp, &["每日记录"][..]);
    }
    ("L2", 4, &["归档"][..])
}

/// Split content by ## headings
fn split_by_headings(content: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current = String::new();
    let mut found_heading = false;

    for line in content.lines() {
        if line.starts_with("## ") {
            if found_heading || !current.is_empty() {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    sections.push(trimmed);
                }
            }
            current = line.trim_start_matches("# ").to_string();
            found_heading = true;
        } else {
            current.push_str("\n");
            current.push_str(line);
        }
    }
    // Push last section
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        sections.push(trimmed);
    }

    if sections.is_empty() && !content.trim().is_empty() {
        sections.push(content.trim().to_string());
    }

    sections
}

fn cmd_vacuum() -> Result<(), Box<dyn std::error::Error>> {
    let home = get_home();
    let mut hippo = hippocampus::Hippocampus::new(&home)?;
    let result = hippo.vacuum()?;
    print_json(&serde_json::json!({
        "status": "ok",
        "vacuum": result,
    }));
    Ok(())
}
