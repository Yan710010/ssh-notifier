# ssh-notifier
为连接到主机的ssh会话发送欢迎语，并给主机发送提醒信息

## 构建
`cargo build --release`  
程序运行需要外部程序 journalctl loginctl notify-send

## 配置
使用`$XDG_CONFIG_HOME/ssh-notifier.kdl`作为配置文件  
配置格式如下  
```kdl
// 设置提醒消息的标题
notify-title "有人连上来了喵?"
// 当不在users列表的用户连接的时显示消息的标题
notify-title-for-stranger "有陌生人连上来了喵!"
// 消息内容(可以替换的字段如下所示)
notify-message """
    密钥属于: {name}
    时间: {time}
    登录用户: {user}
    IP: {ip}
    """
notify-message-for-stranger """
    时间: {time}
    登录用户: {user}
    IP: {ip}
    密钥指纹: {fpr}
    """
// 用于显示时间的格式
// 使用 rfc3339 或 rfc2822 指定, 或用类似date的替换符
// rfc3339: 2026-01-21T14:15:41+08:00
// rfc2822: Wed, 21 Jan 2026 14:15:41 +0800
time-format "%Y-%m-%d %H:%M:%S" // 2026-1-21 14:15:41

users {
    // 节点以用户名称为名
    foo {
        // 能够对应到用户的ssh密钥指纹
        fingerprint "SHA256:xxx" "SHA256:aaa"
        // 如果不希望主机在这个用户登录时获得提示，可以加上
        /- no-notify
        // 在连接者会话中显示的消息
        greeting "bar"
    }
}
```
