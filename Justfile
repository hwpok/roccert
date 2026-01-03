# set shell := ["bash", "-c"]
set shell := ["powershell.exe", "-c"]

# --------------- 开发常用 ----------------
# 指定默认
default: r

# 编译
b:
   cargo build

# 编译并运行
r:
   cargo run

# 编译Release版本
br:
    cargo build --release

# 检查
c:
    cargo check

# 清理
cl:
    cargo clean

# 运行debug中的App
rd:
   .\target\debug\renew-cert.exe

# 运行release中的App
rr:
   .\target\release\renew-cert.exe

# 添加crate
a v: 
   cargo add {{v}}
# --------------- 代码质量 ----------------
# 代码规范检查
cy:
   cargo clippy

# 格式化代码
f:
   cargo fmt

# 运行单元测试
t v:
  cargo test --test {{v}} -- --test-threads=1

# 运行单元测试
tn v:
  cargo test --test {{v}} -- --nocapture --test-threads=1
  