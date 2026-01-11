/**
 * Pure JS RGB565 conversion for LMS plugin binary
 *
 * Uses jpeg-js (pure JS) to decode images, avoiding native sharp module
 * which doesn't bundle correctly with pkg.
 */

const jpeg = require('jpeg-js');

/**
 * Convert JPEG buffer to RGB565 format for ESP32 displays
 * @param {Buffer} jpegBuffer - JPEG image buffer
 * @param {number} expectedWidth - Expected width (for validation)
 * @param {number} expectedHeight - Expected height (for validation)
 * @returns {{ data: Buffer, width: number, height: number }}
 */
function convertToRgb565(jpegBuffer, expectedWidth = 360, expectedHeight = 360) {
  // Decode JPEG to raw RGBA pixels
  const decoded = jpeg.decode(jpegBuffer, { useTArray: true });
  const { width, height, data } = decoded;

  // data is RGBA (4 bytes per pixel)
  const rgb565 = Buffer.alloc(width * height * 2);

  for (let i = 0; i < width * height; i++) {
    const srcIdx = i * 4;  // RGBA
    const dstIdx = i * 2;  // RGB565

    // Extract RGB (ignore alpha)
    const r = data[srcIdx] >> 3;      // 5 bits
    const g = data[srcIdx + 1] >> 2;  // 6 bits
    const b = data[srcIdx + 2] >> 3;  // 5 bits

    // Pack into RGB565 (little-endian for ESP32)
    const pixel = (r << 11) | (g << 5) | b;
    rgb565[dstIdx] = pixel & 0xFF;
    rgb565[dstIdx + 1] = (pixel >> 8) & 0xFF;
  }

  return { data: rgb565, width, height };
}

module.exports = { convertToRgb565 };
