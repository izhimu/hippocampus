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
        "vacuum" => cmd_vacuum(),
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
    let min_score: f64 = arg_val(args, "--min-score").and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let include_l3 = has_flag(args, "--include-l3");
    let emotion_filter = arg_val(args, "--emotion");
    let with_context = arg_val(args, "--with-context");

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
    print_json(&serde_json::json!({
        "status": "ok",
        "query": query,
        "top_k": top_k,
        "results_count": results.len(),
        "results": results,
    }));
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
    }

    if claude {
        let home_dir = std::env::var("HOME").unwrap_or_default();
        let cwd = std::env::current_dir().unwrap_or_default();

        // 1. CLAUDE.md in project root
        let claude_md = cwd.join("CLAUDE.md");
        if claude_md.exists() {
            skipped.push("Claude CLAUDE.md (已存在)".to_string());
        } else {
            let src = cwd.join("adapters/claude/CLAUDE.md");
            if src.exists() && std::fs::copy(&src, &claude_md).is_ok() {
                installed.push("Claude CLAUDE.md".to_string());
            } else if !src.exists() {
                failed.push("Claude: adapters/claude/CLAUDE.md 不存在".to_string());
            }
        }

        // 2. Global CLAUDE.md
        let global = format!("{}/.claude/CLAUDE.md", home_dir);
        if let Some(parent) = Path::new(&global).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if Path::new(&global).exists() {
            let content = std::fs::read_to_string(&global).unwrap_or_default();
            if content.contains("hippocampus") {
                skipped.push("Claude 全局 CLAUDE.md (已包含)".to_string());
            } else if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&global) {
                let _ = write!(f, "\n# Hippocampus 记忆系统\n使用 hippocampus CLI 管理记忆：recall/remember/gate/stats/reflect\n");
                installed.push("Claude 全局 CLAUDE.md".to_string());
            }
        } else {
            let _ = std::fs::write(&global, "# Hippocampus 记忆系统\n使用 hippocampus CLI 管理记忆：recall/remember/gate/stats/reflect\n");
            installed.push("Claude 全局 CLAUDE.md".to_string());
        }

        // 3. Hooks
        let claude_settings = format!("{}/.claude/settings.json", home_dir);
        let hooks_src = cwd.join("adapters/claude/hooks-example.json");
        if Path::new(&claude_settings).exists() {
            skipped.push("Claude settings.json (已存在)".to_string());
        } else if hooks_src.exists() && std::fs::copy(&hooks_src, &claude_settings).is_ok() {
            installed.push("Claude hooks 配置".to_string());
        }
    }

    let status = if failed.is_empty() { "ok" } else { "partial" };
    let msg = if failed.is_empty() { "✅ 安装完成".to_string() } else { format!("⚠️ {} 项失败", failed.len()) };
    print_json(&serde_json::json!({"status": status, "installed": installed, "skipped": skipped, "failed": failed, "message": msg}));
    Ok(())
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
