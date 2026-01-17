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
