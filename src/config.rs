use std::{
    fs::{File, read_to_string},
    io::Write,
};

use anyhow::{Ok, anyhow};
use indoc::indoc;
use kdl::KdlDocument;
use log::info;

// 数据结构定义
#[derive(Debug)]
pub struct Config {
    pub notify_title: String,
    pub notify_title_for_stranger: String,
    pub notify_message: String,
    pub notify_message_for_stranger: String,
    pub time_format: String,
    pub users: Vec<User>,
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
impl Default for Config {
    fn default() -> Self {
        Self {
            notify_title: "New ssh Connection".into(),
            notify_title_for_stranger: "UNKNOWN SSH CONNECTION".into(),
            users: vec![],
            notify_message: "User {name} logined {user} at {time}".into(),
            notify_message_for_stranger: "Someone logined {user} at {time} with fingerprint {fpr}"
                .into(),
            time_format: "rfc3339".into(),
        }
    }
}
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
            file.write_all(indoc! {r#"
                /- notify-title "new ssh connection"
                /- notify-title-for-stranger "UNKNOWN SSH CONNECTION"
                /- notify-message "User {name} logined {user} at {time}"
                /- notify-message-for-stranger "Someone logined {user} at {time} with fingerprint {fpr}"
                /- time-format "rfc3339"
                users {
                    /- username {
                        fingerprint "SHA256:xxx..." "SHA256:xxx..."
                        /- no-notify
                        /- greeting "Hello!"
                    }
                }
                "#}.as_bytes())?;
            return Ok(Config::default());
        }

        // 读取配置文件并解析
        let config = read_to_string(config_path)?;
        let doc: KdlDocument = config.parse()?;

        let mut conf = Config::default();
        for node in doc.nodes() {
            let nodename = node.name().value();
            let as_first_arg = |field: &mut String| {
                if let Some(value) = node.get(0).and_then(|v| v.as_string()) {
                    *field = value.to_string();
                }
            };
            match nodename {
                "notify-title" => as_first_arg(&mut conf.notify_title),
                "notify-title-for-stranger" => as_first_arg(&mut conf.notify_title_for_stranger),
                "notify-message" => as_first_arg(&mut conf.notify_message),
                "notify-message-for-stranger" => {
                    as_first_arg(&mut conf.notify_message_for_stranger)
                }
                "time-format" => as_first_arg(&mut conf.time_format),
                "users" => {
                    for user in node.children().iter().map(|c| c.nodes()).flatten() {
                        let name = user.name().value();
                        let Some(children) = user.children() else {
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
                        conf.users.push(User {
                            name: name.into(),
                            key_fingerprints: fpr,
                            no_notify,
                            greeting,
                        });
                    }
                }
                &_ => {}
            }
        }
        info!("Loaded configuration");
        return Ok(conf);
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
