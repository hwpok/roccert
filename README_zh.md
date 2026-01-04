## 简介

Roccert（取名源自中东传说中的巨鸟 Roc, 译作大鹏）是一款基于 Rust 语言开发的 SSL 证书自动续期CLI工具。本工具支持通过 HTTP-01 和 DNS-01 两种方式验证域名所有权，可申请单域名、多域名及泛域名（通配符）证书。基于已有证书的过期时间，配合系统 crontab 定时任务，可实现证书的自动续期，无需人工干预。

## &#x20;用法

Roccert 以最符合直觉的方式解决 HTTPS 证书管理问题, 10分钟的投入, 换来一劳永逸.

```
roccert docs -l zh      # 获取英文文档，掌握基本用法
roccert init -i dns01   # 生成 DNS-01 挑战所需的 config.toml 配置文件
roccert test            # 验证配置是否有效, 测试流程
roccert show -t         # 查看并验证已签发证书的有效性
roccert new             # 根据配置申请新证书
roccert show            # 查看并验证已签发证书的有效性

# Cron 任务加入定时调度，实现自动续期
5 9 * * * /usr/local/share/roccert/roccert renew /usr/local/share/roccert/config.toml > /usr/local/share/roccert/roccert.log 2>&1

```

## &#x20;未来路线

作为一款旨在解放双手的域名证书运维工具，我们将持续聚焦自动化、轻量化和高兼容性三大核心方向。 在短期内，我们计划扩展对更多证书颁发机构（CA）API 的支持，包括 ZeroSSL、Buypass Go SSL 等；同时集成更多域名解析服务商的 API，例如帝恩思、新网（Xinnet）、西部数码、ZDNS、Google Cloud DNS、Microsoft Azure DNS、Cloudflare、GoDaddy 和 Namecheap。 本项目以 Rust 语言的高性能与内存安全特性为基础，致力于构建结构清晰、资源占用低、易于维护的轻量级运维工具。我们希望通过开源协作，吸纳社区的创意与贡献，逐步打磨出一款开发者与运维人员值得信赖的域名证书管理助手。

如有疑问，请联系： <hwpok@163.com> 或 <hwpok@qq.com>
