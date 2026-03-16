// NewClaw Build Script - gRPC Code Generation

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 生成 watchdog.proto 的 Rust 代码
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["src/proto/watchdog.proto"],
            &["src/proto/"],
        )?;

    // 生成 frame.proto 的 Rust 代码（飞书 WebSocket 协议）
    let out_dir = PathBuf::from("src/proto");
    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(&["src/proto/frame.proto"], &["src/proto/"])?;

    println!("cargo:rerun-if-changed=src/proto/watchdog.proto");
    println!("cargo:rerun-if-changed=src/proto/frame.proto");
    Ok(())
}
