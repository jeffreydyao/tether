#!/usr/bin/env bash
# =============================================================================
# Tether MCP Server - Cloud Run Deployment Script
# =============================================================================
#
# This script deploys the Tether MCP server to Google Cloud Run.
#
# Prerequisites:
#   - gcloud CLI installed and configured
#   - Docker installed (for local builds)
#   - Authenticated to Google Cloud: gcloud auth login
#   - Project set: gcloud config set project YOUR_PROJECT_ID
#
# Usage:
#   ./deploy.sh [TICKET]
#
# Environment Variables:
#   TETHER_DUMBPIPE_TICKET - Required if not passed as argument
#   GCP_PROJECT_ID         - Google Cloud project ID (default: from gcloud config)
#   GCP_REGION             - Deployment region (default: us-central1)
#   SERVICE_NAME           - Cloud Run service name (default: tether-mcp)
#
# =============================================================================

set -euo pipefail

# Configuration with defaults
GCP_PROJECT_ID="${GCP_PROJECT_ID:-$(gcloud config get-value project 2>/dev/null)}"
GCP_REGION="${GCP_REGION:-us-central1}"
SERVICE_NAME="${SERVICE_NAME:-tether-mcp}"
IMAGE_NAME="gcr.io/${GCP_PROJECT_ID}/${SERVICE_NAME}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Get the dumbpipe ticket
TICKET="${1:-${TETHER_DUMBPIPE_TICKET:-}}"

if [[ -z "$TICKET" ]]; then
    log_error "Dumbpipe ticket is required."
    echo ""
    echo "Usage: $0 <TETHER_DUMBPIPE_TICKET>"
    echo "   or: TETHER_DUMBPIPE_TICKET=<ticket> $0"
    echo ""
    echo "Get the ticket from your Raspberry Pi's web UI."
    exit 1
fi

# Validate prerequisites
if [[ -z "$GCP_PROJECT_ID" ]]; then
    log_error "GCP_PROJECT_ID not set and couldn't determine from gcloud config."
    echo "Run: gcloud config set project YOUR_PROJECT_ID"
    exit 1
fi

log_info "Deploying Tether MCP Server to Cloud Run"
log_info "  Project:  ${GCP_PROJECT_ID}"
log_info "  Region:   ${GCP_REGION}"
log_info "  Service:  ${SERVICE_NAME}"
log_info "  Image:    ${IMAGE_NAME}"

# Confirm deployment
echo ""
read -p "Continue with deployment? (y/N) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    log_warn "Deployment cancelled."
    exit 0
fi

# Step 1: Enable required APIs
log_info "Enabling required Google Cloud APIs..."
gcloud services enable \
    run.googleapis.com \
    containerregistry.googleapis.com \
    cloudbuild.googleapis.com \
    --project="${GCP_PROJECT_ID}"

# Step 2: Build and push container image
log_info "Building container image..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Submit build to Cloud Build using the MCP Dockerfile
gcloud builds submit \
    --project="${GCP_PROJECT_ID}" \
    --tag="${IMAGE_NAME}:latest" \
    --timeout=1200s \
    --file="${PROJECT_ROOT}/crates/tether-mcp/Dockerfile" \
    "${PROJECT_ROOT}"

log_info "Container image built and pushed to ${IMAGE_NAME}:latest"

# Step 3: Deploy to Cloud Run
log_info "Deploying to Cloud Run..."

gcloud run deploy "${SERVICE_NAME}" \
    --project="${GCP_PROJECT_ID}" \
    --region="${GCP_REGION}" \
    --image="${IMAGE_NAME}:latest" \
    --platform=managed \
    --allow-unauthenticated \
    --port=8080 \
    --memory=512Mi \
    --cpu=1 \
    --min-instances=0 \
    --max-instances=10 \
    --timeout=300 \
    --concurrency=80 \
    --set-env-vars="TETHER_DUMBPIPE_TICKET=${TICKET}" \
    --set-env-vars="MCP_TRANSPORT=stdio" \
    --set-env-vars="RUST_LOG=info,tether_mcp=debug"

# Step 4: Get the service URL
SERVICE_URL=$(gcloud run services describe "${SERVICE_NAME}" \
    --project="${GCP_PROJECT_ID}" \
    --region="${GCP_REGION}" \
    --format='value(status.url)')

log_info "Deployment complete!"
echo ""
echo "============================================"
echo "Tether MCP Server deployed successfully!"
echo "============================================"
echo ""
echo "Service URL: ${SERVICE_URL}"
echo ""
echo "To use with an MCP client, configure:"
echo "  URL: ${SERVICE_URL}/mcp"
echo "  Transport: streamable-http"
echo ""
echo "To view logs:"
echo "  gcloud run logs tail ${SERVICE_NAME} --region=${GCP_REGION}"
echo ""
echo "To update the ticket:"
echo "  gcloud run services update ${SERVICE_NAME} \\"
echo "    --region=${GCP_REGION} \\"
echo "    --set-env-vars=\"TETHER_DUMBPIPE_TICKET=<new-ticket>\""
echo ""
