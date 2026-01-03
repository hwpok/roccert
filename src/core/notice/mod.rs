use crate::core::config::{app_config::Notice, enums::NoticeWebhookProvider};

mod send_msg_tool;

// 发送消息, 所有发送失败不处理, 只记录日志, 不能影响主要的业务流程
pub async fn send_msg(notice: &Notice, message: &str) {
    // 消息参数不可用时,直接返回
    if !notice.enabled {
        return;
    }

    // 配置了webhook通知
    if let Some(webhook_notice) = &notice.webhook_notice {
        // 处理各消息服务商
        let result = match webhook_notice.provider {
            NoticeWebhookProvider::Dingtalk => {
                // 钉钉无关键字, 不发送
                if webhook_notice.extra_param1.is_none() {
                    return;
                }
                let message = format!(
                    "{}: {}",
                    webhook_notice.extra_param1.clone().unwrap_or_default(),
                    message
                );
                send_msg_tool::send_dingtalk_msg(&webhook_notice.url, &message).await
            }
            NoticeWebhookProvider::Wecom => {
                send_msg_tool::send_weico_msg(&webhook_notice.url, message).await
            }
            NoticeWebhookProvider::Slack => {
                send_msg_tool::send_slack_msg(&webhook_notice.url, message).await
            }
        };

        // 记录错误日志
        match result {
            Ok(()) => log::info!("Webhook notification sent successfully"),
            Err(e) => log::error!("Webhook notification send failed, {}, {:?}", message, e),
        }
    }

    // 配置了邮件通知
    if let Some(notice_email) = &notice.email_notice {
        match send_msg_tool::send_email(&notice_email, message) {
            Ok(_) => log::info!("Email notification sent successfully"),
            Err(e) => log::error!("Webhook notification send failed, {}, {:?}", message, e),
        }
    }
}
