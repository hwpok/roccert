use anyhow::{Result, bail};
use command_group::AsyncCommandGroup;
use tokio::process::Command;
use tokio::time::timeout;

/// 脚本执行器
pub struct ScriptExecutor {
    /// 超时时间（秒），None 表示不超时
    pub timeout_secs: Option<u64>,
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        Self {
            timeout_secs: Some(300),
        }
    }
}

impl ScriptExecutor {
    pub fn new(seconds: u64) -> Self {
        Self {
            timeout_secs: Some(seconds),
        }
    }
    /// 执行 Shell 命令或脚本文件
    /// 如: "npm run build && nginx -s reload" 或 "./deploy.sh"
    pub async fn execute(&self, commond: &str) -> Result<()> {
        // 空命令认为执行成功
        if commond.trim().is_empty() {
            return Ok(());
        }

        // 确定使用的 Shell
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        log::debug!("Start executing script: {}", commond);

        // 构建命令
        let mut child = Command::new(shell) // 使用 tokio::Command
            .arg(flag)
            .arg(commond)
            .group_spawn()?;

        // 执行状态
        let status = if let Some(secs) = self.timeout_secs {
            match timeout(std::time::Duration::from_secs(secs), child.wait()).await {
                Ok(result) => result?,
                Err(_) => bail!("Command timeout after {}s", secs),
            }
        } else {
            child.wait().await? // 无超时（保持原样）
        };

        match status.success() {
            true => Ok(()),
            _ => bail!("Fail to execute"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_execute() {
        let executor = ScriptExecutor::default();
        let result = executor.execute("echo hui").await;
        assert!(result.is_ok(), "Failed");
    }

    #[tokio::test]
    async fn test_time_out() {
        let executor = ScriptExecutor {
            timeout_secs: Some(3),
        };
        let result = executor.execute("ping 127.0.0.1 -n 100").await;
        assert!(result.is_ok(), "Failed");
    }
}
