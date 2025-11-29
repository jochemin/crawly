# Backend - Bitcoin Network Crawler

Rust-based Bitcoin P2P network crawler and REST API server.

## 🏗️ Architecture

- **P2P Module** (`src/p2p/`): Handles Bitcoin protocol communication
- **Database Module** (`src/database/`): PostgreSQL interactions
- **Common Module** (`src/common/`): Shared utilities (GeoIP, address parsing)
- **Main** (`src/main.rs`): API server and crawler orchestration

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- GeoIP databases (GeoLite2-City, GeoLite2-ASN)

### Setup

1. **Configure environment**:
```bash
cp .env.example .env
# Edit .env with your database credentials
```

2. **Run migrations**:
```bash
sqlx migrate run
```

3. **Build and run**:
```bash
cargo build --release
cargo run --release
```

The API will be available at `http://localhost:3000`

## 🔧 Configuration

Environment variables in `.env`:

```env
DATABASE_URL=postgresql://user:YOUR_PASSWORD_HERE@localhost:5432/bnetwork
GEOIP_DB_PATH=/path/to/geoip/databases
```

## 📊 Database Schema

### `bnetwork` table
Stores individual node information:
- Address, port, network type
- Software version, services
- Geolocation data
- Last seen timestamp
- Incoming connection status

### `hourly_stats` table
Aggregated hourly statistics:
- Total nodes, incoming nodes
- Network type breakdown (IPv4, IPv6, Tor, I2P)
- Top 10 software versions (JSON)

## 🔌 API Endpoints

### Statistics
- `GET /api/stats` - Current network statistics
- `GET /api/stats/history?range=48h` - Historical data (24h, 48h, 7d, 30d)

### Nodes
- `GET /api/nodes` - List all nodes (paginated)
- `GET /api/nodes/search?q=<query>` - Search by address/software
- `GET /api/node/<address>` - Get node details

### Protocol Stats
- `GET /api/stats/protocol` - Breakdown by network type

## 🕷️ Crawler Behavior

1. **Discovery**: Connects to seed nodes and requests peer addresses
2. **Filtering**: Only accepts nodes seen in last 48 hours (BIP 155)
3. **Validation**: Handles clock skew (±10 minutes) and rejects far-future timestamps
4. **Storage**: Batch inserts discovered nodes to database
5. **Monitoring**: Periodically scans nodes for incoming connection capability

## 🔐 Security Features

- **Attack Detection**: Rejects timestamps >1 hour in future
- **Rate Limiting**: Batch processing with deadlock retry logic
- **Input Validation**: Strict address parsing and validation

## 📈 Performance

- Batch database operations (50 nodes/chunk)
- Background async processing for AddrV2 messages
- Connection pooling with SQLx
- Efficient GeoIP lookups with arc-swap

## 🛠️ Development

### Run tests
```bash
cargo test
```

### Check code
```bash
cargo check
cargo clippy
```

### Format code
```bash
cargo fmt
```

## 📝 Logging

Logs are controlled via `RUST_LOG` environment variable:
```bash
RUST_LOG=info cargo run
RUST_LOG=debug,sqlx=warn cargo run  # Debug but quiet SQLx
```

## 🔄 Scheduled Tasks

The crawler runs hourly snapshots via `tokio-cron-scheduler`:
- Aggregates current network state
- Stores in `hourly_stats` table
- Calculates top 10 software versions

## 🌐 Supported Networks

- IPv4
- IPv6
- Tor v2 (legacy)
- Tor v3
- I2P
- CJDNS
- Yggdrasil

## 📦 Dependencies

Key dependencies:
- `bitcoin` - Bitcoin protocol implementation
- `sqlx` - Async PostgreSQL driver
- `axum` - Web framework
- `tokio` - Async runtime
- `maxminddb` - GeoIP lookups

See `Cargo.toml` for complete list.
