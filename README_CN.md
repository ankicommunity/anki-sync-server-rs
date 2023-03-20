<div align="center">

# anki-sync-server-rs

[![License](https://img.shields.io/github/license/ankicommunity/anki-sync-server-rs)](https://github.com/ankicommunity/anki-sync-server-rs/blob/master/LINCENSE)[![Github status](https://img.shields.io/github/checks-status/ankicommunity/anki-sync-server-rs/master?label=github%20status)](https://github.com/ankicommunity/anki-sync-server-rs/actions)[![Github contributors](https://img.shields.io/github/contributors/ankicommunity/anki-sync-server-rs?label=github%20contributors)](https://github.com/ankicommunity/anki-sync-server-rs/graphs/contributors)[![DockerHub version](https://img.shields.io/docker/v/ankicommunity/anki-sync-server-rs?label=dockerhub%20version&sort=date)](https://hub.docker.com/repository/docker/ankicommunity/anki-sync-server-rs)[![DockerHub pulls](https://img.shields.io/docker/pulls/ankicommunity/anki-sync-server-rs)](https://hub.docker.com/repository/docker/ankicommunity/anki-sync-server-rs)[![DockerHub stars](https://img.shields.io/docker/stars/ankicommunity/anki-sync-server-rs)](https://hub.docker.com/repository/docker/ankicommunity/anki-sync-server-rs)
[![](https://img.shields.io/github/v/release/ankicommunity/anki-sync-server-rs)](https://github.com/ankicommunity/anki-sync-server-rs/releases/latest)[![](https://img.shields.io/github/last-commit/ankicommunity/anki-sync-server-rs)]()[![Gitter](https://badges.gitter.im/ankicommunity/community.svg)](https://gitter.im/ankicommunity/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)
[![Downloads](https://img.shields.io/github/downloads/ankicommunity/anki-sync-server-rs/total?label=Release%20Download)](https://github.com/ankicommunity/anki-sync-server-rs/releases/latest)

[简体中文](README_CN.md)|[English](README.md)
</div>

这是一个Rust语言版本的Anki 自建同步服务端，这个服务器追踪[Anki官方](https://github.com/ankitects/anki)同步服务端的进度，它们都是基于sqlite c 作为数据存储后端。

也有Anki官方推出的镶嵌在Anki客户端的同步服务端和通过Python安装的同步服务端，[看这里](https://docs.ankiweb.net/sync-server.html)

## 服务端的简单的使用说明
### 安装 (通过二进制可执行文件)
1. 下载二进制文件，地址[releases](https://github.com/ankicommunity/anki-sync-server-rs/releases) ，注意下载与您的计算机平台相符的文件，比如说，对于Windows的用户来说，下载文件名带有`windows_x86_64`的文件。下载后解压缩并进入解压后的文件夹。
2. 添加账号（注：下面提到的`username`，`password`为您想设置的用户名和密码）。

对于Linux、macOS的用户，运行命令：
```
 ./ankisyncd user --add username password
```
对于WIndows用户，进入解压后的文件夹，打开一个命令行终端，运行命令：
```
 ./ankisyncd.exe user --add username password
```
如果您还想进行其它账号相关的操作，执行帮助命令：
```
 ./ankisyncd user --help
```
3. 启动即运行服务端，对于Linux、macOS的用户，运行命令：`./ankisyncd`,对于WIndows用户直接鼠标双击可执行文件`ankisyncd.exe`。
4. 到这里服务端的配置基本完成了。
### 安装（通过容器Docker安装）
具体细节查看文件[Docker](docs/CONTAINER.md)

当然您也可以同步从源码构建目标平台的二进制文件[Install](docs/INSTALL.md)或者从源码构建docker镜像来安装服务端[DockerBuild](docs/CONTAINER.md)。
## 设置Anki客户端
### Anki 电脑端
#### >=2.1.57
因为软件更新，Anki客户端将自定义同步服务端作为内建功能。
1. 打开Anki，依次鼠标点击`工具`-->`设置`-->`网络`
2. 往下看，可以看到标有`self-hosted sync server(自建同步服务器)`的方框，在里面填写您的服务端的地址
3. 举个栗子。如果您的服务端地址为`192.0.0.1`,那么空白处应该填写的内容为 `http://192.0.0.1:27701/`。 
4. 重启Anki
#### <2.1.57
1. 打开Anki，依次鼠标点击选中`工具` -> `插件`。
2. 在插件页面，点击`获取插件`，填写代码`358444159`，点击确认（OK）。
3. 下载好后鼠标选中我们的插件`custom sync server redirector`,点击右下角的配置(Config)。
4. 不出意外接着会弹出一个窗口，在里面填写您的服务端的地址。
5. 重启Anki。
### AnkiMobile
AnkiMobile似乎已经支持和自建的同步服务器同步了。至少对于版本Ankimobile 2.0.90(20090.2)来说，似乎是可行的，这是一位IOS系统用户[在anki论坛报告的](https://forums.ankiweb.net/t/ankimobile-self-sync-server-failure-the-one-bundled-in-version-2-1-60-qt6/27862)。

对于详细的配置步骤，如果正在使用AnkiMobile的用户愿意贡献出宝贵的时间和睿智提交一个PR，详细讲解如何设置AnkiMobile来和自建的同步服务器同步，我们将无比感谢。

如果设置完成后发现不能同步可以参考下面的内容再试一次：
> If you're using AnkiMobile and are unable to connect to a server on your local network, please go into the iOS settings, locate Anki near the bottom, and toggle "Allow Anki to access local network" off and then on again.

上面的内容摘自[ANki tutorial](https://docs.ankiweb.net/sync-server.html#client-setup)
### AnkiDroid
打开AnkiDroid,依次进入 `设置（Settings）` -> `高级（Advanced）` -> `自定义同步服务器（Custom sync server）` (对于2.16及以上的版本，依次进入 `设置（Settings）` -> `同步(Sync)` -> `Custom sync server自定义同步服务器（Custom sync server）` )。

除非设置反向代理来处理加密连接，我们使用`HTTP`协议。端口可以是默认的`27701`或者可以在配置文件`ankisyncd.toml`中设置您中意的端口。

安卓端提供了和Anki `endpoint`类似的两个地址来同步收藏数据(Collection)和媒体文件(Media),分别是`同步地址(Sync url)` and the `媒体文件同步地址(Media sync url)`,但是在新版2.16中出现了些微的改变。

举个例子，假设我们的服务器IP地址为``192.0.0.0``,而且我们使用HTTP协议，`27701`作为端口，相应的地址是，

同步地址(Sync url):`http://192.0.0.0:27701`

媒体文件同步地址(Media sync url): `http://192.0.0.0:27701/msync`

在2.16及以上版本中,

同步地址(Sync url):`http://192.0.0.0:27701/sync/`

媒体文件同步地址(Media sync url): `http://192.0.0.0:27701/msync/`

想要支持`https`，查看文件[certificate setup](docs/CERTS.md) （注：2.16版本允许不安全HTTP连接）；反向代理如何设置，查看文件[reverse proxy setup](docs/REVERSE_PROXY.md)。

## 贡献
如果您有建议或者批评，请提交问题或者PR，我们洗耳恭听。具体操作查看文件[CONTRIBUTING.md](CONTRIBUTING.md)。
## 配置
### 环境变量
支持通过环境变量添加账号啦。
|键|值|
|-|-|
|ANKISYNCD_USERNAME|用户名,如果设置则非空|
|ANKISYNCD_PASSWORD|密码,如果设置则非空|
### 可选的服务端配置
注意，这并不是必选项，这一步可以略过。如果您想改变服务端同步数据存储位置或者改变监听端口，可以修改我们提供的配置文件`ankisyncd.toml`,它也在解压缩后的文件夹里面，最后运行如下命令（注：下面的命令适用于linux/和macOS，使用Windows的用户将`ankisyncd`替换成`ankisyncd.exe`,配置文件`ankisyncd.toml`的具体路径根据您计算机配置文件的实际路径而定），
```
./ankisyncd  --config /path/to/ankisyncd.toml
```

## 许可
See [LICENSE](LICENSE)

## 引用
本项目的建立，与另外两个项目密不可分，它们是 [ankicommunity/anki-sync-server](https://github.com/ankicommunity/anki-sync-server) ,
[ankitects/anki](https://github.com/ankitects/anki),我们几乎复刻了`anki-sync-server`中的媒体同步的实现逻辑；而对于`Anki`,我们使用了它的Collection同步API，所以，如果我们不在能够访问到这个API，那么这个项目就停摆了。

