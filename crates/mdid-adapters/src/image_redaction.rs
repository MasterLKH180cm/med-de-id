use mdid_domain::ImageRedactionRegion;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ImageRedactionError {
    #[error("rgb image buffer length does not match dimensions")]
    MalformedRgbBuffer,
    #[error("image redaction region is outside image bounds")]
    RegionOutOfBounds,
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
