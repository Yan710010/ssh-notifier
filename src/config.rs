use std::{
    fs::{File, read_to_string},
    io::Write,
};

use anyhow::{Ok, anyhow};
use indoc::indoc;
use kdl::KdlDocument;
use log::info;

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
        info!("Loading configuration...");
        // 判断配置文件是否存在
        let config_path = dirs::config_dir()
            .ok_or(anyhow!("无法识别配置文件夹"))?
            .join("ssh-notifier.kdl");
        if !config_path.try_exists()? {
            // 配置不存在, 创建默认配置文件并返回
            info!("Configuration not found, creating one");
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
        info!("Loaded configuration");
        return Ok(Config { users: users });
    }
}

#[allow(unused)]
mod test {
    use super::*;

    #[test]
    fn ssh_verify() {
        let mut config = Config::default();
        config.users.push(User {
            name: "foo".into(),
            key_fingerprints: vec!["key0".into(), "key2".into()],
            greeting: "Content of a greeting test".into(),
            ..Default::default()
        });
        config.users.push(User {
            name: "bar".into(),
            key_fingerprints: vec!["key7".into(), "pmdish".into()],
            greeting: "Margarita pizza is delicious".into(),
            ..Default::default()
        });
        config.users.push(User {
            name: "buz".into(),
            key_fingerprints: vec!["pasta".into(), "onion".into()],
            greeting: "Creamy mushroom soup...".into(),
            ..Default::default()
        });
        let k0 = config.verify_ssh("key0");
        let k1 = config.verify_ssh("key1");
        let k2 = config.verify_ssh("key2");
        let k3 = config.verify_ssh("pmdish");
        let k4 = config.verify_ssh("pasta");

        assert!(k0.is_some());
        assert!(k1.is_none());
        assert!(k2.is_some());
        assert!(k3.is_some());
        assert!(k4.is_some());

        let k0 = k0.unwrap();
        let k2 = k2.unwrap();
        let k3 = k3.unwrap();
        let k4 = k4.unwrap();
        assert!(k0.name == "foo");
        assert!(k0.greeting == "Content of a greeting test");
        assert!(k2.name == "foo");
        assert!(k2.greeting == "Content of a greeting test");
        assert!(k3.name == "bar");
        assert!(k3.greeting == "Margarita pizza is delicious");
        assert!(k4.name == "buz");
        assert!(k4.greeting == "Creamy mushroom soup...");
    }
}
