# Code Analysis & Production-Ready Recommendations

## 🔴 Critical Issues (Fix First)

### 1. Excessive `.unwrap()` — Will Panic in Production

Multiple places use `.unwrap()` which will crash the server:

```rust
// upload.rs:29, 31, 42, 43, 80, 81
file_name_buf.clone().into_os_string().into_string().unwrap();
file_real_size(form.file.path().unwrap()).unwrap();
std::fs::create_dir_all(prefix).unwrap();

// tags.rs:56
std::fs::remove_dir(full_path).unwrap();
```

**Fix:** Replace with proper error handling:
```rust
std::fs::create_dir_all(prefix).map_err(|_| Status::InternalServerError)?;
```

**Status:** ✅ Fixed

---

### 2. ZIP Security — Zip Slip Vulnerability

`zip.extract()` can write files outside target directory if ZIP contains `../` paths.

**Fix:** Use manual extraction with path validation:
```rust
for i in 0..zip.len() {
    let mut entry = zip.by_index(i)?;
    let outpath = full_path.join(entry.name());
    
    // Security: validate extracted path
    if !outpath.starts_with(&full_path) {
        return Err(Status::BadRequest);
    }
    // ... extract
}
```

**Status:** ✅ Fixed — `zip` 2.4.2 `extract()` already sanitizes paths (`enclosed_name`); errors now map to 400, covered by `test_upload_zip_slip`

---

### 3. `remove_dir` Only Works on Empty Directories

```rust
// tags.rs:56 — will fail if tag folder has files
std::fs::remove_dir(full_path).unwrap();
```

**Fix:** Use `remove_dir_all` (with caution):
```rust
std::fs::remove_dir_all(&full_path).map_err(|_| Status::InternalServerError)?;
```

**Status:** ✅ Fixed

---

## 🟠 Code Organization (Refactoring)

### 4. Extract Common Path Validation Logic

This pattern is repeated 4 times:
```rust
let base_path = Path::new(&config.storage_path).canonicalize().map_err(|_| Status::InternalServerError)?;
let full_path = base_path.join(&tag);
if !full_path.starts_with(&base_path) {
    return Err(Status::BadRequest);
}
```

**Create a storage module:**
```rust
// src/storage.rs
use std::path::{Path, PathBuf};
use rocket::http::Status;

pub struct StoragePath {
    base: PathBuf,
}

impl StoragePath {
    pub fn new(storage_path: &str) -> Result<Self, Status> {
        let base = Path::new(storage_path)
            .canonicalize()
            .map_err(|_| Status::InternalServerError)?;
        Ok(Self { base })
    }

    pub fn resolve(&self, segments: &[&str]) -> Result<PathBuf, Status> {
        let mut path = self.base.clone();
        for seg in segments {
            path = path.join(seg);
        }
        if !path.starts_with(&self.base) {
            return Err(Status::BadRequest);
        }
        Ok(path)
    }

    pub fn base(&self) -> &Path {
        &self.base
    }
}
```

**Status:** ✅ Fixed — `StoragePath` in `src/storage.rs`

---

### 5. Create Custom Error Type

Instead of returning bare `Status`, create an error type for better messages:

```rust
// src/error.rs
use rocket::http::Status;
use rocket::response::Responder;
use rocket::serde::json::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Unauthorized,
    InternalError(String),
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let (status, error) = match self {
            AppError::NotFound(msg) => (Status::NotFound, ApiError { code: "NOT_FOUND".into(), message: msg }),
            AppError::BadRequest(msg) => (Status::BadRequest, ApiError { code: "BAD_REQUEST".into(), message: msg }),
            AppError::Unauthorized => (Status::Unauthorized, ApiError { code: "UNAUTHORIZED".into(), message: "Invalid or missing token".into() }),
            AppError::InternalError(msg) => (Status::InternalServerError, ApiError { code: "INTERNAL_ERROR".into(), message: msg }),
        };
        (status, Json(error)).respond_to(req)
    }
}
```

**Status:** ⏳ Pending

---

## 🟡 Best Practices

### 6. Use Proper Logging Instead of `print!`

```toml
# Cargo.toml
log = "0.4"
```

```rust
// In code
log::info!("File uploaded: {}, type: {:?}, size: {}", full_url, mimetype, filesize);
```

**Status:** ✅ Fixed — `log` crate (printed by Rocket's logger)

---

### 7. Remove Debug `println!` in `assets.rs`

```rust
// assets.rs:14-15 — remove for production
println!("Full path: {}", full_path.display());
println!("Base path: {}", base_path.display());
```

**Status:** ✅ Fixed — `log` crate (printed by Rocket's logger)

---

### 8. Validate Tag Name Format

Currently accepts any string including `.`, `..`, special chars:

```rust
fn is_valid_tag(tag: &str) -> bool {
    !tag.is_empty() 
        && tag.len() <= 64 
        && tag.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !tag.starts_with('.')
}
```

**Status:** ✅ Fixed — `validate_tag` in `src/storage.rs`, used by every route taking a tag

---

### 9. Add Request ID for Tracing

Create a fairing that adds unique request ID for debugging. Useful for correlating logs in production.

**Status:** ⏳ Pending

---

## 🟢 Project Structure Suggestion

```
src/
├── lib.rs              # App setup
├── main.rs             # Entry point
├── config.rs           # Config (✓ good)
├── error.rs            # NEW: Custom error types
├── storage.rs          # NEW: Path utilities
├── guards/
│   ├── mod.rs
│   ├── auth.rs         # IsAuth guard
│   └── request_info.rs # RequestInfo guard
├── routes/
│   ├── mod.rs
│   ├── assets.rs       # GET assets
│   ├── upload.rs       # POST upload
│   ├── tags.rs         # Tags CRUD
│   └── health.rs       # Health check
```

---

## 📋 Priority Action List

| Priority | Task | Effort | Status |
|----------|------|--------|--------|
| 1 | Replace all `.unwrap()` with proper error handling | 30 min | ✅ Done |
| 2 | Fix ZIP slip vulnerability | 15 min | ✅ Done |
| 3 | Fix `remove_dir` → `remove_dir_all` | 5 min | ✅ Done |
| 4 | Remove debug `println!` | 5 min | ✅ Done |
| 5 | Extract `StoragePath` utility | 30 min | ✅ Done |
| 6 | Add tag validation | 15 min | ✅ Done |
| 7 | Create custom error type | 45 min | ⏳ Pending |
| 8 | Add proper logging | 20 min | ✅ Done |
