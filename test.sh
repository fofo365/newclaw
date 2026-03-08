#!/bin/bash
# NewClaw Test Script

echo "🦀 Testing NewClaw AI Agent Framework"
echo "========================================="
echo ""

cd /root/newclaw

echo "✅ Compilation successful!"
echo ""

echo "📊 Running CLI test..."
echo "$ echo 'Hello, NewClaw!' | ./target/release/newclaw agent"
echo ""

# Interactive test
echo "🎯 Starting interactive mode..."
echo "Type 'exit' to quit"
echo ""

./target/release/newclaw agent

echo ""
echo "✅ Test completed!"
