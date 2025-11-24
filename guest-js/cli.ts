import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { pipeline } from 'stream/promises';
import { createWriteStream } from 'fs';
import SevenZip from '7z-wasm';

const TARGET_DIR = path.join(process.cwd(), 'src-tauri', 'lib');
const TEMP_DIR = path.join(TARGET_DIR, 'temp');

const WRAPPER_BASE_URL = "https://github.com/nini22P/libmpv-wrapper/releases/latest/download";
const MPV_BASE_URL = "https://github.com/zhongfly/mpv-winbuild/releases/latest/download";

const COLORS = {
  Reset: "\x1b[0m",
  Red: "\x1b[31m",
  Green: "\x1b[32m",
  Yellow: "\x1b[33m",
  Cyan: "\x1b[36m",
  Gray: "\x1b[90m",
};

function log(message: string, color: string = COLORS.Reset) {
  console.log(`${color}${message}${COLORS.Reset}`);
}

function errorExit(message: string) {
  console.error(`${COLORS.Red}Error: ${message}${COLORS.Reset}`);
  process.exit(1);
}

function getSystemInfo() {
  const platform = os.platform();
  const arch = os.arch();
  let osName = '', archName = '', wrapperLibName = '';

  if (platform === 'win32') {
    osName = 'windows';
    wrapperLibName = 'libmpv-wrapper.dll';
  } else if (platform === 'darwin') {
    osName = 'macos';
    wrapperLibName = 'libmpv-wrapper.dylib';
  } else if (platform === 'linux') {
    osName = 'linux';
    wrapperLibName = 'libmpv-wrapper.so';
  } else {
    errorExit(`Unsupported platform: ${platform}`);
  }

  if (arch === 'x64') archName = 'x86_64';
  else if (arch === 'arm64') archName = 'aarch64';
  else errorExit(`Unsupported architecture: ${arch}`);

  return { platform, osName, archName, wrapperLibName };
}

async function downloadFile(url: string, destPath: string) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to fetch ${url}: ${res.statusText}`);
  if (!res.body) throw new Error(`Response body is empty for ${url}`);
  const fileStream = createWriteStream(destPath);
  await pipeline(res.body, fileStream);
}

async function extractArchive(archivePath: string, extractDir: string) {
  if (fs.existsSync(extractDir)) {
    fs.rmSync(extractDir, { recursive: true, force: true });
  }
  fs.mkdirSync(extractDir, { recursive: true });

  const archiveName = path.basename(archivePath);
  const archiveDir = path.dirname(archivePath);

  log(`  Extracting ${archiveName}...`, COLORS.Cyan);

  const sevenZip = await SevenZip({
    print: (s: string) => { },
    printErr: (s: string) => { }
  });

  const mountSrc = "/archive_source";
  const mountDest = "/archive_dest";

  sevenZip.FS.mkdir(mountSrc);
  sevenZip.FS.mkdir(mountDest);

  sevenZip.FS.mount(sevenZip.NODEFS, { root: archiveDir }, mountSrc);
  sevenZip.FS.mount(sevenZip.NODEFS, { root: extractDir }, mountDest);

  const vArchivePath = `${mountSrc}/${archiveName}`;
  const args = ["x", vArchivePath, `-o${mountDest}`, "-y"];

  try {
    sevenZip.callMain(args);
  } catch (err: any) {
    if (err && err.status === 0) {
      // success
    } else {
      throw new Error(`7z extraction failed: ${err}`);
    }
  } finally {
    try {
      sevenZip.FS.unmount(mountSrc);
      sevenZip.FS.unmount(mountDest);
    } catch (e) { }
  }
}

function findAndMove(searchDir: string, fileName: string, destDir: string) {
  let foundPath: string | null = null;

  function search(dir: string) {
    const files = fs.readdirSync(dir);
    for (const file of files) {
      const fullPath = path.join(dir, file);
      if (file.startsWith('.')) continue;
      try {
        const stat = fs.statSync(fullPath);
        if (stat.isDirectory()) {
          search(fullPath);
        } else if (file === fileName) {
          foundPath = fullPath;
          return;
        }
      } catch (e) { }
      if (foundPath) return;
    }
  }
  search(searchDir);

  if (foundPath) {
    const destPath = path.join(destDir, fileName);
    fs.renameSync(foundPath, destPath);
    log(`  -> ${fileName} downloaded successfully.`, COLORS.Green);
  } else {
    throw new Error(`${fileName} not found in extracted files.`);
  }
}

async function runSetup() {
  const { platform, osName, archName, wrapperLibName } = getSystemInfo();
  log(`Detected System: ${osName} (${archName})`, COLORS.Yellow);

  if (!fs.existsSync(TEMP_DIR)) fs.mkdirSync(TEMP_DIR, { recursive: true });
  if (!fs.existsSync(TARGET_DIR)) fs.mkdirSync(TARGET_DIR, { recursive: true });

  log("\n[1/2] Processing libmpv-wrapper...", COLORS.Cyan);
  try {
    const shaUrl = `${WRAPPER_BASE_URL}/sha256.txt`;
    const response = await fetch(shaUrl);
    const shaContent = await response.text();
    const searchKey = `libmpv-wrapper-${osName}-${archName}`;
    const line = shaContent.split('\n').find(l => l.includes(searchKey));
    if (!line) throw new Error(`Could not find '${searchKey}' in SHA256 file.`);

    const fileName = line.trim().split(/\s+/).pop()!;
    log(`  Found file: ${fileName}`, COLORS.Yellow);

    const zipPath = path.join(TEMP_DIR, fileName);
    log("  Downloading...");
    await downloadFile(`${WRAPPER_BASE_URL}/${fileName}`, zipPath);

    const extractDir = path.join(TEMP_DIR, 'wrapper_extract');
    await extractArchive(zipPath, extractDir);
    findAndMove(extractDir, wrapperLibName, TARGET_DIR);
  } catch (err) {
    errorExit(`Failed to process libmpv-wrapper: ${err}`);
  }

  if (platform === 'win32') {
    log("\n[2/2] Processing libmpv (Windows)...", COLORS.Cyan);
    try {
      const shaUrl = `${MPV_BASE_URL}/sha256.txt`;
      const response = await fetch(shaUrl);
      const shaContent = await response.text();
      const searchKey = `mpv-dev-lgpl-${archName}`;
      const line = shaContent.split('\n').find(l => l.includes(searchKey) && !l.includes('v3'));
      if (!line) throw new Error(`Could not find '${searchKey}' (non-v3) in SHA256 file.`);

      const fileName = line.trim().split(/\s+/).pop()!;
      log(`  Found file: ${fileName}`, COLORS.Yellow);

      const archivePath = path.join(TEMP_DIR, fileName);
      log("  Downloading (this file is large)...");
      await downloadFile(`${MPV_BASE_URL}/${fileName}`, archivePath);

      const extractDir = path.join(TEMP_DIR, 'libmpv_extract');
      await extractArchive(archivePath, extractDir);
      findAndMove(extractDir, 'libmpv-2.dll', TARGET_DIR);
    } catch (err) {
      errorExit(`Failed to process libmpv: ${err}`);
    }
  } else {
    log("\n[2/2] Skipping libmpv download (Non-Windows)", COLORS.Gray);
    log("NOTE: On macOS/Linux, ensure system libmpv is installed.", COLORS.Yellow);
  }

  log("\nCleaning up temporary files...", COLORS.Gray);
  try { fs.rmSync(TEMP_DIR, { recursive: true, force: true }); } catch (e) { }

  log("\nSUCCESS! Libraries are set up in src-tauri/lib", COLORS.Green);
}

const args = process.argv.slice(2);
if (args[0] === 'setup-lib') {
  runSetup();
} else {
  console.log("Usage: npx tauri-plugin-libmpv-api setup-lib");
  console.log("\nCommands:");
  console.log("  setup-lib   Download and configure libmpv libraries automatically.");
}