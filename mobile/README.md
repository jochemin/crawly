# Mobile App - Crawly

React Native mobile application for monitoring the Bitcoin network.

## 📱 Features

- Real-time network statistics
- Node search and details
- Historical charts (24h trends)
- Top 10 Bitcoin clients
- Protocol distribution breakdown

## 🚀 Quick Start

### Prerequisites

- Node.js 16+
- npm or yarn
- Android Studio (for Android)
- Xcode (for iOS, macOS only)

### Installation

```bash
npm install
```

### Development

```bash
npm start
# Then press 'a' for Android or 'i' for iOS
```

### Build APK (Android)

```bash
cd android
./gradlew assembleRelease
```

The APK will be at: `android/app/build/outputs/apk/release/app-release.apk`

## 🔧 Configuration

Update API endpoint in `src/services/api.js`:

```javascript
const BASE_URL = 'http://YOUR_SERVER_IP:3000';
```

## 📂 Project Structure

```
mobile/
├── src/
│   ├── screens/        # Screen components
│   ├── components/     # Reusable components
│   └── services/       # API service
├── android/            # Android native code
├── assets/             # Images and icons
└── App.js              # Main app component
```

## 🎨 Screens

- **Dashboard**: Network overview and quick stats
- **Statistics**: Detailed charts and trends
- **Node Detail**: Individual node information

## 📊 API Integration

The app connects to the backend API:
- `/api/stats` - Current statistics
- `/api/stats/history` - Historical data
- `/api/node/<address>` - Node details

## 🔨 Build Configuration

### Android

App name and icon configured in:
- `android/app/src/main/res/values/strings.xml`
- `android/app/src/main/res/mipmap-*/`

### Package Name

`com.crawly.mobile` (configured in `android/app/build.gradle`)

## 🐛 Troubleshooting

### Metro bundler issues
```bash
npm start -- --reset-cache
```

### Android build fails
```bash
cd android
./gradlew clean
./gradlew assembleRelease
```

### Cannot connect to API
- Check BASE_URL in `src/services/api.js`
- Ensure backend is running
- Check firewall/network settings

## 📦 Dependencies

- `react-native` - Core framework
- `@react-navigation/native` - Navigation
- `react-native-chart-kit` - Charts
- `react-native-safe-area-context` - Safe areas

See `package.json` for complete list.

## 🎯 Future Features

- [ ] Push notifications for network events
- [ ] Offline mode with cached data
- [ ] Dark mode
- [ ] Custom alerts
- [ ] Export data functionality
