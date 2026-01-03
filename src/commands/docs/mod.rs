use std::env;
use std::fs;

use crate::cli::param_enums::Lang;

// 生成文档
pub fn gen_docs(lang: &Lang) {
    // 读取中文和英文的资源文件
    let content_zh = include_str!("doc_zh.html");
    let content_en = include_str!("doc_en.html");

    // 获取当前目录
    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            return;
        }
    };

    // 处理最终的文件名和文件内容
    let (file_name, content) = match lang {
        Lang::En => ("roccert-doc-en.html", content_en),
        Lang::Zh => ("roccert-doc-zh.html", content_zh),
    };

    // 写入文件
    let output_path = current_dir.join(file_name);
    if let Err(e) = fs::write(&output_path, content) {
        eprintln!("Failed to write to {:?}: {}", output_path, e);
        return;
    }

    // 打印成功信息
    println!("Document generated successfully.");
}
