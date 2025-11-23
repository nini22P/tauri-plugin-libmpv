# Tauri Plugin libmpv

A Tauri plugin for embedding the mpv player in your app via libmpv.

## Prerequisites

Before installing the plugin, you need to gather the necessary dynamic libraries. This plugin requires **two** parts to work:

1. **The Wrapper Library**: `libmpv-wrapper` (Interface between plugin and libmpv).
2. **The Actual mpv Library**: `libmpv` (The video player core).

### Windows Setup

1. **Download the Wrapper:**
    * Go to [libmpv-wrapper Releases](https://github.com/nini22P/libmpv-wrapper/releases).
    * Download `libmpv-wrapper-windows-x86_64.zip`
    * Extract `libmpv-wrapper.dll`.
2. **Download libmpv:**
    * Go to [zhongfly's builds](https://github.com/zhongfly/mpv-winbuild/releases).
    * Download the latest `mpv-dev-....7z`.
    * Extract `libmpv-2.dll`.
3. **Project Setup:**
    * Create a folder named `lib` inside your `src-tauri` directory.
    * Copy both `libmpv-wrapper.dll` and `libmpv-2.dll` into `src-tauri/lib/`.

Or you can use the [setup script](./examples/react/setup-lib.ps1).

### Linux Setup (Debian/Ubuntu)

1. **Install System libmpv:**

    ```bash
    sudo apt install libmpv-dev mpv
    ```

2. **Download the Wrapper:**
    * Go to [libmpv-wrapper Releases](https://github.com/nini22P/libmpv-wrapper/releases).
    * Download `libmpv-wrapper-linux-x86_64.zip`
    * Extract `libmpv-wrapper.so`.
3. **Project Setup:**
    * Create a folder named `lib` inside your `src-tauri` directory.
    * Copy `libmpv-wrapper.so` into `src-tauri/lib/`.

## Installation

### Install the Plugin

```bash
npm run tauri add libmpv
```

### Configure Resources (Important)

You must configure Tauri to bundle the dynamic libraries (.dll or .so) with your application so they are available at runtime.

#### Modify `src-tauri/tauri.conf.json`

```json
{
  "bundle": {
    "resources": [
      "lib/**/*"
    ]
  }
}
```

### Configure Window Transparency

For mpv to properly embed into your Tauri window, you need to configure transparency:

#### Set window transparency in `src-tauri/tauri.conf.json`

```json
{
  "app": {
    "windows": [
      {
        "title": "Your App",
        "width": 1280,
        "height": 720,
        "transparent": true  // Add this line
      }
    ]
  }
}
```

#### Set web page background to transparent in your CSS

```css
/* In your main CSS file */
html,
body {
  background: transparent;
}
```

## Quick Start

```typescript
import {
  MpvObservableProperty,
  MpvConfig,
  init,
  observeProperties,
  command,
  setProperty,
  getProperty,
  destroy,
} from 'tauri-plugin-libmpv-api'

// Properties to observe
// Tip: The optional third element, 'none', signals to TypeScript that the property's value may be null 
// (e.g., when a file is not loaded), ensuring type safety in the callback function.
const OBSERVED_PROPERTIES = [
  ['pause', 'flag'],
  ['time-pos', 'double', 'none'],
  ['duration', 'double', 'none'],
  ['filename', 'string', 'none'],
] as const satisfies MpvObservableProperty[]

// mpv configuration
const mpvConfig: MpvConfig = {
  initialOptions: {
    'vo': 'gpu-next',
    'hwdec': 'auto-safe',
    'keep-open': 'yes',
    'force-window': 'yes',
  },
  observedProperties: OBSERVED_PROPERTIES,
}

// Initialize mpv
try {
  await init(mpvConfig)
  console.log('mpv initialization completed successfully!')
} catch (error) {
  console.error('mpv initialization failed:', error)
}

// Observe properties
const unlisten = await observeProperties(
  OBSERVED_PROPERTIES,
  ({ name, data }) => {
    switch (name) {
      case 'pause':
        // data type: boolean
        console.log('Playback paused state:', data)
        break
      case 'time-pos':
        // data type: number | null
        console.log('Current time position:', data)
        break
      case 'duration':
        // data type: number | null
        console.log('Duration:', data)
        break
      case 'filename':
        // data type: string | null
        console.log('Current playing file:', data)
        break
    }
  })

// Load and play a file
await command('loadfile', ['/path/to/video.mp4'])

// Set property
await setProperty('volume', 75)

// Get property
const volume = await getProperty('volume', 'int64')
console.log('Current volume is:', volume)

// Clean up when done
// unlisten()
// await destroy()
```

## Platform Support

| Platform | Status | Notes |
| :--- | :---: | :--- |
| **Windows** | ✅ | Fully tested. Requires `libmpv-2.dll` and `libmpv-wrapper.dll`. |
| **Linux** | ⚠️ | Experimental. Window embedding is not working. Requires system `libmpv` and `libmpv-wrapper.so`. |
| **macOS** | ⚠️ | Not tested. |

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MPL-2.0 License - see the [LICENSE](LICENSE) file for details.
