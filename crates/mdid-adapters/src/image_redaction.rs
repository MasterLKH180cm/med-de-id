use mdid_domain::ImageRedactionRegion;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ImageRedactionError {
    #[error("rgb image buffer length does not match dimensions")]
    MalformedRgbBuffer,
    #[error("PPM P6 image bytes are malformed or unsupported")]
    MalformedPpmP6,
    #[error("image redaction region is outside image bounds")]
    RegionOutOfBounds,
}

pub fn redact_ppm_p6_bytes(
    bytes: &[u8],
    regions: &[ImageRedactionRegion],
) -> Result<Vec<u8>, ImageRedactionError> {
    let (width, height, payload_offset) = parse_ppm_p6_header(bytes)?;
    let mut pixels = bytes[payload_offset..].to_vec();
    let expected_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or(ImageRedactionError::MalformedPpmP6)?;
    if pixels.len() != expected_len {
        return Err(ImageRedactionError::MalformedPpmP6);
    }

    redact_rgb_regions(&mut pixels, width, height, regions, [0, 0, 0])?;

    let mut output = bytes[..payload_offset].to_vec();
    output.extend_from_slice(&pixels);
    Ok(output)
}

fn parse_ppm_p6_header(bytes: &[u8]) -> Result<(u32, u32, usize), ImageRedactionError> {
    let mut offset = 0;
    let magic = next_ppm_token(bytes, &mut offset)?;
    if magic != b"P6" {
        return Err(ImageRedactionError::MalformedPpmP6);
    }
    let width = parse_ppm_u32(next_ppm_token(bytes, &mut offset)?)?;
    let height = parse_ppm_u32(next_ppm_token(bytes, &mut offset)?)?;
    let maxval = parse_ppm_u32(next_ppm_token(bytes, &mut offset)?)?;
    if maxval != 255 {
        return Err(ImageRedactionError::MalformedPpmP6);
    }
    if offset >= bytes.len() || !bytes[offset].is_ascii_whitespace() {
        return Err(ImageRedactionError::MalformedPpmP6);
    }
    offset += 1;
    Ok((width, height, offset))
}

fn next_ppm_token<'a>(bytes: &'a [u8], offset: &mut usize) -> Result<&'a [u8], ImageRedactionError> {
    while *offset < bytes.len() && bytes[*offset].is_ascii_whitespace() {
        *offset += 1;
    }
    let start = *offset;
    while *offset < bytes.len() && !bytes[*offset].is_ascii_whitespace() {
        *offset += 1;
    }
    if start == *offset {
        return Err(ImageRedactionError::MalformedPpmP6);
    }
    Ok(&bytes[start..*offset])
}

fn parse_ppm_u32(token: &[u8]) -> Result<u32, ImageRedactionError> {
    let text = std::str::from_utf8(token).map_err(|_| ImageRedactionError::MalformedPpmP6)?;
    text.parse::<u32>()
        .map_err(|_| ImageRedactionError::MalformedPpmP6)
}

pub fn redact_rgb_regions(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    regions: &[ImageRedactionRegion],
    fill: [u8; 3],
) -> Result<(), ImageRedactionError> {
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or(ImageRedactionError::MalformedRgbBuffer)?;
    let expected_len = pixel_count
        .checked_mul(3)
        .ok_or(ImageRedactionError::MalformedRgbBuffer)?;

    if pixels.len() != expected_len {
        return Err(ImageRedactionError::MalformedRgbBuffer);
    }

    for region in regions {
        let right = region
            .x()
            .checked_add(region.width())
            .ok_or(ImageRedactionError::RegionOutOfBounds)?;
        let bottom = region
            .y()
            .checked_add(region.height())
            .ok_or(ImageRedactionError::RegionOutOfBounds)?;
        if right > width || bottom > height {
            return Err(ImageRedactionError::RegionOutOfBounds);
        }
    }

    for region in regions {
        for y in region.y()..region.y() + region.height() {
            for x in region.x()..region.x() + region.width() {
                let offset = ((y as usize * width as usize) + x as usize) * 3;
                pixels[offset..offset + 3].copy_from_slice(&fill);
            }
        }
    }

    Ok(())
}
