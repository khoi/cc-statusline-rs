use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::Command;

pub fn statusline(_show_pr_status: bool) -> String {
    let input = read_input().unwrap_or_default();

    let current_dir = input
        .get("workspace")
        .and_then(|w| w.get("current_dir"))
        .and_then(|d| d.as_str());

    let model = input
        .get("model")
        .and_then(|m| m.get("display_name"))
        .and_then(|d| d.as_str());

    let output_style = input
        .get("output_style")
        .and_then(|o| o.get("name"))
        .and_then(|n| n.as_str());

    let model_display = if let Some(model) = model {
        let style_suffix = match output_style {
            Some(style) => format!(" \x1b[90m({})\x1b[0m", style),
            None => String::new(),
        };
        format!(
            "\x1b[38;5;14m\u{e26d} \x1b[38;5;208m{}{}",
            model, style_suffix
        )
    } else {
        String::new()
    };

    let context_display = if let Some(ctx) = input.get("context_window") {
        let window_size = ctx
            .get("context_window_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(200000);

        let used = if let Some(current) = ctx.get("current_usage") {
            let input = current
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let cache_creation = current
                .get("cache_creation_input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let cache_read = current
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            input + cache_creation + cache_read
        } else {
            0
        };
        let pct = if window_size > 0 {
            ((used as f64 * 100.0) / window_size as f64).min(100.0)
        } else {
            0.0
        };

        let pct_color = if pct >= 90.0 {
            "\x1b[31m"
        } else if pct >= 70.0 {
            "\x1b[38;5;208m"
        } else if pct >= 50.0 {
            "\x1b[33m"
        } else {
            "\x1b[90m"
        };

        let bar_width: usize = 10;
        let filled = (pct * bar_width as f64 / 100.0).round() as usize;
        let empty = bar_width.saturating_sub(filled);
        let bar: String = "█".repeat(filled) + &"░".repeat(empty);

        format!(
            "\x1b[38;5;13m\u{f49b} \x1b[90m{}\x1b[0m {}{}%\x1b[0m",
            bar,
            pct_color,
            pct.round() as u32
        )
    } else {
        String::new()
    };

    let current_dir = match current_dir {
        Some(dir) => dir,
        None => return format!("\x1b[31m\u{f071} missing workspace.current_dir\x1b[0m"),
    };

    let branch = if is_git_repo(current_dir) {
        truncate_middle(&get_git_branch(current_dir), 32)
    } else {
        String::new()
    };

    let display_dir = format!("{} ", fish_shorten_path(current_dir));

    let lines_changed = if let Some(cost_obj) = input.get("cost") {
        let lines_added = cost_obj
            .get("total_lines_added")
            .and_then(|l| l.as_u64())
            .unwrap_or(0);
        let lines_removed = cost_obj
            .get("total_lines_removed")
            .and_then(|l| l.as_u64())
            .unwrap_or(0);

        if lines_added > 0 || lines_removed > 0 {
            format!(
                "(\x1b[32m+{}\x1b[0m \x1b[31m-{}\x1b[0m)",
                lines_added, lines_removed
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let cost_display = if let Some(cost_obj) = input.get("cost") {
        if let Some(total_cost) = cost_obj.get("total_cost_usd").and_then(|c| c.as_f64()) {
            let formatted_cost = format_cost(total_cost);
            let cost_color = if total_cost < 5.0 {
                "\x1b[32m"
            } else if total_cost < 20.0 {
                "\x1b[33m"
            } else {
                "\x1b[31m"
            };
            format!(
                "\x1b[38;5;3m\u{f155} {}{}\x1b[0m",
                cost_color, formatted_cost
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut components = Vec::new();
    if !model_display.is_empty() {
        components.push(model_display.clone());
    }
    if !context_display.is_empty() {
        components.push(context_display.clone());
    }
    if !cost_display.is_empty() {
        components.push(cost_display.clone());
    }

    let components_str = if components.is_empty() {
        String::new()
    } else {
        format!(
            " \x1b[90m• \x1b[0m{}",
            components.join(" \x1b[90m• \x1b[0m")
        )
    };

    if !branch.is_empty() {
        if display_dir.is_empty() {
            format!(
                "\x1b[38;5;12m\u{f02a2} \x1b[32m{}{}\x1b[0m{}",
                branch, lines_changed, components_str
            )
        } else {
            format!(
                "\x1b[36m{}\x1b[0m \x1b[38;5;12m\u{f02a2} \x1b[32m{}{}\x1b[0m{}",
                display_dir.trim_end(),
                branch,
                lines_changed,
                components_str
            )
        }
    } else {
        format!(
            "\x1b[36m{}\x1b[0m{}",
            display_dir.trim_end(),
            components_str
        )
    }
}

pub fn read_input() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(serde_json::from_str(&buffer)?)
}

pub fn get_git_branch(working_dir: &str) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(working_dir)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => String::new(),
    }
}

pub fn is_git_repo(dir: &str) -> bool {
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output();

    matches!(output, Ok(output) if output.status.success() &&
             String::from_utf8_lossy(&output.stdout).trim() == "true")
}

pub fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
}

pub fn get_session_duration(transcript_path: Option<&str>) -> Option<String> {
    let transcript_path = transcript_path?;
    if !Path::new(transcript_path).exists() {
        return None;
    }

    let data = fs::read_to_string(transcript_path).ok()?;
    let lines: Vec<&str> = data.lines().filter(|l| !l.trim().is_empty()).collect();

    if lines.len() < 2 {
        return None;
    }

    let mut first_ts = None;
    let mut last_ts = None;

    for line in &lines {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(timestamp) = json.get("timestamp") {
                first_ts = Some(parse_timestamp(timestamp)?);
                break;
            }
        }
    }

    for line in lines.iter().rev() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(timestamp) = json.get("timestamp") {
                last_ts = Some(parse_timestamp(timestamp)?);
                break;
            }
        }
    }

    if let (Some(first), Some(last)) = (first_ts, last_ts) {
        let duration_ms = last - first;
        let hours = duration_ms / (1000 * 60 * 60);
        let minutes = (duration_ms % (1000 * 60 * 60)) / (1000 * 60);

        if hours > 0 {
            Some(format!("{}h{}m", hours, minutes))
        } else if minutes > 0 {
            Some(format!("{}m", minutes))
        } else {
            Some("<1m".to_string())
        }
    } else {
        None
    }
}

pub fn parse_timestamp(timestamp: &serde_json::Value) -> Option<i64> {
    if let Some(ts_str) = timestamp.as_str() {
        chrono::DateTime::parse_from_rfc3339(ts_str)
            .map(|dt| dt.timestamp_millis())
            .ok()
    } else {
        timestamp.as_i64()
    }
}

pub fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        format!("{:.3}", cost)
    } else {
        format!("{:.2}", cost)
    }
}

pub fn format_tokens(tokens: u64) -> String {
    let k = tokens as f64 / 1000.0;
    if k >= 100.0 {
        format!("{}k", k.round() as u64)
    } else if k >= 10.0 {
        format!("{:.0}k", k)
    } else {
        format!("{:.1}k", k)
    }
}

pub fn truncate_middle(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    let keep = max - 1;
    let head = keep - keep / 2;
    let tail = keep / 2;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[chars.len() - tail..]);
    out
}

pub fn fish_shorten_path(path: &str) -> String {
    let home = home_dir();
    let path = path.replace(&home, "~");

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 1 {
        return path;
    }

    let shortened: Vec<String> = parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            if i == parts.len() - 1 || part.is_empty() || *part == "~" {
                part.to_string()
            } else if part.starts_with('.') && part.len() > 1 {
                format!(".{}", part.chars().nth(1).unwrap_or_default())
            } else {
                part.chars()
                    .next()
                    .map(|c| c.to_string())
                    .unwrap_or_default()
            }
        })
        .collect();

    shortened.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_middle_short_unchanged() {
        assert_eq!(truncate_middle("main", 32), "main");
    }

    #[test]
    fn truncate_middle_exact_max_unchanged() {
        let s = "a".repeat(32);
        assert_eq!(truncate_middle(&s, 32), s);
    }

    #[test]
    fn truncate_middle_caps_long_branch() {
        let result = truncate_middle("khoi/goo-3197-cursor-smoothing-diagnostics", 32);
        assert_eq!(result, "khoi/goo-3197-cu…ing-diagnostics");
        assert_eq!(result.chars().count(), 32);
    }
}
