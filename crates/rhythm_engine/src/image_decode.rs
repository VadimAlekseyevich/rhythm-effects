use std::{
    path::{Path, PathBuf},
    sync::mpsc::{
        Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError, sync_channel,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use image::{ImageFormat, ImageReader};
use rhythm_core::ids::AssetId;

pub const IMAGE_DECODE_QUEUE_CAPACITY: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ImageDecodeGeneration(u64);

impl ImageDecodeGeneration {
    pub const INITIAL: Self = Self(0);

    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageDecodeRequest {
    pub asset_id: AssetId,
    pub generation: ImageDecodeGeneration,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

impl DecodedImage {
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.rgba8.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageDecodeError {
    Io(String),
    UnsupportedFormat,
    Decode(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageDecodeResult {
    pub asset_id: AssetId,
    pub generation: ImageDecodeGeneration,
    pub path: PathBuf,
    pub result: Result<DecodedImage, ImageDecodeError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageDecodeSubmitError {
    QueueFull(ImageDecodeRequest),
    Disconnected(ImageDecodeRequest),
}

pub struct ImageDecodeWorker {
    request_tx: SyncSender<ImageDecodeRequest>,
    result_rx: Receiver<ImageDecodeResult>,
    _thread: JoinHandle<()>,
}

impl ImageDecodeWorker {
    pub fn spawn() -> std::io::Result<Self> {
        let (request_tx, request_rx) =
            sync_channel::<ImageDecodeRequest>(IMAGE_DECODE_QUEUE_CAPACITY);
        let (result_tx, result_rx) = sync_channel::<ImageDecodeResult>(IMAGE_DECODE_QUEUE_CAPACITY);

        let thread = thread::Builder::new()
            .name("rhythm-image-decode".to_owned())
            .spawn(move || {
                while let Ok(request) = request_rx.recv() {
                    let result = decode_image_file(&request.path);
                    let response = ImageDecodeResult {
                        asset_id: request.asset_id,
                        generation: request.generation,
                        path: request.path,
                        result,
                    };
                    if result_tx.send(response).is_err() {
                        break;
                    }
                }
            })?;

        Ok(Self {
            request_tx,
            result_rx,
            _thread: thread,
        })
    }

    pub fn try_submit(&self, request: ImageDecodeRequest) -> Result<(), ImageDecodeSubmitError> {
        self.request_tx
            .try_send(request)
            .map_err(|error| match error {
                TrySendError::Full(request) => ImageDecodeSubmitError::QueueFull(request),
                TrySendError::Disconnected(request) => {
                    ImageDecodeSubmitError::Disconnected(request)
                }
            })
    }

    pub fn try_recv(&self) -> Result<Option<ImageDecodeResult>, TryRecvError> {
        match self.result_rx.try_recv() {
            Ok(result) => Ok(Some(result)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(error @ TryRecvError::Disconnected) => Err(error),
        }
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<ImageDecodeResult, RecvTimeoutError> {
        self.result_rx.recv_timeout(timeout)
    }
}

fn decode_image_file(path: &Path) -> Result<DecodedImage, ImageDecodeError> {
    let reader =
        ImageReader::open(path).map_err(|error| ImageDecodeError::Io(error.to_string()))?;
    let reader = reader
        .with_guessed_format()
        .map_err(|error| ImageDecodeError::Io(error.to_string()))?;
    if !matches!(
        reader.format(),
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP)
    ) {
        return Err(ImageDecodeError::UnsupportedFormat);
    }

    let image = reader
        .decode()
        .map_err(|error| ImageDecodeError::Decode(error.to_string()))?
        .into_rgba8();
    let (width, height) = image.dimensions();

    Ok(DecodedImage {
        width,
        height,
        rgba8: image.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        IMAGE_DECODE_QUEUE_CAPACITY, ImageDecodeError, ImageDecodeGeneration, ImageDecodeRequest,
        ImageDecodeWorker, decode_image_file,
    };
    use image::{
        ExtendedColorType, ImageEncoder,
        codecs::{jpeg::JpegEncoder, png::PngEncoder, webp::WebPEncoder},
    };
    use rhythm_core::ids::AssetId;
    use std::{
        fs,
        path::PathBuf,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_path(extension: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rhythm-effects-image-{}-{nanos}.{extension}",
            std::process::id()
        ))
    }

    fn write_rgba_png(path: &PathBuf, width: u32, height: u32, rgba: &[u8]) {
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(rgba, width, height, ExtendedColorType::Rgba8)
            .expect("encode PNG fixture");
        fs::write(path, encoded).expect("write PNG fixture");
    }

    fn write_rgb_jpeg(path: &PathBuf, width: u32, height: u32, rgb: &[u8]) {
        let mut encoded = Vec::new();
        JpegEncoder::new_with_quality(&mut encoded, 100)
            .write_image(rgb, width, height, ExtendedColorType::Rgb8)
            .expect("encode JPEG fixture");
        fs::write(path, encoded).expect("write JPEG fixture");
    }

    fn write_rgba_webp(path: &PathBuf, width: u32, height: u32, rgba: &[u8]) {
        let mut encoded = Vec::new();
        WebPEncoder::new_lossless(&mut encoded)
            .write_image(rgba, width, height, ExtendedColorType::Rgba8)
            .expect("encode WebP fixture");
        fs::write(path, encoded).expect("write WebP fixture");
    }

    #[test]
    fn png_decode_returns_rgba8_pixels_and_intrinsic_dimensions() {
        let path = unique_temp_path("png");
        write_rgba_png(&path, 2, 1, &[255, 0, 0, 255, 0, 128, 255, 64]);

        let decoded = decode_image_file(&path).expect("decode PNG");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.byte_len(), 8);
        assert_eq!(decoded.rgba8, vec![255, 0, 0, 255, 0, 128, 255, 64]);

        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn jpeg_decode_returns_rgba8_pixels_and_intrinsic_dimensions() {
        let path = unique_temp_path("jpg");
        write_rgb_jpeg(&path, 2, 1, &[240, 20, 10, 10, 220, 40]);

        let decoded = decode_image_file(&path).expect("decode JPEG");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.byte_len(), 8);
        assert_eq!(decoded.rgba8[3], 255);
        assert_eq!(decoded.rgba8[7], 255);

        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn webp_decode_returns_rgba8_pixels_and_intrinsic_dimensions() {
        let path = unique_temp_path("webp");
        write_rgba_webp(&path, 2, 1, &[12, 34, 56, 78, 90, 123, 210, 255]);

        let decoded = decode_image_file(&path).expect("decode WebP");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.byte_len(), 8);
        assert_eq!(decoded.rgba8, vec![12, 34, 56, 78, 90, 123, 210, 255]);

        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn unsupported_input_is_rejected_before_decode() {
        let path = unique_temp_path("bin");
        fs::write(&path, b"not an image").expect("write invalid fixture");

        assert_eq!(
            decode_image_file(&path),
            Err(ImageDecodeError::UnsupportedFormat)
        );

        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn worker_preserves_asset_and_generation_identity() {
        let path = unique_temp_path("png");
        write_rgba_png(&path, 1, 1, &[10, 20, 30, 40]);

        let worker = ImageDecodeWorker::spawn().expect("spawn image worker");
        let asset_id = AssetId::new(7).expect("asset id");
        let generation = ImageDecodeGeneration::new(3);
        worker
            .try_submit(ImageDecodeRequest {
                asset_id,
                generation,
                path: path.clone(),
            })
            .expect("queue image decode");

        let result = worker
            .recv_timeout(Duration::from_secs(2))
            .expect("decode result");
        assert_eq!(result.asset_id, asset_id);
        assert_eq!(result.generation, generation);
        assert_eq!(result.path, path);
        assert_eq!(
            result.result.expect("decoded image").rgba8,
            vec![10, 20, 30, 40]
        );

        fs::remove_file(result.path).expect("remove fixture");
    }

    #[test]
    fn queue_capacity_is_small_and_bounded() {
        assert_eq!(IMAGE_DECODE_QUEUE_CAPACITY, 4);
    }
}
