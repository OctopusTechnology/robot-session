#!/bin/bash

# Script to run LiveKit integration tests
# This script starts the Docker services and runs tests from the host

set -e

echo "🚀 Starting LiveKit Integration Test Suite"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if network exists, create if not
ensure_network() {
    if ! docker network ls | grep -q "rk3588-network"; then
        print_status "Creating rk3588-network..."
        docker network create rk3588-network
        print_success "Network rk3588-network created"
    else
        print_status "Network rk3588-network already exists"
    fi
}


# Function to cleanup
cleanup() {
    print_status "Cleaning up..."
    docker-compose down
    print_success "Cleanup completed"
}

# Trap to ensure cleanup on exit
trap cleanup EXIT

# Main execution
main() {
    print_status "Starting LiveKit integration test suite..."
    
    # Ensure network exists
    ensure_network
    
    # Build and start services
    print_status "Building and starting Docker services..."
    docker-compose up --build -d
    
    # Give services a moment to fully initialize
    print_status "Waiting for services to fully initialize..."
    sleep 10
    
    # Check service endpoints
    print_status "Checking service endpoints..."
    
    if curl -f http://localhost:7880 >/dev/null 2>&1; then
        print_success "LiveKit is responding on port 7880"
    else
        print_warning "LiveKit may not be fully ready on port 7880"
    fi
    
    if curl -f http://localhost:8080/health >/dev/null 2>&1; then
        print_success "Session Manager is responding on port 8080"
    else
        print_warning "Session Manager may not be fully ready on port 8080"
    fi
    
    # Run the integration tests
    print_status "Running integration tests..."
    cd session-manager
    
    # Run the basic integration test
    print_status "Running basic integration test..."
    if cargo test test_session_manager_integration --release -- --nocapture; then
        print_success "Basic integration test passed"
    else
        print_error "Basic integration test failed"
        exit 1
    fi
    
    # Run the LiveKit integration test
    print_status "Running LiveKit integration test..."
    if cargo test test_session_creation_with_livekit_client_join --release -- --nocapture; then
        print_success "LiveKit integration test passed"
    else
        print_error "LiveKit integration test failed"
        exit 1
    fi
    
    # Run microservice simulation test
    print_status "Running microservice simulation test..."
    if cargo test test_microservice_join_simulation --release -- --nocapture; then
        print_success "Microservice simulation test passed"
    else
        print_error "Microservice simulation test failed"
        exit 1
    fi
    
    print_success "All tests passed! 🎉"
}

# Check if Docker and Docker Compose are available
if ! command -v docker &> /dev/null; then
    print_error "Docker is not installed or not in PATH"
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    print_error "Docker Compose is not installed or not in PATH"
    exit 1
fi

# Check if we're in the right directory
if [ ! -f "docker-compose.yml" ]; then
    print_error "docker-compose.yml not found. Please run this script from the project root directory."
    exit 1
fi

# Run main function
main "$@"