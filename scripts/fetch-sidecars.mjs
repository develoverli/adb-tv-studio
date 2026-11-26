#!/usr/bin/env node
// Downloads the third-party files the app bundles, each verified against a pinned SHA-256:
//   - adb from the official Android platform-tools, placed where Tauri expects a sidecar:
//       src-tauri/binaries/adb-<target-triple>[.exe]
//     On Windows the two DLLs adb needs are copied next to it as well.
//   - scrcpy-server (pushed to the TV for live video and fast input), bundled as a resource:
//       src-tauri/binaries/scrcpy-server.jar
//     It runs on the TV, so the same file serves every target.
//
// Usage:
//   node scripts/fetch-sidecars.mjs                 # host target (from `rustc -vV`)
//   node scripts/fetch-sidecars.mjs --target <triple>
//
// Extraction uses the system `tar` on Windows (bsdtar reads .zip) and `unzip`
// elsewhere, so the script has no npm dependencies.

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** Bump this together with the checksums below. */
const PLATFORM_TOOLS_VERSION = "37.0.3";
const DOWNLOAD_BASE_URL = "https://dl.google.com/android/repository";

/** SHA-256 of each platform-tools archive. Google publishes SHA-1 in repository2-3.xml; verify both when bumping. */
const ARCHIVES = {
  win: {
    sha256: "89d108360a3bf2924d3bea837239edaccc22fd6ff5befd3787c4348a9fe4f09d",
    files: ["adb.exe", "AdbWinApi.dll", "AdbWinUsbApi.dll"],
  },
  darwin: {
    // Universal (arm64 + x86_64) binary, so both macOS triples share it.
    sha256: "df21339b9371fef9b19264404338dc686f59a88a1dce634b963da344909001d6",
    files: ["adb"],
  },
  linux: {
    sha256: "214548146cadb2ca3508143d8a1d4be4c59de5bb837253e3ede89ed74cf450d1",
    files: ["adb"],
  },
};

/**
 * scrcpy-server release. The app starts it with this exact version string and the server refuses
 * any other, so bump SCRCPY_VERSION in src-tauri/src/adb/scrcpy.rs together with these two.
 */
const SCRCPY_VERSION = "5.0.1";
const SCRCPY_SHA256 = "764eb6f79811d5211fe9df341120882ba9994c7a61b897d7bf3fb662e53bc536";

const SUPPORTED_TARGETS = {
  "x86_64-pc-windows-msvc": "win",
  "aarch64-pc-windows-msvc": "win",
  "x86_64-apple-darwin": "darwin",
  "aarch64-apple-darwin": "darwin",
  "x86_64-unknown-linux-gnu": "linux",
};

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const binariesDir = join(repoRoot, "src-tauri", "binaries");
const cacheDir = join(repoRoot, ".cache", "platform-tools");
const scrcpyCacheDir = join(repoRoot, ".cache", "scrcpy");

function fail(message) {
  console.error(`fetch-sidecars: ${message}`);
  process.exit(1);
}

function resolveTarget() {
  const flagIndex = process.argv.indexOf("--target");
  if (flagIndex !== -1) {
    const value = process.argv[flagIndex + 1];
    if (!value) fail("--target needs a value");
    return value;
  }
  const rustcInfo = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  const hostLine = rustcInfo.split("\n").find((line) => line.startsWith("host:"));
  if (!hostLine) fail("could not read host triple from `rustc -vV`");
  return hostLine.replace("host:", "").trim();
}

function sha256(buffer) {
  return createHash("sha256").update(buffer).digest("hex");
}

async function loadArchive(os) {
  const { sha256: expected } = ARCHIVES[os];
  const fileName = `platform-tools_r${PLATFORM_TOOLS_VERSION}-${os}.zip`;
  const cachedPath = join(cacheDir, fileName);

  if (existsSync(cachedPath) && sha256(readFileSync(cachedPath)) === expected) {
    console.log(`fetch-sidecars: using cached ${fileName}`);
    return cachedPath;
  }

  const url = `${DOWNLOAD_BASE_URL}/${fileName}`;
  console.log(`fetch-sidecars: downloading ${url}`);
  const response = await fetch(url);
  if (!response.ok) fail(`download failed: HTTP ${response.status}`);
  const buffer = Buffer.from(await response.arrayBuffer());

  const actual = sha256(buffer);
  if (actual !== expected) fail(`checksum mismatch for ${fileName}\n  expected ${expected}\n  actual   ${actual}`);

  mkdirSync(cacheDir, { recursive: true });
  writeFileSync(cachedPath, buffer);
  return cachedPath;
}

function extract(archivePath, destination) {
  if (process.platform === "win32") {
    // Use the Windows-bundled bsdtar explicitly: Git Bash puts GNU tar first in PATH,
    // and GNU tar neither reads .zip nor accepts drive-letter paths ("D:" = remote host).
    const windowsTar = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
    execFileSync(windowsTar, ["-xf", archivePath, "-C", destination], { stdio: "inherit" });
  } else {
    execFileSync("unzip", ["-q", "-o", archivePath, "-d", destination], { stdio: "inherit" });
  }
}

async function downloadVerified(url, expected, label) {
  console.log(`fetch-sidecars: downloading ${url}`);
  const response = await fetch(url);
  if (!response.ok) fail(`download failed: HTTP ${response.status}`);
  const buffer = Buffer.from(await response.arrayBuffer());
  const actual = sha256(buffer);
  if (actual !== expected) fail(`checksum mismatch for ${label}
  expected ${expected}
  actual   ${actual}`);
  return buffer;
}

async function fetchScrcpyServer() {
  const fileName = `scrcpy-server-v${SCRCPY_VERSION}`;
  const cachedPath = join(scrcpyCacheDir, fileName);
  let buffer;
  if (existsSync(cachedPath) && sha256(readFileSync(cachedPath)) === SCRCPY_SHA256) {
    console.log(`fetch-sidecars: using cached ${fileName}`);
    buffer = readFileSync(cachedPath);
  } else {
    const url = `https://github.com/Genymobile/scrcpy/releases/download/v${SCRCPY_VERSION}/${fileName}`;
    buffer = await downloadVerified(url, SCRCPY_SHA256, fileName);
    mkdirSync(scrcpyCacheDir, { recursive: true });
    writeFileSync(cachedPath, buffer);
  }
  mkdirSync(binariesDir, { recursive: true });
  const destination = join(binariesDir, "scrcpy-server.jar");
  writeFileSync(destination, buffer);
  console.log(`fetch-sidecars: wrote ${destination}`);
}

async function main() {
  const target = resolveTarget();
  const os = SUPPORTED_TARGETS[target];
  if (!os) fail(`unsupported target "${target}". Supported: ${Object.keys(SUPPORTED_TARGETS).join(", ")}`);

  const archivePath = await loadArchive(os);
  const workDir = mkdtempSync(join(tmpdir(), "adb-tv-studio-"));

  try {
    extract(archivePath, workDir);
    mkdirSync(binariesDir, { recursive: true });

    for (const file of ARCHIVES[os].files) {
      const source = join(workDir, "platform-tools", file);
      if (!existsSync(source)) fail(`${file} not found in archive`);

      const isAdb = file === "adb" || file === "adb.exe";
      const extension = file.endsWith(".exe") ? ".exe" : "";
      const destination = isAdb ? join(binariesDir, `adb-${target}${extension}`) : join(binariesDir, file);

      copyFileSync(source, destination);
      if (isAdb && os !== "win") chmodSync(destination, 0o755);
      console.log(`fetch-sidecars: wrote ${destination}`);
    }
  } finally {
    rmSync(workDir, { recursive: true, force: true });
  }

  await fetchScrcpyServer();
}

await main();
