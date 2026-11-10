# Assets Manager

![Tests](https://github.com/gboutte/assets-manager/actions/workflows/tests.yml/badge.svg)
![Coverage](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/gboutte/bc333884a8d5cfb4d495a3e56152d12b/raw/coverage.json)

A lightweight static asset server written in Rust for learning purposes. Upload and serve versioned static files (JS, CSS, etc.) from Angular builds or any other frontend framework.

## Planned Features

- **Versioned asset serving** – Files served at `/<tag>/<filename>` URLs
- **Single file upload** – Push individual files with a version tag
- **ZIP upload** – Upload multiple files at once via ZIP archive
- **API token authentication** – Secure upload endpoints
- **Filesystem storage** – Store assets locally
- **Per-domain storage** – Optional (`STORAGE_PER_HOST`): each domain of `ALLOWED_HOSTS` has its own tags, e.g. `v1.0.0` on `assets.domain1.com` and `v1.0.0` on `assets.domain2.com` are different
- **S3-compatible bucket storage** – Store assets in cloud storage
- **Health check endpoint** – Monitor service status
- **List tags** – View all available versions
- **Delete tags** – Remove a version and all its files
- **Usage statistics** – Track hits per tag (time-series database)
- **Auto-cleanup** – Delete tags after X days without hits (configurable)
- **Configurable caching headers** – `Cache-Control: public, max-age=<CACHE_MAX_AGE>, immutable` on served assets
- **CORS configuration**

## Configuration

All configuration is done via environment variables. Create a `.env` file in the project root:

```env
# Server
HOST=0.0.0.0
PORT=8000

# Authentication
# Every variable starting with API_TOKEN is a valid token (empty values are ignored)
API_TOKEN=your-secret-token-here
# API_TOKEN_CI=another-token

# Storage
STORAGE_TYPE=filesystem
STORAGE_PATH=./uploads

# Cache: Cache-Control max-age in seconds for served assets (default: 1 year, 0 = no-cache)
# Assets are sent as immutable: upload new files under a new tag rather than overwriting a tag
# CACHE_MAX_AGE=31536000

# Per-domain storage: assets are stored in <STORAGE_PATH>/<host>/<tag>/ and only served on that host
# Requests for a host that is not in ALLOWED_HOSTS get a 404 (except /health)
# STORAGE_PER_HOST=true
# ALLOWED_HOSTS=assets.domain1.com,assets.domain2.com

# Future: S3 storage
# STORAGE_TYPE=s3
# S3_BUCKET=my-assets-bucket
# S3_REGION=eu-west-1
# S3_ACCESS_KEY=xxx
# S3_SECRET_KEY=xxx

# Future: Auto-cleanup
# AUTO_DELETE_DAYS=30
```

## API Endpoints

### Public Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/tags` | List all available tags |
| `GET` | `/<tag>/<path>` | Serve a file from a specific tag |

### Protected Endpoints (require `Authorization: Bearer <token>`)

`<token>` can be the value of any `API_TOKEN*` variable, so you can give each consumer (CI, deploy script…) its own token and revoke it independently.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/upload/<tag>/<filename>` | Upload a single file |
| `POST` | `/upload/<tag>` | Upload a ZIP archive (extracts contents) |
| `DELETE` | `/tags/<tag>` | Delete a tag and all its files |

## Usage Examples

### Upload a single file

```bash
curl -X POST \
  -H "Authorization: Bearer your-secret-token-here" \
  -F "file=@dist/main.js" \
  http://localhost:8000/upload/v1.0.0/main.js
```

### Upload a ZIP archive

```bash
# Create a ZIP of your build
zip -r build.zip dist/

# Upload it
curl -X POST \
  -H "Authorization: Bearer your-secret-token-here" \
  -F "file=@build.zip" \
  http://localhost:8000/upload/v1.0.0
```

### Access files

```
http://localhost:8000/v1.0.0/main.js
http://localhost:8000/v1.0.0/styles.css
http://localhost:8000/v1.0.0/assets/logo.png
```

### List available tags

```bash
curl http://localhost:8000/tags
```

Response:
```json
{
  "tags": ["v1.0.0", "v1.1.0", "v2.0.0"]
}
```

### Health check

```bash
curl http://localhost:8000/health
```

Response:
```json
{
  "status": "ok"
}
```

## Docker

### Run

```bash
docker run -d \
  --name assets-manager \
  -p 8000:8000 \
  -e API_TOKEN=your-secret-token-here \
  -v assets-data:/data \
  gboutte/assets-manager:latest
```

Uploaded files are stored in the `/data` volume. The image already sets `HOST=0.0.0.0`, `PORT=8000`, `STORAGE_TYPE=filesystem` and `STORAGE_PATH=/data`; only an `API_TOKEN*` variable is required. The server runs as a non-root user (uid `10001`): when mounting a host directory instead of a named volume, make it writable by that uid.

### docker compose

```yaml
services:
  assets-manager:
    image: gboutte/assets-manager:latest
    ports:
      - "8000:8000"
    environment:
      API_TOKEN: your-secret-token-here
      # API_TOKEN_CI: another-token
      # CACHE_MAX_AGE: 31536000
      # STORAGE_PER_HOST: "true"
      # ALLOWED_HOSTS: assets.domain1.com,assets.domain2.com
    volumes:
      - assets-data:/data
    restart: unless-stopped

volumes:
  assets-data:
```

### Build and publish

```bash
docker build -t gboutte/assets-manager:latest .
docker push gboutte/assets-manager:latest
```

The image is built for the architecture of the machine running `docker build`. For ARM servers too, use `docker buildx build --platform linux/amd64,linux/arm64 -t gboutte/assets-manager:latest --push .`

### Per-domain storage behind a reverse proxy

The domain is read from the `Host` header, so the proxy must forward it (nginx: `proxy_set_header Host $host;`). `X-Forwarded-Host` is not used, as any client can set it.

When enabling `STORAGE_PER_HOST` on an existing storage, move the existing tags into `<STORAGE_PATH>/<host>/` first, otherwise they are no longer served.

## Development

### Prerequisites

- Rust (latest stable)

### Run locally

```bash
# Clone the repository
git clone https://github.com/your-username/assets-manager.git
cd assets-manager

# Create .env file
cp .env.example .env
# Edit .env with your configuration

# Run in development mode
cargo run

# Run tests
cargo test
```

### Project Structure

```
assets-manager/
├── src/
│   ├── main.rs           # Application entry point
│   ├── config.rs         # Environment configuration
│   ├── routes/           # HTTP route handlers
│   │   ├── mod.rs
│   │   ├── health.rs
│   │   ├── tags.rs
│   │   ├── upload.rs
│   │   └── serve.rs
│   ├── storage/          # Storage backends
│   │   ├── mod.rs
│   │   ├── filesystem.rs
│   │   └── s3.rs         # (future)
│   ├── auth/             # Authentication
│   │   └── mod.rs
│   └── models/           # Data structures
│       └── mod.rs
├── tests/                # Integration tests
├── uploads/              # Default local storage directory
├── .env.example
├── Cargo.toml
└── README.md
```

## Testing

Tests are created for each feature. Run the test suite with:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_upload_single_file
```

## Architecture Decisions

### Why Rocket?
Rocket provides a clean, type-safe API with built-in support for:
- Request guards (for authentication)
- Form/multipart handling
- Async file operations
- Configuration via environment

### Storage Abstraction
The storage layer is designed with a trait-based approach to easily swap between filesystem and S3 backends:

```rust
trait Storage {
    async fn store(&self, tag: &str, path: &str, data: &[u8]) -> Result<()>;
    async fn retrieve(&self, tag: &str, path: &str) -> Result<Vec<u8>>;
    async fn delete_tag(&self, tag: &str) -> Result<()>;
    async fn list_tags(&self) -> Result<Vec<String>>;
}
```

## Roadmap

- [ ] Environment-based configuration
- [ ] API token authentication
- [ ] Single file upload
- [ ] Versioned asset serving (`/<tag>/<path>`)
- [ ] ZIP archive upload
- [ ] List tags endpoint
- [ ] Health check endpoint
- [ ] Delete tag endpoint
- [ ] S3 storage backend
- [ ] Usage statistics (time-series DB)
- [ ] Auto-cleanup of unused tags

## License

MIT

---

*This project is built for learning Rust. Contributions and feedback are welcome!*
