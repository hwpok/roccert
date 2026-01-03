use anyhow::{Context, Result};
use instant_acme::LetsEncrypt;
use instant_acme::RetryPolicy;
use instant_acme::{Account, Identifier, NewOrder, Order};
use instant_acme::{AccountCredentials, NewAccount};
use rcgen::CertificateParams;
use rcgen::DistinguishedName;
use rcgen::DnType;
use rcgen::KeyPair;
use std::fs;
use std::path::{Path, PathBuf};

/// 加载或创建账户口
pub async fn load_or_create_account(
    email: &str,
    credentials_path: &PathBuf,
    dry_run: bool,
) -> Result<Account> {
    log::info!("In cureent, The dry_run value: {}", dry_run);
    let credentials_file = Path::new(credentials_path);

    // 检查凭证JSON文件是否存在
    if credentials_file.exists() {
        log::debug!("Loading account from existing credentials");

        // 读取凭证文件数据
        let credentials_data =
            fs::read_to_string(credentials_file).context("Failed to read credentials file")?;

        // 把Json数据转成用户凭证
        let credentials: AccountCredentials =
            serde_json::from_str(&credentials_data).context("Failed to parse credentials")?;

        // 构建账户
        let account = Account::builder()
            .context("Failed to create account builder")?
            .from_credentials(credentials)
            .await
            .context("Failed to restore account from credentials")?;

        log::debug!("Account restored successfully");
        return Ok(account);
    }

    // 开始创建新账户
    log::debug!("Start creating new ACME account");
    let new_account = NewAccount {
        contact: &[&format!("mailto:{}", email)],
        terms_of_service_agreed: true,
        only_return_existing: false,
    };

    // 是否"空跑", 空跑的话, 使用测试Acme的测试环境
    let directory_url = match dry_run {
        true => LetsEncrypt::Staging.url().to_owned(),
        false => LetsEncrypt::Production.url().to_owned(),
    };

    // 创建账户
    let (account, credentials) = Account::builder()
        .context("Failed to create account builder")?
        .create(&new_account, directory_url, None)
        .await
        .context("Failed to create new account")?;

    // 保存凭证以便将来使用
    let credentials =
        serde_json::to_string_pretty(&credentials).context("Failed to serialize credentials")?;

    // 确保父目录的存在
    if let Some(parent) = credentials_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(credentials_file, credentials).context("Failed to save credentials file")?;

    log::debug!("New account created successfully, credentials saved");
    Ok(account)
}

/// 创建证书订单
pub async fn create_certificate_order(
    account: &Account,
    domain_names: &Vec<String>,
) -> Result<Order> {
    println!("Creating order for domains: {:?}", domain_names);

    // 为订单准备标识符
    let identifiers: Vec<Identifier> = domain_names
        .iter()
        .map(|domain| Identifier::Dns(domain.to_string()))
        .collect();

    // 创建新的订单
    let new_order = NewOrder::new(&identifiers);

    // 提供订单到 ACME 服务器
    let mut order = account
        .new_order(&new_order)
        .await
        .context("Failed to create certificate order")?;

    log::debug!("Order created successfully!");
    log::debug!("Order URL: {}", order.url());

    // 检查此时订单的状态
    let order_status = order.state();
    log::debug!("Initial order status: {:?}", order_status.status);

    Ok(order)
}

// 提交CSR, 获取PM证书
pub async fn submit_csr_and_down_pem_cert(
    domains: &Vec<String>,
    order: &mut Order,
) -> Result<(String, String)> {
    // 准备参数, 创建CSR
    let key_pair = KeyPair::generate()?;
    let mut params = CertificateParams::new(domains.clone())?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, domains[0].clone());
    params.distinguished_name = dn;

    // 创建CSR
    let csr = params.serialize_request(&key_pair)?;
    let mut csr_der = csr.der().to_vec();

    // 提交CSR
    order
        .finalize_csr(&mut csr_der)
        .await
        .context("Failed to submit CSR. Please check if domain validation is complete.")?;

    log::debug!("CSR submitted. Waiting for certificate issuance");

    // 自定义策略：初始延迟 2 秒，最大超时 5 分钟
    let retry_policy = RetryPolicy::new()
        .initial_delay(std::time::Duration::from_secs(2))
        .timeout(std::time::Duration::from_secs(300));

    // 轮询订单, 将返回证书字符
    let cert_chain_pem = order.poll_certificate(&retry_policy).await?;
    let private_key_pem = key_pair.serialize_pem();

    log::debug!(
        "private_key_pem: {}, cert_chain_pem: {}",
        private_key_pem,
        cert_chain_pem
    );

    // 返回私钥和证书链
    Ok((private_key_pem, cert_chain_pem))
}
