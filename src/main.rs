mod config;

use anyhow::Result;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

fn main() -> Result<()> {
    // 使用子进程从journal读取sshd信息
    let mut journal = Command::new("journalctl")
        .args([
            "--unit", "sshd", "--grep", "Accepted", "--output", "cat", "--since", "now", "--follow",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    // 获取输出
    let output = journal.stdout.take().unwrap();
    let mut reader = BufReader::new(output);

    let mut line = String::new();
    loop {
        line.clear();
        // 获取
        let length = reader.read_line(&mut line).unwrap();
        // EOF判断
        if length == 0 {
            break;
        }

        // 获取需要的部分
        let info = SSHInfo::parse(&line);
        // TODO: 读取配置文件识别密钥指纹

        // 发送有人登录的提示
        let notify = Command::new("notify-send")
            .arg("--urgency=critical")
            .arg("--app-name=ssh-notify")
            .arg("有人连上来了喵?") // Title
            .arg(format!(
                "登录用户:{}\nIP:{}\n指纹:{}",
                info.user, info.ip, info.fingerprint
            )) // Message
            .output();
        if notify.is_err() || notify.is_ok_and(|o| !o.status.success()) {
            eprintln!("发送提示失败!");
        }
    }
    eprintln!("子进程journalctl关闭, 正在退出");
    Ok(())
}

/// 储存sshd message 的信息
#[derive(Debug)]
struct SSHInfo {
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
        let user = extract(&msg, "for ", " from").unwrap_or("UNKNOWN".into());
        let ip = extract(&msg, "from ", " port").unwrap_or("UNKNOWN".into());
        let fingerprint = msg.split_whitespace().last().unwrap_or("NONE").to_string();

        SSHInfo {
            user,
            ip,
            fingerprint,
        }
    }
}
