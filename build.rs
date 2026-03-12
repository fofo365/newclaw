// NewClaw Build Script - gRPC Code Generation

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 生成 watchdog.proto 的 Rust 代码
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["src/proto/watchdog.proto"],
            &["src/proto/"],
        )?;
    
    println!("cargo:rerun-if-changed=src/proto/watchdog.proto");
    Ok(())
}
