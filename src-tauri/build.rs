fn main() {
    // 捕获编译所用的 rustc 版本号（如 "1.88.0"），供关于对话框展示。
    // cargo 保证 RUSTC 环境变量指向当前使用的编译器。
    let rustc = std::env::var("RUSTC").expect("RUSTC environment variable is not set");
    let output = std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .expect("failed to execute rustc --version");
    let stdout =
        String::from_utf8(output.stdout).expect("rustc --version output is not valid UTF-8");
    // rustc --version 输出形如 "rustc 1.88.0 (6b00bc388 2025-06-23)"，取第二段纯版本号。
    let version = stdout
        .split_whitespace()
        .nth(1)
        .expect("rustc --version output has unexpected format");
    println!("cargo:rustc-env=I_NET_RUSTC_VERSION={version}");

    tauri_build::build();
}