#![cfg(feature = "bevy-app")]

use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, Image, ImageSampler, ImageType, TextureError},
    render::render_resource::TextureFormat,
};

const PIXELS: [u8; 16] = [255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 64, 1, 2, 3, 0];

// A 2x2 RGBA8 KTX2 with one zstd-supercompressed mip and a basic RGBA DFD.
// Build it in memory so this test needs no GPU, external asset, or player state.
fn ktx2_with_zstd_mip(mip: &[u8]) -> Vec<u8> {
    let mut bytes = b"\xABKTX 20\xBB\r\n\x1A\n".to_vec();
    for word in [
        37_u32, // VK_FORMAT_R8G8B8A8_UNORM
        1,      // type size
        2, 2, 0, // width, height, depth
        0, 1, 1, // layers, faces, levels
        2, // Zstandard supercompression
        104, 92, // DFD offset and length (header + level index, RGBA descriptor)
        0, 0, // no key/value data
    ] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes.extend_from_slice(&[0; 16]); // no supercompression global data
    for word in [196_u64, mip.len() as u64, PIXELS.len() as u64] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes.extend_from_slice(&92_u32.to_le_bytes()); // DFD total length
    bytes.extend_from_slice(&[0, 0, 0, 0, 2, 0, 88, 0]); // basic descriptor, version 2
    bytes.extend_from_slice(&[1, 1, 1, 0]); // RGBSDA, BT709, linear, straight alpha
    bytes.extend_from_slice(&[0; 4]); // 1x1x1x1 texel block
    bytes.extend_from_slice(&[4, 0, 0, 0, 0, 0, 0, 0]); // four bytes per texel
    for (offset, channel) in [(0_u16, 0), (8, 1), (16, 2), (24, 15)] {
        bytes.extend_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&[7, channel]); // eight-bit sample, channel type
        bytes.extend_from_slice(&[0; 8]); // sample position and lower bound
        bytes.extend_from_slice(&255_u32.to_le_bytes());
    }
    bytes.extend_from_slice(mip);
    bytes
}

fn decode(bytes: &[u8]) -> Result<Image, TextureError> {
    // Same public path used by Bevy's image loader and tonemapping LUT setup.
    Image::from_buffer(
        bytes,
        ImageType::Extension("ktx2"),
        CompressedImageFormats::NONE,
        false,
        ImageSampler::default(),
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[test]
fn bevy_decodes_zstd_supercompressed_ktx2_pixels() {
    let mip = zstd::encode_all(PIXELS.as_slice(), 0).unwrap();
    let image = decode(&ktx2_with_zstd_mip(&mip)).unwrap();
    assert_eq!(image.data.as_deref(), Some(PIXELS.as_slice()));
    assert_eq!(image.texture_descriptor.format, TextureFormat::Rgba8Unorm);
    assert_eq!(image.texture_descriptor.size.width, 2);
    assert_eq!(image.texture_descriptor.size.height, 2);
    assert_eq!(image.texture_descriptor.mip_level_count, 1);
}

#[test]
fn bevy_rejects_invalid_and_truncated_zstd_mips() {
    let mut truncated = zstd::encode_all(PIXELS.as_slice(), 0).unwrap();
    truncated.pop();
    for mip in [b"invalid zstd frame".as_slice(), truncated.as_slice()] {
        // Keep the KTX2 container valid so the error must come from decompression.
        let error = decode(&ktx2_with_zstd_mip(mip)).unwrap_err();
        assert!(matches!(error, TextureError::SuperDecompressionError(_)));
    }
}
