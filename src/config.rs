use std::{
    fs::{File, read_to_string},
    io::Write,
};

use anyhow::{Ok, anyhow};
use indoc::indoc;
use kdl::KdlDocument;

// 数据结构定义
#[derive(Default, Debug)]
pub struct Config {
    users: Vec<User>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub name: String,
    pub key_fingerprints: Vec<String>,
    pub no_notify: bool,
    pub greeting: String,
}

impl User {
    pub fn verify(&self, fingerprint: &str) -> bool {
        self.key_fingerprints.contains(&fingerprint.into())
    }
}
// 默认值
impl Default for User {
    fn default() -> Self {
        Self {
            name: "UNKNOWN".into(),
            key_fingerprints: vec![],
            no_notify: false,
            greeting: "...你是谁".into(),
        }
    }
}

impl Config {
    pub fn verify_ssh(&self, fin: &str) -> Option<User> {
        if fin.is_empty() {
            return None;
        }
        for user in &self.users {
            if user.verify(fin) {
                return Some(user.clone());
            }
        }
        None
    }

    /// 从文件加载用户配置
    pub fn load() -> anyhow::Result<Config> {
        // 判断配置文件是否存在
        let config_path = dirs::config_dir()
            .ok_or(anyhow!("无法识别配置文件夹"))?
            .join("ssh-notifier.kdl");
        if !config_path.try_exists()? {
            // 配置不存在, 创建默认配置文件并返回
            eprintln!("创建默认配置文件");
            let mut file = File::create(config_path)?;
            file.write_all(indoc! {br#"
                /- username {
                    fingerprint "SHA256:xxx..." "SHA256:xxx..."
                    /- no-notify
                }
                "#})?;
            return Ok(Config::default());
        }
        // 读取配置文件并解析
        let config = read_to_string(config_path)?;
        let doc: KdlDocument = config.parse()?;

        let mut users: Vec<User> = vec![];
        for node in doc.nodes() {
            let name = node.name().value();
            let Some(children) = node.children() else {
                // 不儿这都没有那你定义了个什么
                continue;
            };
            let fpr: Vec<String> = children
                .nodes()
                .iter()
                .filter(|n| n.name().value() == "fingerprint")
                .map(|n| n.entries())
                .map(|entries| {
                    entries
                        .iter()
                        .map(|e| e.value().as_string().unwrap_or_default().into())
                })
                .flatten()
                .collect();
            let no_notify = children.get("no-notify").is_some();
            let greeting = children
                .get("greeting")
                .and_then(|n| n.get(0))
                .and_then(|arg| arg.as_string())
                .map(|s| s.into())
                .unwrap_or_default();
            users.push(User {
                name: name.into(),
                key_fingerprints: fpr,
                no_notify,
                greeting,
            });
        }
        return Ok(Config { users: users });
    }
}
