mod config;

use anyhow::Result;
use anyhow::anyhow;
use std::path::Path;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use crate::config::{Config, User};

fn main() -> Result<()> {
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
    // 主循环
    loop {
        line.clear();
        // 获取
        let length = reader.read_line(&mut line).unwrap();
        // EOF判断
        if length == 0 {
            break;
        }

        // 获取连接信息
        let info = SSHInfo::parse(&line);
        // 读取配置文件识别密钥指纹
        let logined_user = match Config::load() {
            Ok(conf) => conf.verify_ssh(&info.fingerprint).unwrap_or_default(),
            Err(err) => {
                eprintln!("\x1b[31m{}\x1b[0m",err);
                config::User::default()
            }
        };

        // 发送有人登录的提示
        notify_send(&logined_user, &info);
        // 显示问候语
        greet(&logined_user.greeting, &info)
            .map_err(|e| eprintln!("{}", e))
            .ok();
    }
    eprintln!("子进程journalctl关闭, 正在退出");
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
    pub fn parse(msg: &String) -> SSHInfo {
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
fn notify_send(user: &User, info: &SSHInfo) {
    if user.no_notify {
        //return;
    }
    // 格式化消息文本
    let mut title = "有人连上来了喵?";
    let mut message = format!(
        "公钥属于 {}\n来自 {}\n登录用户: {}",
        user.name, info.ip, info.user,
    );
    if user.name == "UNKNOWN" {
        title = "有陌生人连上来了喵！";
        message.push_str(&format!("\n密钥指纹: {}", info.fingerprint));
    }
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
                "\x1b[u\x1b[4A" // 恢复保存的位置 光标上移(因为之前向下滚动了)
            );
            // 注入信息(狠狠灌注!)
            std::fs::write(Path::new("/dev").join(tty), greeting_formatted)?;
            return Ok(());
        }
    }
    Err(anyhow!("未找到符合的login"))
}
