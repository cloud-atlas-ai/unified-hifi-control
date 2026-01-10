/**
 * RGB565 conversion regression tests
 *
 * Bug: PNG images with alpha channels (RGBA, 4 bytes/pixel) caused channel
 * misalignment because the conversion loop assumed 3 bytes/pixel (RGB).
 *
 * Fix: Add .removeAlpha() to the sharp pipeline before .raw()
 *
 * Issue: https://github.com/open-horizon-labs/unified-hifi-control/issues/35
 * Forum: https://forums.lyrion.org/forum/user-forums/3rd-party-hardware/1804977-roon-knob-includes-lms-support?p=1805839#post1805839
 */

const sharp = require('sharp');

function convertToRgb565(rgb888, width, height) {
  const rgb565 = Buffer.alloc(width * height * 2);
  for (let i = 0; i < rgb888.length; i += 3) {
    const r = rgb888[i] >> 3;
    const g = rgb888[i + 1] >> 2;
    const b = rgb888[i + 2] >> 3;
    const rgb565Pixel = (r << 11) | (g << 5) | b;
    const pixelIndex = (i / 3) * 2;
    rgb565[pixelIndex] = rgb565Pixel & 0xff;
    rgb565[pixelIndex + 1] = (rgb565Pixel >> 8) & 0xff;
  }
  return rgb565;
}

describe('RGB565 conversion - alpha channel handling', () => {
  const targetWidth = 10;
  const targetHeight = 10;

  test('CRITICAL: RGBA PNG produces correct buffer size with removeAlpha()', async () => {
    const rgbaPng = await sharp({
      create: { width: 20, height: 20, channels: 4, background: { r: 255, g: 0, b: 0, alpha: 0.5 } },
    }).png().toBuffer();

    const result = await sharp(rgbaPng)
      .resize(targetWidth, targetHeight, { fit: 'cover' })
      .removeAlpha()
      .raw()
      .toBuffer({ resolveWithObject: true });

    expect(result.data.length).toBe(targetWidth * targetHeight * 3);
    expect(result.info.channels).toBe(3);
  });

  // Documents the pre-fix behavior - RGBA images without removeAlpha() have 4 channels
  test('Without removeAlpha(), RGBA PNG has wrong buffer size (documents bug)', async () => {
    const rgbaPng = await sharp({
      create: { width: 20, height: 20, channels: 4, background: { r: 255, g: 0, b: 0, alpha: 0.5 } },
    }).png().toBuffer();

    const result = await sharp(rgbaPng)
      .resize(targetWidth, targetHeight, { fit: 'cover' })
      .raw()
      .toBuffer({ resolveWithObject: true });

    expect(result.data.length).toBe(targetWidth * targetHeight * 4);
    expect(result.info.channels).toBe(4);
  });

  test('RGB JPEG works with removeAlpha()', async () => {
    const rgbJpeg = await sharp({
      create: { width: 20, height: 20, channels: 3, background: { r: 0, g: 255, b: 0 } },
    }).jpeg().toBuffer();

    const result = await sharp(rgbJpeg)
      .resize(targetWidth, targetHeight, { fit: 'cover' })
      .removeAlpha()
      .raw()
      .toBuffer({ resolveWithObject: true });

    expect(result.data.length).toBe(targetWidth * targetHeight * 3);
  });

  test('RGB565 output size is correct', async () => {
    const rgbaPng = await sharp({
      create: { width: 20, height: 20, channels: 4, background: { r: 128, g: 64, b: 192, alpha: 1 } },
    }).png().toBuffer();

    const rgb888Result = await sharp(rgbaPng)
      .resize(targetWidth, targetHeight, { fit: 'cover' })
      .removeAlpha()
      .raw()
      .toBuffer({ resolveWithObject: true });

    const rgb565 = convertToRgb565(rgb888Result.data, targetWidth, targetHeight);
    expect(rgb565.length).toBe(targetWidth * targetHeight * 2);
  });
});
