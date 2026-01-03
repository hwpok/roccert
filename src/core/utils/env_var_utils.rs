use anyhow::{Context, Result, bail};

// 从环境变量中获取值
pub fn get_env_var(var_name: &str) -> Result<String> {
    if var_name.starts_with("${") && var_name.ends_with("}") {
        let inner_key = var_name.trim_start_matches("${").trim_end_matches("}");
        std::env::var(inner_key).with_context(|| format!("var name: {} has no value", var_name))
    } else {
        bail!("var name: {} no value", var_name)
    }
}

// 从环境变量中获取值
pub fn get_env_var_name_value(name_value: &str) -> Result<String> {
    if name_value.starts_with("${") && name_value.ends_with("}") {
        let inner_key = name_value.trim_start_matches("${").trim_end_matches("}");
        std::env::var(inner_key).with_context(|| format!("var name: {} has no value", inner_key))
    } else {
        Ok(name_value.to_string())
    }
}
