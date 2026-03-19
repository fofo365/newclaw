// 记忆迁移演示
use newclaw::tools::memory::MemoryTool;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 开始记忆迁移演示...\n");

    // 创建记忆工具
    let memory_dir = PathBuf::from("./data/memory");
    let openclaw_workspace = PathBuf::from("/root/.openclaw/workspace");

    let tool = MemoryTool::new(memory_dir.clone(), openclaw_workspace);

    // 1. 自动迁移
    println!("1️⃣ 自动迁移 OpenClaw 记忆...");
    tool.auto_migrate().await?;
    println!("   ✅ 迁移完成\n");

    // 2. 统计信息
    println!("2️⃣ 记忆统计信息...");
    let stats = tool.stats().await?;
    println!("   📊 总文件数: {}", stats.total_files);
    println!("   📊 总大小: {} bytes ({:.2} KB)\n", stats.total_size, stats.total_size as f64 / 1024.0);

    // 3. 搜索测试
    println!("3️⃣ 搜索测试: 'NewClaw'");
    let results = tool.keyword_search("NewClaw", 3).await?;
    println!("   📝 找到 {} 个结果:\n", results.len());

    for (i, entry) in results.iter().enumerate() {
        println!("   结果 #{}", i + 1);
        println!("   - 路径: {}", entry.path);
        println!("   - 来源: {}", entry.source);
        println!("   - 内容预览:");
        for line in entry.content.lines().take(3) {
            println!("     {}", line);
        }
        println!();
    }

    // 4. 读取测试
    println!("4️⃣ 读取测试: MEMORY.md (前 10 行)");
    let content = tool.get("MEMORY.md", Some(0), Some(10)).await?;
    println!("   📄 内容:\n");
    for line in content.lines() {
        println!("     {}", line);
    }

    println!("\n✅ 演示完成！");
    Ok(())
}
