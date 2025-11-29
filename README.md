# Crawly - Bitcoin Network Crawler

A comprehensive, high-performance Bitcoin P2P network crawler and analyzer designed to discover, monitor, and visualize Bitcoin nodes across multiple networks (IPv4, IPv6, Tor, I2P, CJDNS, Yggdrasil).

## 🚀 Features

- **Advanced P2P Crawling**: Implements AddrV2 (BIP 155) for modern node discovery.
- **Multi-Network Support**: Native support for Clearnet, Tor (v2/v3), I2P, CJDNS, and Yggdrasil.
- **Real-time Analytics**: Live monitoring of node availability, versions, and network health.
- **Historical Data**: Automated hourly snapshots for trend analysis and long-term statistics.
- **Geo-Location**: Integrated MaxMind GeoIP support for physical node mapping.
- **Cross-Platform**: Rust-based high-performance backend and React Native mobile application.

## � Prerequisites

Before building the project, ensure you have the following software installed:

### Core Dependencies
- **Rust**: Latest stable version (via `rustup`).
- **Node.js & npm**: Required for the mobile application (LTS version recommended).
- **PostgreSQL**: Database for storing node data and statistics.
- **Java (JDK 17+)**: Required for building the Android app.

### Network Proxies
To crawl privacy networks, you must have the respective proxies running:

- **Tor**: Required for `.onion` addresses.
  - Default configuration (SOCKS5 on port 9050).
- **i2pd**: Required for `.i2p` addresses.
  - **IMPORTANT**: This project is configured to use **port 4446** for the I2P SOCKS proxy.
  - *Note*: The default i2pd port is usually 4447. You must configure your `i2pd.conf` or tunnel settings to listen on port 4446, or update the backend code.

### GeoIP Data
For geolocation enrichment to work, the application expects the MaxMind databases to be present:
1.  Create a directory named `database` in the same location as the executable (or project root).
2.  Download **GeoLite2-City.mmdb** and **GeoLite2-ASN.mmdb**.
3.  Place both files inside the `database` directory.

## 🛠️ Installation & Build

### 1. Backend (Rust)

```bash
# Clone the repository
git clone https://github.com/yourusername/crawly.git
cd crawly/backend

# Configure Environment
cp .env.example .env
# Edit .env with your database credentials and API keys

# Install SQLx CLI for migrations
cargo install sqlx-cli

# Run Database Migrations
sqlx migrate run

# Build and Run
cargo build --release
cargo run --release
```

### 2. Mobile App (React Native)

```bash
cd crawly/mobile

# Install Dependencies
npm install

# Start Metro Bundler
npm start

# Run on Android
npm run android
```

## ⚙️ Configuration

The backend is configured via the `.env` file. Key variables include:

- `DATABASE_URL`: PostgreSQL connection string.
- `GEOIP_DB_PATH`: Path to MaxMind GeoIP databases (optional).
- `I2P_PROXY_ADDRESS`: (Optional) If you modified the code to support env vars, otherwise defaults to `127.0.0.1:4446`.

## 📊 Architecture

The project consists of two main components:

1.  **Crawler Backend (Rust)**: Handles P2P handshakes, database operations, and exposes a REST API.
2.  **Mobile Client (React Native)**: Consumes the API to provide a user-friendly dashboard.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1.  Fork the project
2.  Create your feature branch (`git checkout -b feature/AmazingFeature`)
3.  Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4.  Push to the branch (`git push origin feature/AmazingFeature`)
5.  Open a Pull Request

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.

### Attribution
Any use of this software must include a clear acknowledgement to "Crawly" and display the official image of this software.
