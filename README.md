# Tauri Plugin libmpv

A Tauri plugin for embedding the mpv player in your app via libmpv.

## Installation

### Prerequisites

- Setup libmpv development environment.
- Tauri v2.x
- Node.js 18+

### Install the Plugin

```bash
npm run tauri add libmpv
```

### Configure Window Transparency

For mpv to properly embed into your Tauri window, you need to configure transparency:

#### Set window transparency in `tauri.conf.json`

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

// Unlisten when no longer needed
unlisten()

// Load and play a file
await command('loadfile', ['/path/to/video.mp4'])

// Set property
await setProperty('volume', 75)

// Get property
const volume = await getProperty('volume', 'int64')
console.log('Current volume is:', volume)

// Destroy mpv when no longer needed
await destroy()
```

## Platform Support

- ✅ **Windows** - Fully tested and supported
- ⚠️ **Linux** - Not tested
- ⚠️ **macOS** - Not tested

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the LGPL-2.1 License - see the [LICENSE](LICENSE) file for details.

This project was initially developed using [`libmpv2-rs`](https://github.com/kohsine/libmpv2-rs) (LGPL-2.1). To address licensing compatibility issues with Tauri's static builds, we attempted to create our own bindings for `libmpv-sys`. However, during this process,we found that our architectural approach naturally converged with that of the libmpv2-rs source code, which we had partially consulted during initial development. Although we didn't use the code directly, we believe the right thing to do is to honor the original author's foundational work. Therefore, in the spirit of open source, we have decided to license this project under the LGPL-2.1.
