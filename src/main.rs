use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        // 例: Accepted publickey for yan from ::1 port 39332 ssh2: ED25519 SHA256:9F4zuuHf3+TUADxbug4BcLQJ/wRLoWFZwU8wYMotqMk
        // # 考虑到我这没公网, 都是用的frp, 就不判断IP了
        // byd 直接取最后一段吧
        let fields: Vec<&str> = line.split_whitespace().collect();
        let fingerprint = *fields.last().unwrap_or(&"");
        // TODO: 读取配置文件识别密钥指纹

        // 发送有人登录的提示
        let notify = Command::new("notify-send")
            .arg("--urgency=critical")
            .arg("--app-name=ssh-notify")
            .arg("SSH连接") // Title
            .arg("有人连上来了喵!") // Message
            .spawn();
        // 检测是否发送成功
    }
    Ok(())
}
