#!/bin/bash

# Test script for the Rust version of Cline Auth Service
# This script tests the full authentication flow by:
# 1. Fetching an access token from the local auth service.
# 2. Using that token to make a real API call to the Cline chat completions endpoint.
#
# Prerequisites:
# - The auth-service must be running (`cargo run --release` in the auth-service-rust directory).
# - You must have completed the login flow in your browser.
# - `jq` must be installed to parse the JSON response. (On macOS: `brew install jq`)

echo "Attempting to fetch access token from local auth service..."

# 1. Fetch the token and extract it from the JSON response.
# The -s flag makes curl silent.
ACCESS_TOKEN=$(curl -s http://localhost:8888/token | jq -r .access_token)

if [ -z "$ACCESS_TOKEN" ] || [ "$ACCESS_TOKEN" == "null" ]; then
    echo "Error: Could not retrieve access token."
    echo "Please ensure the auth-service is running and you have logged in."
    exit 1
fi

echo "Access token retrieved successfully."
echo "Making a test API call to Cline's chat completions endpoint..."
echo ""

# 2. Use the token to make an authenticated API request.
# We are sending a simple, non-streaming request to a default model.
# Note: Based on the original Cline extension code, we need specific headers
curl https://api.cline.bot/api/v1/chat/completions \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -H "HTTP-Referer: https://cline.bot" \
  -H "X-Title: Cline" \
  -H "X-Cline-Version: 3.0.0" \
  -d '{
    "model": "x-ai/grok-code-fast-1",
    "messages": [
      {
        "role": "user",
        "content": "Hello, world!"
      }
    ],
    "stream": false
  }'

echo ""
echo ""
echo "Script finished. If you see a JSON response above with a 'choices' array, the token is valid and the API call was successful."
echo "🚀 Rust version test completed!"
