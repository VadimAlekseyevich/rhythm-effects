//! Three-slot export GPU readback ring with aligned rows and explicit
//! reservation/release. A frame is never allowed to overwrite an in-flight
//! transfer; consumer mapping and FFmpeg writes run outside the UI/audio path.

use crate::export_sdr::SdrExportConverter;

pub const MAX_EXPORT_READBACK_BUFFERS: usize = 3;
const RGBA8_BYTES_PER_PIXEL: u64 = 4;
const COPY_ROW_ALIGNMENT: u64 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportReadbackError {
    InvalidDimensions,
    SizeOverflow,
    SourceSizeMismatch,
    PoolExhausted,
    StaleTicket,
    InvalidMappedLength,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportReadbackLayout {
    size: [u32; 2],
    packed_row_bytes: u32,
    padded_row_bytes: u32,
    total_bytes: u64,
}

impl ExportReadbackLayout {
    pub fn new(size: [u32; 2]) -> Result<Self, ExportReadbackError> {
        if size[0] == 0 || size[1] == 0 {
            return Err(ExportReadbackError::InvalidDimensions);
        }
        let raw = u64::from(size[0])
            .checked_mul(RGBA8_BYTES_PER_PIXEL)
            .ok_or(ExportReadbackError::SizeOverflow)?;
        let padded = raw
            .div_ceil(COPY_ROW_ALIGNMENT)
            .checked_mul(COPY_ROW_ALIGNMENT)
            .ok_or(ExportReadbackError::SizeOverflow)?;
        let total = padded
            .checked_mul(u64::from(size[1]))
            .ok_or(ExportReadbackError::SizeOverflow)?;
        Ok(Self {
            size,
            packed_row_bytes: u32::try_from(raw).map_err(|_| ExportReadbackError::SizeOverflow)?,
            padded_row_bytes: u32::try_from(padded)
                .map_err(|_| ExportReadbackError::SizeOverflow)?,
            total_bytes: total,
        })
    }

    #[must_use]
    pub const fn size(self) -> [u32; 2] {
        self.size
    }

    #[must_use]
    pub const fn bytes_per_row(self) -> u32 {
        self.padded_row_bytes
    }

    #[must_use]
    pub const fn allocation_bytes(self) -> u64 {
        self.total_bytes
    }

    #[must_use]
    pub const fn bounded_pool_bytes(self) -> u64 {
        self.total_bytes
            .saturating_mul(MAX_EXPORT_READBACK_BUFFERS as u64)
    }

    /// Strip wgpu 256-byte copy-row alignment from a completed mapped buffer
    /// before piping tightly packed RGBA8 bytes into FFmpeg. Nothing is read
    /// beyond the provided mapped range and no padding reaches the encoder.
    pub fn packed_rgba(self, mapped: &[u8]) -> Result<Vec<u8>, ExportReadbackError> {
        let expected =
            usize::try_from(self.total_bytes).map_err(|_| ExportReadbackError::SizeOverflow)?;
        if mapped.len() != expected {
            return Err(ExportReadbackError::InvalidMappedLength);
        }
        let width = usize::try_from(self.packed_row_bytes)
            .map_err(|_| ExportReadbackError::SizeOverflow)?;
        let stride = usize::try_from(self.padded_row_bytes)
            .map_err(|_| ExportReadbackError::SizeOverflow)?;
        let height =
            usize::try_from(self.size[1]).map_err(|_| ExportReadbackError::SizeOverflow)?;
        let packed_len = width
            .checked_mul(height)
            .ok_or(ExportReadbackError::SizeOverflow)?;
        let mut packed = Vec::with_capacity(packed_len);
        for row in mapped.chunks_exact(stride) {
            packed.extend_from_slice(&row[..width]);
        }
        Ok(packed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportReadbackTicket {
    pub frame_index: u64,
    slot: usize,
    serial: u64,
}

#[derive(Debug, Default)]
struct ReadbackReservations {
    active: [Option<ExportReadbackTicket>; MAX_EXPORT_READBACK_BUFFERS],
    next_serial: u64,
}

impl ReadbackReservations {
    fn reserve(&mut self, frame_index: u64) -> Result<ExportReadbackTicket, ExportReadbackError> {
        let slot = self
            .active
            .iter()
            .position(Option::is_none)
            .ok_or(ExportReadbackError::PoolExhausted)?;
        let serial = self
            .next_serial
            .checked_add(1)
            .ok_or(ExportReadbackError::SizeOverflow)?;
        self.next_serial = serial;
        let ticket = ExportReadbackTicket {
            frame_index,
            slot,
            serial,
        };
        self.active[slot] = Some(ticket);
        Ok(ticket)
    }

    fn valid(&self, ticket: ExportReadbackTicket) -> bool {
        self.active.get(ticket.slot).copied().flatten() == Some(ticket)
    }

    fn release(&mut self, ticket: ExportReadbackTicket) -> Result<(), ExportReadbackError> {
        if !self.valid(ticket) {
            return Err(ExportReadbackError::StaleTicket);
        }
        self.active[ticket.slot] = None;
        Ok(())
    }

    fn active_count(&self) -> usize {
        self.active.iter().flatten().count()
    }
}

/// GPU buffers are created once, before export work, and never grow with the
/// frame count or output duration. A consumer maps a reserved buffer, copies
/// its packed data, UNMAPS it, then calls release_after_unmap(ticket).
#[derive(Debug)]
pub struct ExportReadbackPool {
    layout: ExportReadbackLayout,
    buffers: Vec<wgpu::Buffer>,
    reservations: ReadbackReservations,
}

impl ExportReadbackPool {
    #[must_use]
    pub fn new(device: &wgpu::Device, layout: ExportReadbackLayout) -> Self {
        let buffers = (0..MAX_EXPORT_READBACK_BUFFERS)
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Rhythm Effects bounded export readback buffer"),
                    size: layout.allocation_bytes(),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })
            })
            .collect();
        Self {
            layout,
            buffers,
            reservations: ReadbackReservations::default(),
        }
    }

    #[must_use]
    pub const fn layout(&self) -> ExportReadbackLayout {
        self.layout
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.reservations.active_count()
    }

    /// Reserve a distinct COPY_DST buffer and encode the RGBA8 texture copy.
    /// A full pool returns PoolExhausted; callers must handle backpressure,
    /// not allocate another buffer or overwrite pending frames.
    pub fn encode_copy(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        source: &SdrExportConverter,
        frame_index: u64,
    ) -> Result<ExportReadbackTicket, ExportReadbackError> {
        if source.size() != self.layout.size() {
            return Err(ExportReadbackError::SourceSizeMismatch);
        }
        let ticket = self.reservations.reserve(frame_index)?;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: source.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffers[ticket.slot],
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.layout.bytes_per_row()),
                    rows_per_image: Some(self.layout.size()[1]),
                },
            },
            wgpu::Extent3d {
                width: self.layout.size()[0],
                height: self.layout.size()[1],
                depth_or_array_layers: 1,
            },
        );
        Ok(ticket)
    }

    /// Valid until the reservation is released; the export worker controls
    /// its asynchronous map/copy/unmap lifecycle, not the editor UI thread.
    pub fn buffer(
        &self,
        ticket: ExportReadbackTicket,
    ) -> Result<&wgpu::Buffer, ExportReadbackError> {
        if !self.reservations.valid(ticket) {
            return Err(ExportReadbackError::StaleTicket);
        }
        Ok(&self.buffers[ticket.slot])
    }

    pub fn release_after_unmap(
        &mut self,
        ticket: ExportReadbackTicket,
    ) -> Result<(), ExportReadbackError> {
        self.reservations.release(ticket)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExportReadbackError, ExportReadbackLayout, MAX_EXPORT_READBACK_BUFFERS,
        ReadbackReservations,
    };

    #[test]
    fn layout_aligns_gpu_rows_and_strips_padding_for_ffmpeg_rgba() {
        let layout = ExportReadbackLayout::new([3, 2]).expect("3x2 RGBA8");
        assert_eq!(layout.bytes_per_row(), 256);
        assert_eq!(layout.allocation_bytes(), 512);
        let mut raw = vec![0xEE; 512];
        raw[..12].copy_from_slice(&[1; 12]);
        raw[256..268].copy_from_slice(&[2; 12]);
        assert_eq!(layout.packed_rgba(&raw), Ok([[1_u8; 12], [2; 12]].concat()));
        assert_eq!(
            layout.packed_rgba(&raw[..511]),
            Err(ExportReadbackError::InvalidMappedLength)
        );

        let aligned = ExportReadbackLayout::new([64, 1]).expect("aligned row");
        assert_eq!(aligned.bytes_per_row(), 256);
        assert_eq!(aligned.allocation_bytes(), 256);
        assert_eq!(
            ExportReadbackLayout::new([0, 2]),
            Err(ExportReadbackError::InvalidDimensions)
        );
    }

    #[test]
    fn ring_never_issues_more_than_three_unique_live_reservations() {
        let mut slots = ReadbackReservations::default();
        let first = slots.reserve(0).expect("slot 0");
        let second = slots.reserve(1).expect("slot 1");
        let third = slots.reserve(2).expect("slot 2");
        assert_eq!(slots.active_count(), MAX_EXPORT_READBACK_BUFFERS);
        assert_eq!(slots.reserve(3), Err(ExportReadbackError::PoolExhausted));
        assert_ne!(first.slot, second.slot);
        assert_ne!(second.slot, third.slot);
        slots.release(second).expect("release after unmap");
        let reused = slots.reserve(3).expect("reuse slot");
        assert_eq!(reused.slot, second.slot);
        assert_ne!(reused.serial, second.serial);
        assert_eq!(slots.release(second), Err(ExportReadbackError::StaleTicket));
        assert_eq!(slots.active_count(), MAX_EXPORT_READBACK_BUFFERS);
        slots.release(first).expect("release first");
        slots.release(third).expect("release third");
        slots.release(reused).expect("release fourth");
        assert_eq!(slots.active_count(), 0);
    }
}
