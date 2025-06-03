#!/bin/bash

set -e

echo "🚀 Starting Local LiveKit Integration Test"
echo "=========================================="

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "🧹 Cleaning up..."
    
    # Stop session manager if running
    if [ ! -z "$SESSION_MANAGER_PID" ]; then
        echo "Stopping session manager (PID: $SESSION_MANAGER_PID)..."
        kill $SESSION_MANAGER_PID 2>/dev/null || true
        wait $SESSION_MANAGER_PID 2>/dev/null || true
    fi
    
    # Stop LiveKit Docker
    echo "Stopping LiveKit Docker container..."
    docker-compose -f docker-compose.livekit-only.yml down
    
    echo "✅ Cleanup completed"
}

# Set trap to cleanup on script exit
trap cleanup EXIT INT TERM

echo "📦 Starting LiveKit in Docker..."
docker-compose -f docker-compose.livekit-only.yml up -d

echo "⏳ Waiting for LiveKit to be ready..."
sleep 5

# Check if LiveKit is responding
echo "🔍 Checking LiveKit health..."
for i in {1..30}; do
    if curl -s http://localhost:7880 > /dev/null 2>&1; then
        echo "✅ LiveKit is ready!"
        break
    fi
    echo "Waiting for LiveKit... attempt $i/30"
    sleep 2
done

# Verify LiveKit is actually ready
if ! curl -s http://localhost:7880 > /dev/null 2>&1; then
    echo "❌ LiveKit failed to start"
    exit 1
fi

echo ""
echo "🔧 Building session manager..."
cd session-manager
cargo build --release

echo ""
echo "🚀 Starting session manager locally..."
RUST_LOG=session_manager=trace,livekit=trace,livekit_api=trace,tower_http=debug \
    cargo run --release -- --config config/local.toml &
SESSION_MANAGER_PID=$!

echo "Session manager started with PID: $SESSION_MANAGER_PID"

echo "⏳ Waiting for session manager to be ready..."
sleep 3

# Check if session manager is responding
echo "🔍 Checking session manager health..."
for i in {1..15}; do
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo "✅ Session manager is ready!"
        break
    fi
    echo "Waiting for session manager... attempt $i/15"
    sleep 2
done

# Verify session manager is actually ready
if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "❌ Session manager failed to start"
    exit 1
fi

echo ""
echo "🧪 Running integration tests..."
RUST_LOG=session_manager=trace,livekit=trace,livekit_api=trace,tower_http=debug \
    cargo test --release test_session_creation_with_livekit_client_join -- --nocapture

echo ""
echo "✅ Test completed successfully!"