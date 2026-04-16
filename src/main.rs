/// hippocampus CLI — 手动解析 args，无外部依赖

use std::io::Read;

// Embed adapter files into binary for global install support
const OPENCLAW_HOOK_MD: &str = include_str!("../adapters/openclaw/HOOK.md");
const OPENCLAW_HANDLER_TS: &str = include_str!("../adapters/openclaw/handler.ts");

fn get_home() -> String {
    std::env::var("HIPPOCAMPUS_HOME").unwrap_or_else(|_| {
        std::env::var("HOME").map(|h| format!("{}/.hippocampus", h)).unwrap_or_else(|_| "./.hippocampus".to_string())
    })
}

fn print_json<T: serde::Serialize>(val: &T) {
    println!("{}", serde_json::to_string_pretty(val).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e)));
}

/// 通知 Gateway 发生了更新（实现 CLI 和 Web 同步，跨平台支持）
fn notify_gateway(payload: &serde_json::Value) {
    // Skip proxy for localhost connections (avoids http_proxy interference)
    std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_millis(500)))
        .build();
    let agent: ureq::Agent = config.into();
    // 使用同步方式发送，确保 CLI 进程退出前请求已发出
    // 如果 Gateway 没开，500ms 后会自动跳过，不会长时间阻塞
    let _ = agent.post("http://127.0.0.1:8088/api/notify")
        .send_json(payload.clone());
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
  eprintln!("  hippocampus gateway [--port 8088]");
  eprintln!("  hippocampus hook <event>");
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
      "gateway" => cmd_gateway(args),
      "hook" => cmd_hook(args),
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
    let res_json = serde_json::json!({
        "status": "ok",
        "engram_id": id,
        "content": content,
        "importance": importance,
        "layer": layer,
        "permanent": permanent,
    });
    print_json(&res_json);
    
    // 通知 Gateway
    notify_gateway(&serde_json::json!({
        "type": "hook_event", // 借用 hook_event 类型触发刷新
        "hook_type": "remember",
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
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

    // --- 模拟大脑检索状态并通知 Gateway ---
    // 1. 计算召回结果的平均情绪强度
    let avg_emotion: f64 = if results.is_empty() {
        0.1
    } else {
        results.iter()
            .filter_map(|r| r.get("emotion_score").and_then(|v| v.as_f64()))
            .sum::<f64>() / (results.len() as f64)
    };

    // 2. 发送 recall 专用同步信号
    notify_gateway(&serde_json::json!({
        "type": "gate", // 借用 gate 类型来触发 3D 动效
        "components": {
            "amygdala": avg_emotion.max(0.2), // 根据结果情绪决定
            "hippocampus": 0.95,             // 检索核心，极高活跃
            "prefrontal": 0.75,              // 策略控制，高活跃
            "temporal": 0.5,                 // 语义处理，中活跃
        },
        "decision_score": 0.0,
        "should_remember": false,
        "reason": format!("正在检索: {}", query)
    }));

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
    let res_json = serde_json::json!({
        "status": "ok",
        "days": days,
        "semantic_network_learned": result.semantic_network_learned,
        "pruned": result.pruned,
        "reconsolidated": result.reconsolidated,
        "vacuum": result.vacuum,
    });
    print_json(&res_json);
    
    // 通知 Gateway
    notify_gateway(&serde_json::json!({
        "type": "hook_event",
        "hook_type": "reflect",
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
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

    let output = serde_json::json!({
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
    });

    // 保存 last_gate.json 供 gateway 读取
    if do_write || force {
        let gate_path = std::path::Path::new(&home).join("last_gate.json");
        let _ = std::fs::write(&gate_path, serde_json::to_string_pretty(&output).unwrap_or_default());
    }

    print_json(&output);
    // 无论是否写入，都通知 Gateway 进行 3D 脑区动效同步
    let mut notify_payload = output.clone();
    // 如果执行了写入，则标记为 gate_execute 触发 Web 端全量刷新，否则仅标记为 gate 做脑波动效
    notify_payload["type"] = if do_write || force { 
        serde_json::json!("gate_execute") 
    } else { 
        serde_json::json!("gate") 
    };
    
    // 扁平化 components 结构以匹配 Web 端处理逻辑 (data.components[k] 应为 score 数值)
    notify_payload["components"] = serde_json::json!({
        "amygdala": decision.components.amygdala.score,
        "hippocampus": decision.components.hippocampus.score,
        "prefrontal": decision.components.prefrontal.score,
        "temporal": decision.components.temporal.score,
    });
    notify_gateway(&notify_payload);
    
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
    use std::path::{Path, PathBuf};
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
        let cwd = std::env::current_dir().unwrap_or_default();
        let openclaw_json_path = format!("{}/.openclaw/openclaw.json", home_dir);

        // 1. Read workspace path from openclaw.json
        let default_workspace = format!("{}/.openclaw/workspace", home_dir);
        let workspace_dir = std::fs::read_to_string(&openclaw_json_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|c| c.get("agents")
                .and_then(|a| a.get("defaults"))
                .and_then(|d| d.get("workspace"))
                .and_then(|w| w.as_str().map(|s| s.to_string())))
            .unwrap_or(default_workspace);

        // 2. Write HOOK.md and handler.ts (embedded in binary) to <workspace>/hooks/hippocampus/
        let target_dir = format!("{}/hooks/hippocampus", workspace_dir);
        let _ = std::fs::create_dir_all(&target_dir);

        let embedded_files: &[(&str, &str)] = &[
            ("HOOK.md", OPENCLAW_HOOK_MD),
            ("handler.ts", OPENCLAW_HANDLER_TS),
        ];
        for (file, content) in embedded_files {
            let dst = Path::new(&target_dir).join(file);
            match std::fs::write(&dst, content) {
                Ok(_) => installed.push(format!("OpenClaw hook: {}", file)),
                Err(e) => failed.push(format!("OpenClaw hook: {} 写入失败: {}", file, e)),
            }
        }

        // 3. Set HIPPOCAMPUS_HOME in openclaw.json env.vars
        let hippo_home = format!("{}/.hippocampus", home_dir);
        if let Ok(config_str) = std::fs::read_to_string(&openclaw_json_path) {
            if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&config_str) {
                let already_set = config.get("env")
                    .and_then(|e| e.get("vars"))
                    .and_then(|v| v.get("HIPPOCAMPUS_HOME"))
                    .is_some();
                if already_set {
                    skipped.push("OpenClaw HIPPOCAMPUS_HOME env (已配置)".to_string());
                } else {
                    // Ensure env.vars object exists
                    if config.get("env").is_none() {
                        config["env"] = serde_json::json!({});
                    }
                    if config.get("env").and_then(|e| e.get("vars")).is_none() {
                        config["env"]["vars"] = serde_json::json!({});
                    }
                    if let Some(vars) = config.get_mut("env")
                        .and_then(|e| e.get_mut("vars"))
                        .and_then(|v| v.as_object_mut())
                    {
                        vars.insert("HIPPOCAMPUS_HOME".to_string(), serde_json::json!(hippo_home));
                        if let Ok(pretty) = serde_json::to_string_pretty(&config) {
                            if std::fs::write(&openclaw_json_path, pretty).is_ok() {
                                installed.push("OpenClaw HIPPOCAMPUS_HOME env".to_string());
                            } else {
                                failed.push("OpenClaw HIPPOCAMPUS_HOME: 写入失败".to_string());
                            }
                        }
                    }
                }
            }
        }

        // 4. Try to enable hook via openclaw CLI
        match std::process::Command::new("openclaw")
            .args(&["hooks", "enable", "hippocampus"])
            .output()
        {
            Ok(output) if output.status.success() => {
                installed.push("OpenClaw hook enabled".to_string());
            }
            _ => {
                skipped.push("OpenClaw hook enable (请手动执行 openclaw hooks enable hippocampus)".to_string());
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

fn cmd_gateway(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let sub_cmd = args.get(0).map(|s| s.as_str()).unwrap_or("");
    let home = get_home();
    let pid_path = std::path::Path::new(&home).join("gateway.pid");
    let log_path = std::path::Path::new(&home).join("gateway.log");

    match sub_cmd {
        "start" => {
            if pid_path.exists() {
                let old_pid = std::fs::read_to_string(&pid_path).unwrap_or_default();
                eprintln!("⚠️ Gateway 似乎已在运行 (PID: {})。请先执行 stop。", old_pid);
                return Ok(());
            }
            let port = arg_val(args, "--port").unwrap_or_else(|| "8088".to_string());
            let current_exe = std::env::current_exe()?;
            
            println!("🚀 正在后台启动 Gateway (Port: {})...", port);
            let child = std::process::Command::new(current_exe)
                .arg("gateway")
                .arg("--port")
                .arg(&port)
                .stdout(std::fs::File::create(&log_path)?)
                .stderr(std::fs::File::create(&log_path)?)
                .spawn()?;
            
            let pid = child.id();
            std::fs::write(&pid_path, pid.to_string())?;
            println!("✅ Gateway 已启动。PID: {}, Log: {}", pid, log_path.display());
            return Ok(());
        }
        "stop" => {
            if !pid_path.exists() {
                eprintln!("❌ 未发现运行中的 Gateway (未找到 gateway.pid)");
                return Ok(());
            }
            let pid_str = std::fs::read_to_string(&pid_path)?;
            let pid: u32 = pid_str.trim().parse()?;
            
            println!("🛑 正在停止 Gateway (PID: {})...", pid);
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
            }
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("taskkill").arg("/F").arg("/PID").arg(pid.to_string()).status();
            }
            
            let _ = std::fs::remove_file(&pid_path);
            println!("✅ Gateway 已停止。");
            return Ok(());
        }
        _ => {}
    }

    if has_flag(args, "--help") || has_flag(args, "-h") {
        eprintln!("🧠 hippocampus gateway — Web Console");
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  hippocampus gateway [start|stop]");
        eprintln!("  hippocampus gateway [--port PORT]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --port PORT   Listen port (default: 8088)");
        eprintln!();
        eprintln!("API Endpoints:");
        eprintln!("  GET  /api/stats          Memory statistics");
        eprintln!("  GET  /api/engrams         List engrams");
        eprintln!("  POST /api/recall           Recall by query");
        eprintln!("  POST /api/gate            Evaluate gate (dry-run)");
        eprintln!("  POST /api/gate/execute    Evaluate and write");
        eprintln!("  GET  /api/events          WebSocket events");
        return Ok(());
    }
    let port: u16 = arg_val(args, "--port").and_then(|v| v.parse().ok()).unwrap_or(8088);
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
    rt.block_on(hippocampus::gateway::run_gateway(port))
        .map_err(|e| -> Box<dyn std::error::Error> { e })?;
    Ok(())
}

fn cmd_hook(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = String::new();
    let _ = std::io::stdin().read_to_string(&mut buffer);
    let input: serde_json::Value = serde_json::from_str(&buffer).unwrap_or_default();
    
    // 1. 确定通用 Hook 类型 (从命令行第一个参数获取)
    let hook_type = args.get(0).map(|s| s.as_str()).unwrap_or("unknown");
    
    // 2. 识别适配器类型
    let is_claude = args.contains(&"--claude".to_string());
    let is_openclaw = args.contains(&"--openclaw".to_string());

    // 3. 提取通用字段 (根据适配器映射)
    // OpenClaw summarize 需要构建较长生命周期的 summary 字符串
    let mut openclaw_summary = String::new();

    let (event_prompt, event_action, event_summary, session_id) = if is_claude {
        let prompt = input.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
        let last_msg = input.get("lastUserMessage").and_then(|v| v.as_str()).unwrap_or("");
        let sid = input.get("session_id").and_then(|v| v.as_str());

        let action_msg = if let Some(tool) = input.get("toolName").and_then(|v| v.as_str()) {
            let t_in = input.get("toolInput").map(|v| v.to_string()).unwrap_or_default();
            let t_out = input.get("toolOutput").map(|v| v.to_string()).unwrap_or_default();
            Some(format!("工具: {}\n输入: {}\n输出: {}", tool, t_in, t_out))
        } else {
            None
        };

        (Some(prompt), action_msg, Some(last_msg), sid)
    } else if is_openclaw {
        // OpenClaw 适配器：从 stdin JSON 提取 sessionKey 和 context 字段
        let session_key = input.get("sessionKey").and_then(|v| v.as_str());
        let ctx = input.get("context");
        let ctx_content = ctx.and_then(|c| c.get("content")).and_then(|v| v.as_str());
        let ctx_body = ctx.and_then(|c| c.get("bodyForAgent")).and_then(|v| v.as_str());

        // summarize 模式：从 messages 数组拼接摘要
        if hook_type == "summarize" {
            if let Some(messages) = input.get("messages").and_then(|v| v.as_array()) {
                openclaw_summary = messages.iter()
                    .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }

        let summary_ref = if openclaw_summary.is_empty() { None } else { Some(openclaw_summary.as_str()) };

        match hook_type {
            // message:received → auto 模式 (recall + gate)
            "auto" => (ctx_content, None, None, session_key),
            // message:preprocessed / message:sent → record 模式 (gate 评估)
            "record" => (None, ctx_body.or(ctx_content).map(|s| s.to_string()), None, session_key),
            // session:compact:before → summarize 模式
            "summarize" => (None, None, summary_ref, session_key),
            // command:new → reflect 模式
            "reflect" => (None, None, None, session_key),
            _ => (None, None, None, session_key)
        }
    } else {
        // 非 Claude 模式下，尝试直接从 JSON 根部读取标准字段
        (
            input.get("prompt").and_then(|v| v.as_str()),
            input.get("action").and_then(|v| v.as_str()).map(|s| s.to_string()),
            input.get("summary").and_then(|v| v.as_str()),
            input.get("session_id").and_then(|v| v.as_str())
        )
    };

    let home = get_home();
    let mut hippo = hippocampus::Hippocampus::new(&home)?;
    let mut last_decision: Option<hippocampus::MemoryDecision> = None;

    match hook_type {
        "auto" => {
            // 1. 尝试记录行为 (Action/ToolUse)
            if let Some(action) = event_action {
                if let Ok(d) = hippo.auto_remember(&action, "action", session_id, false) {
                    last_decision = Some(d);
                }
            }

            // 2. 尝试召回并记录提示词 (Prompt/Dialogue)
            if let Some(prompt) = event_prompt {
                if !prompt.is_empty() {
                    // 【改进】1. 先进行召回（只寻找过去的记忆）
                    let results = hippo.recall(prompt, 3, 0.05, true, None, None);
                    
                    // 【改进】2. 召回后再记录当前输入（确保不被本次召回看到）
                    if let Ok(d) = hippo.auto_remember(prompt, "dialogue", session_id, false) {
                        last_decision = Some(d);
                    }
                    
                    if !results.is_empty() {
                        let mut mem_context = String::from("🧠 海马体召回背景记忆：\n");
                        for r in &results {
                            let content = r.get("content").and_then(|v| v.as_str()).unwrap_or("");
                            let layer = r.get("layer").and_then(|v| v.as_str()).unwrap_or("?");
                            mem_context.push_str(&format!("- [{}] {}\n", layer, content));
                        }
                        
                        if is_claude {
                            // 方案 1+3: 提取第一条记忆的预览并进行拟人化描述
                            let preview: String = results[0].get("content")
                                .and_then(|v| v.as_str())
                                .map(|s| s.chars().take(20).collect())
                                .unwrap_or_else(|| "...".to_string());
                            let ui_msg = format!("🧠 Hippocampus: 我想起了关于 \"{}...\" 的相关记忆 (共 {} 条)", 
                                preview, results.len());

                            let mut response = serde_json::json!({
                                "continue": true,
                                "additionalContext": mem_context,
                                "systemMessage": ui_msg
                            });

                            if hook_type == "auto" || hook_type == "recall" {
                                response["hookSpecificOutput"] = serde_json::json!({
                                    "hookEventName": "UserPromptSubmit",
                                    "additionalContext": mem_context
                                });
                            }
                            print_json(&response);
                        } else {
                            print_json(&serde_json::json!({ "context": mem_context }));
                        }
                    } else if is_claude {
                        // 没有结果时保持静默，但返回合法的 JSON 以符合规范
                        print_json(&serde_json::json!({ "continue": true }));
                    }
                }
            }
        },
        "recall" => {
            if let Some(prompt) = event_prompt {
                if !prompt.is_empty() {
                    let results = hippo.recall(prompt, 3, 0.05, true, None, None);
                    if let Ok(d) = hippo.auto_remember(prompt, "dialogue", session_id, false) {
                        last_decision = Some(d);
                    }
                    
                    if !results.is_empty() {
                        let mut mem_context = String::from("🧠 海马体召回背景记忆：\n");
                        for r in &results {
                            let content = r.get("content").and_then(|v| v.as_str()).unwrap_or("");
                            let layer = r.get("layer").and_then(|v| v.as_str()).unwrap_or("?");
                            mem_context.push_str(&format!("- [{}] {}\n", layer, content));
                        }
                        
                        if is_claude {
                            let preview: String = results[0].get("content")
                                .and_then(|v| v.as_str())
                                .map(|s| s.chars().take(20).collect())
                                .unwrap_or_else(|| "...".to_string());
                            let ui_msg = format!("🧠 Hippocampus: 我想起了关于 \"{}...\" 的相关记忆 (共 {} 条)", 
                                preview, results.len());

                            print_json(&serde_json::json!({
                                "continue": true,
                                "additionalContext": mem_context,
                                "systemMessage": ui_msg,
                                "hookSpecificOutput": {
                                    "hookEventName": "UserPromptSubmit",
                                    "additionalContext": mem_context
                                }
                            }));
                        } else {
                            print_json(&serde_json::json!({ "context": mem_context }));
                        }
                    } else if is_claude {
                        print_json(&serde_json::json!({ "continue": true }));
                    }
                }
            }
        },
        "record" => {
            if let Some(action) = event_action {
                if let Ok(d) = hippo.auto_remember(&action, "action", session_id, false) {
                    last_decision = Some(d);
                }
            }
        },
        "summarize" => {
            if let Some(summary) = event_summary {
                if !summary.is_empty() {
                    if let Ok(d) = hippo.auto_remember(summary, "dialogue", session_id, false) {
                        if is_claude {
                            print_json(&serde_json::json!({
                                "continue": true,
                                "systemMessage": "🧠 Hippocampus: 今日对话已摘要，这段经历已被存入我的长期记忆库。",
                                "hookSpecificOutput": {
                                    "hookEventName": "Stop"
                                }
                            }));
                        } else if is_openclaw {
                            print_json(&serde_json::json!({
                                "context": format!("🧠 Hippocampus: 对话已摘要并归档 (重要性: {:.2})", d.decision_score)
                            }));
                        }
                        last_decision = Some(d);
                    }
                }
            }
        },
        "reflect" => {
            // 执行记忆整理 (Reflect)
            if let Ok(res) = hippo.reflect(3) {
                if is_claude {
                    let total = res.reconsolidated + res.vacuum.l1_to_l2 + res.vacuum.l2_to_l3;
                    let mut response = serde_json::json!({ "continue": true });
                    if total > 0 {
                        let ui_msg = format!("🧠 Hippocampus: 睡眠反思周期结束。已有 {} 条重要印迹被加深并归档。", total);
                        response["systemMessage"] = serde_json::json!(ui_msg);
                        response["hookSpecificOutput"] = serde_json::json!({
                            "hookEventName": "SessionStart",
                            "additionalContext": ui_msg
                        });
                    }
                    print_json(&response);
                } else {
                    let msg = format!("🧠 记忆反思完成。巩固: {}, Vacuum: L1->L2: {}, L2->L3: {}", 
                        res.reconsolidated, res.vacuum.l1_to_l2, res.vacuum.l2_to_l3);
                    print_json(&serde_json::json!({ "context": msg }));
                }
            }
        },
        _ => {}
    }

    // 发送同步通知给 Gateway
    let mut notify_payload = serde_json::json!({
        "type": "gate_execute", // 使用 gate_execute 以触发 3D 动效 + 数据刷新
        "hook_type": hook_type,
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    });

    // 如果有决策数据，带上脑区得分实现真实表现
    if let Some(d) = last_decision {
        notify_payload["components"] = serde_json::json!({
            "amygdala": d.components.amygdala.score,
            "hippocampus": d.components.hippocampus.score,
            "prefrontal": d.components.prefrontal.score,
            "temporal": d.components.temporal.score,
        });
        notify_payload["decision_score"] = serde_json::json!(d.decision_score);
        notify_payload["should_remember"] = serde_json::json!(d.should_remember);
    } else {
        // 如果没有决策（纯 recall 或失败），模拟一次通用的神经活动
        notify_payload["components"] = serde_json::json!({
            "amygdala": 0.2,
            "hippocampus": 0.8,
            "prefrontal": 0.6,
            "temporal": 0.4,
        });
    }

    notify_gateway(&notify_payload);

    Ok(())
}
