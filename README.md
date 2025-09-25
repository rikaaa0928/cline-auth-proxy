# Standalone OAuth2 Token Service (Rust)

This project provides a self-contained, high-performance authentication service written in Rust. It handles the OAuth2 flow for services that use Firebase for authentication, providing a persistent, auto-refreshing access token through a simple REST API.

It is designed to be run locally or in a container, making it a flexible solution for providing authentication tokens to external applications or scripts.

## Features

- 🚀 **High-Performance**: Built with Rust, Axum, and Tokio for excellent performance and low resource usage.
- 🔐 **OAuth2/Firebase Integration**: Handles the complete Firebase authentication flow, including the client-side credential exchange.
- 🔄 **Automatic Token Refresh**: Automatically refreshes expired `id_token`s using the stored `refreshToken`.
- 💾 **Persistent Login State**: Remembers login sessions across restarts by storing the `refreshToken` in a local `auth_state.json` file.
- 📦 **Container-Ready**: Comes with a multi-stage `Dockerfile` for building lightweight, secure container images.
- ⚙️ **Dual-Mode Operation**: Supports both local development (with automatic browser opening) and containerized (headless) environments via an environment variable.
- 🌐 **Simple REST API**: Exposes clean endpoints to initiate login and retrieve tokens.

## How It Works

The service orchestrates a hybrid authentication flow:

1.  **Login Initiation**: The `/login` endpoint generates a unique authentication URL for the target service (e.g., `app.cline.bot`).
2.  **Client-Side Exchange**: The user is directed to a self-hosted HTML page (`handle-auth.html`) after successful login. This page, using the official Firebase JS SDK, performs the critical `signInWithCredential` step in the user's browser to exchange the temporary OAuth credential for a long-lived `refreshToken`.
3.  **Secure Backend Storage**: The `refreshToken` is then sent to the Rust backend via a `POST` request to `/callback` and is securely stored in `auth_state.json`.
4.  **Token Provisioning**: External applications can then request a short-lived, valid `id_token` from the `/token` or `/raw-token` endpoints. The service handles all necessary refreshes automatically.

This architecture ensures that sensitive operations involving the Firebase Client SDK are performed in the browser, while the Rust backend manages the secure storage and refreshment of tokens.

## Installation and Usage

### Prerequisites

- **Rust**: Required for local development. (Install via [rustup](https://rustup.rs/)).
- **Docker**: Required for containerized deployment.

---

### Option 1: Run Locally

This is the recommended mode for local development.

1.  **Run the service**:
    ```bash
    cd auth-service-rust
    cargo run --release
    ```
    The service will start on `http://127.0.0.1:8888`. If no saved login state is found, it will automatically open the login page in your browser.

2.  **Log in (if required)**:
    If the browser opens, complete the authentication process.

3.  **Get a Token**:
    Once authenticated, you can fetch a token:
    ```bash
    # Get token in JSON format
    curl http://localhost:8888/token

    # Get raw token as plain text
    curl http://localhost:8888/raw-token
    ```

---

### Option 2: Run with Docker

This mode is ideal for production or isolated environments.

1.  **Build the Docker image**:
    ```bash
    cd auth-service-rust
    docker build -t auth-token-service .
    ```

2.  **Run the container**:
    ```bash
    # Create a local directory for persistent data
    mkdir -p ./data

    # Run the container, mounting the data directory
    docker run --rm -it \
      -p 8888:8888 \
      -v "$(pwd)/data:/app/data" \
      auth-token-service
    ```
    > **Note**: The `Dockerfile` sets `HOST_BINDING=0.0.0.0` and `AUTH_STATE_DIR=/app/data` automatically. The `-v` flag ensures your login session persists across container restarts.

3.  **Log in (if required)**:
    If you are running the service for the first time, open your browser and navigate to `http://localhost:8888/login`. The page will display a login URL. Copy and paste this URL into your browser to complete authentication.

4.  **Get a Token**:
    Use `curl` as described in the local setup to fetch your token.

## API Endpoints

| Endpoint | Method | Description |
| :--- | :--- | :--- |
| `/login` | `GET` | Initiates the authentication flow. |
| `/token` | `GET` | Returns the current access token in JSON format. |
| `/raw-token`| `GET` | Returns the current access token as plain text. |
| `/callback` | `POST`| **Internal**: Receives the `refreshToken` from the client-side script. |

## Security

- The `refreshToken` is stored locally in `auth_state.json`. This file should be treated as sensitive and **must not** be committed to version control. The provided `.gitignore` file already excludes it.
- The service binds to `0.0.0.0`, making it accessible on your local network. Ensure your firewall is configured appropriately if this is a concern.

## License

This project is provided as-is. Please adapt the license to your needs.


