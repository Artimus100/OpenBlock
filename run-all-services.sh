#!/bin/bash

# Block Builder Service Orchestrator
# This script runs all services in parallel for the permissionless block builder

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

print_error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

print_info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}"
}

# Function to check if a service is running
check_service() {
    local port=$1
    local service_name=$2
    
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        print_status "$service_name is running on port $port"
        return 0
    else
        print_error "$service_name is not running on port $port"
        return 1
    fi
}

# Function to kill processes on specific ports
cleanup_ports() {
    local ports=("3001" "4000" "6379" "3000")
    
    print_info "Cleaning up existing processes..."
    
    for port in "${ports[@]}"; do
        local pid=$(lsof -ti:$port 2>/dev/null || true)
        if [ ! -z "$pid" ]; then
            print_warning "Killing process on port $port (PID: $pid)"
            kill -9 $pid 2>/dev/null || true
        fi
    done
    
    sleep 2
}

# Function to start Redis
start_redis() {
    print_info "Starting Redis server..."
    
    # Check if Redis is already running
    if check_service 6379 "Redis" 2>/dev/null; then
        return 0
    fi
    
    # Start Redis in background
    redis-server --daemonize yes --port 6379 --loglevel notice
    sleep 2
    
    if check_service 6379 "Redis"; then
        print_status "Redis started successfully"
    else
        print_error "Failed to start Redis"
        exit 1
    fi
}

# Function to start API server
start_api_server() {
    print_info "Starting API server..."
    
    cd api-server
    
    # Install dependencies if needed
    if [ ! -d "node_modules" ]; then
        print_info "Installing API server dependencies..."
        npm install
    fi
    
    # Start in development mode
    npm run dev &
    API_PID=$!
    
    cd ..
    
    # Wait for API server to start
    sleep 5
    
    if check_service 3001 "API Server"; then
        print_status "API server started successfully (PID: $API_PID)"
    else
        print_error "Failed to start API server"
        return 1
    fi
}

# Function to start block engine
start_block_engine() {
    print_info "Starting Rust block engine..."
    
    cd block-engine
    
    # Build if needed
    if [ ! -f "target/debug/block-engine" ]; then
        print_info "Building block engine..."
        cargo build
    fi
    
    # Start the auction demo
    cargo run --bin auction_demo &
    BLOCK_ENGINE_PID=$!
    
    cd ..
    
    print_status "Block engine started (PID: $BLOCK_ENGINE_PID)"
}

# Function to start frontend dashboard (if exists)
start_dashboard() {
    if [ -d "frontend-dashboard" ]; then
        print_info "Starting Next.js dashboard..."
        
        cd frontend-dashboard
        
        # Install dependencies if needed
        if [ ! -d "node_modules" ]; then
            print_info "Installing dashboard dependencies..."
            npm install
        fi
        
        # Start in development mode
        npm run dev &
        DASHBOARD_PID=$!
        
        cd ..
        
        # Wait for dashboard to start
        sleep 5
        
        if check_service 3000 "Dashboard"; then
            print_status "Dashboard started successfully (PID: $DASHBOARD_PID)"
        else
            print_warning "Dashboard may still be starting..."
        fi
    else
        print_warning "Frontend dashboard not found, skipping..."
    fi
}

# Function to run treasury demo
run_treasury_demo() {
    print_info "Running treasury simulation..."
    
    cd api-server
    
    # Wait a bit for services to be ready
    sleep 3
    
    # Run treasury demo
    npx ts-node src/demos/treasury-demo.ts
    
    cd ..
}

# Function to display service status
show_status() {
    echo
    echo -e "${CYAN}========================================${NC}"
    echo -e "${CYAN}         SERVICE STATUS SUMMARY        ${NC}"
    echo -e "${CYAN}========================================${NC}"
    
    check_service 6379 "Redis Server" || true
    check_service 3001 "API Server" || true
    check_service 4000 "Mock Validator" || true
    check_service 3000 "Dashboard" || true
    
    echo
    echo -e "${CYAN}Available Endpoints:${NC}"
    echo -e "${BLUE}  • API Health:       ${NC}http://localhost:3001/health"
    echo -e "${BLUE}  • Bundle Submission: ${NC}http://localhost:3001/api/bundles"
    echo -e "${BLUE}  • Metrics:          ${NC}http://localhost:3001/api/metrics"
    echo -e "${BLUE}  • Treasury:         ${NC}http://localhost:3001/api/metrics/treasury"
    echo -e "${BLUE}  • Mock Validator:   ${NC}http://localhost:4000/submit_block"
    if check_service 3000 "Dashboard" 2>/dev/null; then
        echo -e "${BLUE}  • Dashboard:        ${NC}http://localhost:3000"
    fi
    
    echo
    echo -e "${CYAN}Useful Commands:${NC}"
    echo -e "${BLUE}  • Submit Bundle:    ${NC}curl -X POST http://localhost:3001/api/bundles -H 'Content-Type: application/json' -d '{\"tip\":1500,\"searcher_pubkey\":\"test\",\"transactions\":[\"tx1\"]}'"
    echo -e "${BLUE}  • View Metrics:     ${NC}curl http://localhost:3001/api/metrics | jq ."
    echo -e "${BLUE}  • View Treasury:    ${NC}curl http://localhost:3001/api/metrics/treasury | jq ."
    echo -e "${BLUE}  • Export Treasury:  ${NC}curl 'http://localhost:3001/api/metrics/treasury/export?format=csv'"
    
    echo
    echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}"
}

# Function to handle cleanup on exit
cleanup() {
    echo
    print_info "Shutting down services..."
    
    # Kill background processes
    [ ! -z "$API_PID" ] && kill $API_PID 2>/dev/null || true
    [ ! -z "$BLOCK_ENGINE_PID" ] && kill $BLOCK_ENGINE_PID 2>/dev/null || true
    [ ! -z "$DASHBOARD_PID" ] && kill $DASHBOARD_PID 2>/dev/null || true
    
    # Cleanup ports
    cleanup_ports
    
    print_status "All services stopped"
    exit 0
}

# Main execution
main() {
    echo -e "${PURPLE}"
    echo "╔════════════════════════════════════════╗"
    echo "║     Permissionless Block Builder       ║"
    echo "║         Service Orchestrator           ║"
    echo "╚════════════════════════════════════════╝"
    echo -e "${NC}"
    
    # Set up signal handlers
    trap cleanup SIGINT SIGTERM
    
    # Parse command line arguments
    case "${1:-start}" in
        "clean")
            cleanup_ports
            exit 0
            ;;
        "status")
            show_status
            exit 0
            ;;
        "demo")
            print_info "Running treasury demo only..."
            run_treasury_demo
            exit 0
            ;;
        "start"|"")
            # Continue with normal startup
            ;;
        *)
            echo "Usage: $0 [start|clean|status|demo]"
            echo "  start  - Start all services (default)"
            echo "  clean  - Clean up running processes"
            echo "  status - Show service status"
            echo "  demo   - Run treasury demo"
            exit 1
            ;;
    esac
    
    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ] || [ ! -d "api-server" ]; then
        print_error "Please run this script from the project root directory"
        exit 1
    fi
    
    print_info "Starting all services..."
    
    # Clean up any existing processes
    cleanup_ports
    
    # Start services in order
    start_redis
    start_api_server
    start_block_engine
    start_dashboard
    
    # Run treasury demo
    run_treasury_demo
    
    # Show status
    show_status
    
    # Keep script running
    print_info "All services are running. Monitoring..."
    
    while true; do
        sleep 10
        
        # Check if critical services are still running
        if ! check_service 6379 "Redis" 2>/dev/null; then
            print_error "Redis stopped unexpectedly"
            start_redis
        fi
        
        if ! check_service 3001 "API Server" 2>/dev/null; then
            print_error "API Server stopped unexpectedly"
            start_api_server
        fi
    done
}

# Run main function
main "$@"
