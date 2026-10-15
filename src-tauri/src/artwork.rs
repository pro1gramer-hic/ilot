//! Cover art of a media session, shrunk to a small JPEG and handed to the interface as
//! a `data:` URL (the island's CSP allows `img-src data:` and nothing remote).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use windows::Foundation::{PropertyType, PropertyValue};
use windows::Graphics::Imaging::{
    BitmapAlphaMode, BitmapDecoder, BitmapEncoder, BitmapInterpolationMode, BitmapPixelFormat,
    BitmapPropertySet, BitmapTransform, BitmapTypedValue, ColorManagementMode, ExifOrientationMode,
};
use windows::core::HSTRING;
use windows::Storage::Streams::{DataReader, IRandomAccessStreamReference, InMemoryRandomAccessStream};

/// Longest side of the artwork sent to the interface, in pixels. The largest place it
/// shows is the mini-player (340 px wide, 16:9), so 320 px keeps it sharp enough at a
/// small size.
pub const ARTWORK_MAX_SIDE: u32 = 320;
/// JPEG quality: a 320 px cover then weighs about 20 KB.
const JPEG_QUALITY: f32 = 0.8;

/// Size that fits within `max` × `max` while keeping the aspect ratio; never enlarges.
pub fn fit_within(width: u32, height: u32, max: u32) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (0, 0);
    }
    if width <= max && height <= max {
        return (width, height);
    }
    let scale = f64::from(max) / f64::from(width.max(height));
    let fit = |side: u32| ((f64::from(side) * scale).round() as u32).max(1);
    (fit(width), fit(height))
}

pub fn data_url(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", STANDARD.encode(bytes))
}

/// Reads the session thumbnail and re-encodes it as a small JPEG data URL.
pub fn read(thumbnail: &IRandomAccessStreamReference) -> windows::core::Result<String> {
    let stream = thumbnail.OpenReadAsync()?.join()?;
    let decoder = BitmapDecoder::CreateAsync(&stream)?.join()?;
    let (width, height) =
        fit_within(decoder.OrientedPixelWidth()?, decoder.OrientedPixelHeight()?, ARTWORK_MAX_SIDE);
    let transform = BitmapTransform::new()?;
    transform.SetScaledWidth(width)?;
    transform.SetScaledHeight(height)?;
    transform.SetInterpolationMode(BitmapInterpolationMode::Fant)?;
    let bitmap = decoder
        .GetSoftwareBitmapTransformedAsync(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Ignore,
            &transform,
            ExifOrientationMode::RespectExifOrientation,
            ColorManagementMode::DoNotColorManage,
        )?
        .join()?;
    let output = InMemoryRandomAccessStream::new()?;
    let options = BitmapPropertySet::new()?;
    let quality = BitmapTypedValue::Create(&PropertyValue::CreateSingle(JPEG_QUALITY)?, PropertyType::Single)?;
    options.Insert(&HSTRING::from("ImageQuality"), &quality)?;
    let encoder =
        BitmapEncoder::CreateWithEncodingOptionsAsync(BitmapEncoder::JpegEncoderId()?, &output, &options)?.join()?;
    encoder.SetSoftwareBitmap(&bitmap)?;
    encoder.FlushAsync()?.join()?;
    let size = u32::try_from(output.Size()?).unwrap_or(0);
    let reader = DataReader::CreateDataReader(&output.GetInputStreamAt(0)?)?;
    reader.LoadAsync(size)?.join()?;
    let mut bytes = vec![0; size as usize];
    reader.ReadBytes(&mut bytes)?;
    Ok(data_url("image/jpeg", &bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_images_keep_their_size() {
        assert_eq!(fit_within(300, 300, 320), (300, 300));
        assert_eq!(fit_within(320, 180, 320), (320, 180));
    }

    #[test]
    fn large_images_shrink_and_keep_their_ratio() {
        assert_eq!(fit_within(640, 640, 320), (320, 320));
        assert_eq!(fit_within(1280, 720, 320), (320, 180));
        assert_eq!(fit_within(480, 640, 320), (240, 320));
    }

    #[test]
    fn extreme_ratios_keep_at_least_one_pixel() {
        assert_eq!(fit_within(10_000, 2, 320), (320, 1));
        assert_eq!(fit_within(0, 50, 320), (0, 0));
    }

    #[test]
    fn data_url_is_base64() {
        assert_eq!(data_url("image/jpeg", b"\xff\xd8\xff"), "data:image/jpeg;base64,/9j/");
        assert_eq!(data_url("image/png", b""), "data:image/png;base64,");
    }
}
