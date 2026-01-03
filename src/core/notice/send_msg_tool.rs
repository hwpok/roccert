use anyhow::{Context, Result, ensure};

use lettre::{
    SmtpTransport, Transport,
    message::{Mailbox, Message, header::ContentType},
    transport::smtp::authentication::Credentials,
};

use serde_json::json;

use crate::core::config::app_config::NoticeEmail;

// 发送微信消息
pub async fn send_weico_msg(url: &str, msg: &str) -> Result<()> {
    let payload = json!({
        "msgtype": "text",
        "text": {
            "content": msg,
            "mentioned_list": ["@all"]
        }
    });
    send_data(url, serde_json::to_string(&payload)?).await
}

// 发送钉钉消息
pub async fn send_dingtalk_msg(url: &str, msg: &str) -> Result<()> {
    let payload = json!({
        "msgtype": "text",
        "text": {
            "content": msg
        }
    });
    send_data(url, serde_json::to_string(&payload)?).await
}

// 发送Slack消息
pub async fn send_slack_msg(url: &str, msg: &str) -> Result<()> {
    let payload = json!({
        "text": msg
    });
    send_data(url, serde_json::to_string(&payload)?).await
}

// 发送公共方法
async fn send_data(url: &str, msg: String) -> Result<()> {
    //println!("发出的消息: {}", msg);
    let client = reqwest::Client::new();
    let _res = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(msg)
        .send()
        .await?;
    //println!("返回的消息: {:?}", res);
    Ok(())
}

// 通用的发送函数
pub fn send_email(param: &NoticeEmail, body: &str) -> Result<()> {
    // 处理收件人地址
    let to_mailboxes: Vec<Mailbox> = param
        .to
        .iter()
        .map(|to_addr| {
            to_addr.parse().with_context(|| {
                // 使用闭包延迟格式化，并在日志中提供结构化字段
                format!(
                    "Failed to parse email address: '{}'. Please check the format.",
                    to_addr
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // 收件人列表为空
    ensure!(!to_mailboxes.is_empty(), "to emails list is empty");

    // 主收件人和抄送人
    let first_recipient = to_mailboxes.first().unwrap();
    let cc_recipients = &to_mailboxes[1..];

    // 消息构建器
    let mut builder = Message::builder()
        .header(ContentType::TEXT_PLAIN)
        .from(param.from.parse().context("Invalid email format")?)
        .to(first_recipient.to_owned());

    // 添加所有抄送人
    for cc in cc_recipients {
        builder = builder.cc(cc.clone());
    }

    // 设置主题和邮件体
    let message = builder.subject(&param.subject).body(body.to_string())?;

    // 创建凭证
    let creds = Credentials::new(param.username.clone(), param.password.clone());
    let mailer = SmtpTransport::relay(&param.smtp)
        .context(format!(
            "Failed to connect to the SMTP server: {}",
            param.smtp
        ))?
        .port(param.smtp_port) // 动态设置端口
        .credentials(creds)
        .build();

    // 发送并处理错误
    mailer.send(&message).context("Failed to Send Email")?;
    log::debug!("The email was sent successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_weico_msg() {
        dotenvy::dotenv().unwrap();
        let weico_webhook = std::env::var("WEICO_HOOK").unwrap();
        let message = "Hello, Weico Robot, this is just a test message";
        assert!(
            send_weico_msg(&weico_webhook, message).await.is_ok(),
            "Failed to send message"
        );
    }

    #[tokio::test]
    async fn test_send_dingtalk_msg() {
        dotenvy::dotenv().unwrap();
        let dingtalk_webhook = std::env::var("DINGTALK_HOOK").unwrap();
        let message = "ROBOT, Hello, Dingtalk Robot, this is just a test message";
        assert!(
            send_dingtalk_msg(&dingtalk_webhook, message).await.is_ok(),
            "Failed to send message",
        );
    }

    #[tokio::test]
    async fn test_send_mail() {
        dotenvy::dotenv().unwrap();
        let (username, password) = (
            std::env::var("SMTP_USERNAME").unwrap(),
            std::env::var("SMTP_PASSWORD").unwrap(),
        );

        // 构建发送参数465/587
        let paras: NoticeEmail = NoticeEmail {
            smtp: "smtp.163.com".to_string(),
            smtp_port: 465,
            username,
            password,
            from: "hwpok@163.com".to_string(),
            to: vec!["hwpok@qq.com".to_string()],
            subject: "For roccert test".to_string(),
            template_file: None,
        };
        let result = send_email(&paras, "this is just a test email");
        assert!(result.is_ok(), "Failed to send email")
    }
}
