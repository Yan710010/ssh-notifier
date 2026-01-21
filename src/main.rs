mod config;

use anyhow::Result;
use anyhow::anyhow;
use env_logger::Env;
use log::debug;
use log::error;
use log::info;
use log::warn;
use std::path::Path;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use crate::config::{Config, User};

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    debug!("Application started");
    // 使用子进程从journal读取sshd信息
    let mut journal = Command::new("journalctl")
        .args([
            "--unit", "sshd", "--grep", "Accepted", "--output", "short", "--since", "now",
            "--follow",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    // 获取输出
    let output = journal.stdout.take().unwrap();
    let mut reader = BufReader::new(output);

    let mut line = String::new();

    // Probably mut, for introducing config reloading etc.
    let config = match Config::load() {
        Ok(conf) => conf,
        Err(err) => {
            error!("{}", err);
            warn!("Failed to load configuration due to error, fallback to default");
            Config::default()
        }
    };

    // 主循环
    loop {
        line.clear();
        // 获取
        let length = reader.read_line(&mut line).unwrap();
        // EOF判断
        if length == 0 {
            info!("End-of-file ocurred, stopping");
            break;
        }

        debug!("Got line from sshd: {}", line);

        // 获取连接信息
        let info = SSHInfo::parse(&line);
        debug!("Parsed info: {:?}", info);
        info!(
            "Logging from {}@{}, fingerprint {}",
            info.user, info.ip, info.fingerprint
        );

        // 识别密钥指纹
        let logined_user = config.verify_ssh(&info.fingerprint).unwrap_or_default();

        // 发送有人登录的提示
        notify_send(&config, &logined_user, &info);

        // 显示问候语
        if let Err(e) = greet(&logined_user.greeting, &info) {
            warn!("Error ocurred while greeting: {}", e);
        }
    }
    info!("子进程journalctl关闭, 正在退出");
    Ok(())
}

/// 储存sshd message 的信息
#[derive(Debug)]
struct SSHInfo {
    pub pid: String,
    pub user: String,
    pub ip: String,
    pub fingerprint: String,
}

impl SSHInfo {
    pub fn parse(msg: &str) -> SSHInfo {
        // 例: Accepted publickey for yan from ::1 port 39332 ssh2: ED25519 SHA256:9F4zuuHf3+TUADxbug4BcLQJ/wRLoWFZwU8wYMotqMk
        // 拆!
        let extract = |m: &str, s: &str, e: &str| {
            if let Some(i) = m.find(s) {
                let rest = &m[i + s.len()..];
                if let Some(j) = rest.find(e) {
                    return Some(rest[..j].to_string());
                }
            }
            None
        };
        let pid = extract(&msg, "sshd-session[", "]").unwrap_or_default();
        let user = extract(&msg, "for ", " from").unwrap_or("UNKNOWN".into());
        let ip = extract(&msg, "from ", " port").unwrap_or("UNKNOWN".into());
        let fingerprint = msg.split_whitespace().last().unwrap_or("NONE").to_string();

        SSHInfo {
            pid,
            user,
            ip,
            fingerprint,
        }
    }
}

/// 发送登录提示
fn notify_send(config: &Config, user: &User, info: &SSHInfo) {
    if user.no_notify {
        return;
    }

    // 格式化消息文本
    let (title, message) = if user.name != "UNKNOWN" {
        (&config.notify_title, config.notify_message.clone())
    } else {
        (
            &config.notify_title_for_stranger,
            config.notify_message_for_stranger.clone(),
        )
    };

    // 替换占位符
    let message = message
        .replace("{name}", &user.name)
        .replace("{ip}", &info.ip)
        .replace("{user}", &info.user)
        .replace("{fpr}", &info.fingerprint)
        .replace("{time}", &{
            let format = &config.time_format;
            let time = chrono::Local::now();
            if format == "rfc3339" {
                time.to_rfc3339()
            } else if format == "rfc2822" {
                time.to_rfc2822()
            } else {
                time.format(format).to_string()
            }
        });

    let notify = Command::new("notify-send")
        .arg("--urgency=critical")
        .arg("--app-name=ssh-notify")
        .arg(title) // Title
        .arg(message) // Message
        .output();
    if !notify.is_ok_and(|o| o.status.success()) {
        eprintln!("发送提示失败!");
    }
}

fn greet(greeting: &str, info: &SSHInfo) -> Result<()> {
    // 不需要发送提示的情况
    if info.pid.is_empty() {
        return Err(anyhow!("未知的PID, 无法获取用户会话"));
    }
    if greeting.is_empty() {
        return Err(anyhow!("没有需要发送的信息"));
    }
    // 从loginctl里面捞tty
    let output = Command::new("loginctl")
        .arg("list-sessions")
        .arg("--no-legend")
        .output()?;
    for line in String::from_utf8(output.stdout)?.lines() {
        // loginctl 输出格式
        // SESSION UID USER SEAT LEADER CLASS TTY IDLE SINCE
        let properties: Vec<&str> = line.split_whitespace().collect();
        if properties.get(4).map(|s| *s == info.pid) == Some(true) {
            let tty = properties.get(6).ok_or(anyhow!("无法获取tty"))?;
            // 神奇的ANSI转义环节
            let greeting_formatted = format!(
                "{}{}{}{}",
                "\x1b[s\x1b[4S\x1b[3F", // 保存当前位置 向下滚动(腾出显示空间) 光标上移(达到目标显示位置)
                "+-----------\n",
                greeting,
                "\x1b[u" // 恢复保存的位置 (由于已经向下滚动，所以光标的最终位置会在欢迎语的下方)
            );
            // 注入信息(狠狠灌注!)
            std::fs::write(Path::new("/dev").join(tty), greeting_formatted)?;
            return Ok(());
        }
    }
    Err(anyhow!("未找到符合的login"))
}

#[allow(unused)]
mod test {
    use super::*;

    #[test]
    fn parsing_sshd_log() {
        let info = SSHInfo::parse(
            "Jan 11 23:10:45 HyprOnion sshd-session[31509]: Accepted publickey for yan from 127.0.0.1 port 50178 ssh2: ED25519 SHA256:EjRavL2HEklAJavw5mtARs0LdvhQ+V+ShZ7oDLmjwhs",
        );
        assert_eq!(info.user, "yan");
        assert_eq!(info.ip, "127.0.0.1");
        assert_eq!(info.pid, "31509");
        assert_eq!(
            info.fingerprint,
            "SHA256:EjRavL2HEklAJavw5mtARs0LdvhQ+V+ShZ7oDLmjwhs"
        );
    }
}
