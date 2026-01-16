# ssh-notifier
为连接到主机的ssh会话发送欢迎语，并给主机发送提醒信息

## 构建
`cargo build --release`  
程序运行需要外部程序 journalctl loginctl notify-send

## 配置
使用`$XDG_CONFIG_HOME/ssh-notifier.kdl`作为配置文件  
格式见[样例配置](example.kdl)
