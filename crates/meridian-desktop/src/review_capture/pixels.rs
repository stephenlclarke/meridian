//! Optional actual-surface copies. One in flight, three explicit requests, no GPU wait.
use super::{Capture, Tag, digest};
use std::{
    io::Write,
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
};
const MAX_BYTES: u64 = 16 * 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Layout {
    pub width: u32,
    pub height: u32,
    pub row: u32,
    pub padded: u32,
    pub bytes: u64,
}
impl Layout {
    pub fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        let row = width.checked_mul(4)?;
        let padded = row.checked_add(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - 1)?
            / wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let bytes = u64::from(padded).checked_mul(u64::from(height))?;
        (bytes <= MAX_BYTES).then_some(Self {
            width,
            height,
            row,
            padded,
            bytes,
        })
    }
    pub fn unpack(&self, mapped: &[u8]) -> Option<Vec<u8>> {
        if mapped.len() != usize::try_from(self.bytes).ok()? {
            return None;
        }
        let mut packed =
            Vec::with_capacity(usize::try_from(u64::from(self.row) * u64::from(self.height)).ok()?);
        for row in mapped.chunks_exact(self.padded as usize) {
            packed.extend_from_slice(&row[..self.row as usize]);
        }
        Some(packed)
    }
}
pub(super) fn supported(format: wgpu::TextureFormat) -> bool {
    matches!(
        format,
        wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Bgra8UnormSrgb
            | wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Rgba8UnormSrgb
    )
}
fn copy_eligible(
    directory: bool,
    usage: Option<wgpu::TextureUsages>,
    format: wgpu::TextureFormat,
) -> bool {
    directory
        && usage.is_some_and(|u| u.contains(wgpu::TextureUsages::COPY_SRC))
        && supported(format)
}
fn save_pixels(directory: &std::path::Path, tag: Tag, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let path = directory.join(format!(
        "review-request-{}-frame-{}.pixels",
        tag.request, tag.frame
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(path)
}
fn map_status(receiver: &Receiver<Result<(), String>>) -> Option<Result<(), String>> {
    match receiver.try_recv() {
        Ok(result) => Some(result),
        Err(TryRecvError::Disconnected) => Some(Err("map callback channel disconnected".into())),
        Err(TryRecvError::Empty) => None,
    }
}
struct Pending {
    tag: Tag,
    layout: Layout,
    format: wgpu::TextureFormat,
    buffer: wgpu::Buffer,
    result: Option<Receiver<Result<(), String>>>,
}
pub(crate) struct Pixels {
    capture: Capture,
    directory: Option<PathBuf>,
    usage_supported: bool,
    pending: Option<Pending>,
}
impl Pixels {
    pub fn new(
        capture: Capture,
        capabilities: Option<&wgpu::SurfaceCapabilities>,
        config: &mut wgpu::SurfaceConfiguration,
    ) -> Self {
        let directory = if capture.enabled() {
            std::env::var_os("MERIDIAN_REVIEW_CAPTURE_DIR").map(PathBuf::from)
        } else {
            None
        };
        let usage_supported = copy_eligible(
            directory.as_ref().is_some_and(|path| path.is_dir()),
            capabilities.map(|c| c.usages),
            config.format,
        );
        if directory.is_some() && usage_supported {
            config.usage |= wgpu::TextureUsages::COPY_SRC;
        }
        Self {
            capture,
            directory,
            usage_supported,
            pending: None,
        }
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn encode(
        &mut self,
        tag: Tag,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::Texture,
    ) {
        if self.pending.is_some() {
            self.capture
                .event("pixels_unobservable", (tag, "copy already in flight"));
            return;
        }
        if self.directory.is_none()
            || !self.usage_supported
            || !target.usage().contains(wgpu::TextureUsages::COPY_SRC)
            || !supported(target.format())
        {
            self.capture.event("pixels_unobservable",(tag,"explicit existing capture directory, COPY_SRC surface capability/actual texture usage and RGBA/BGRA8 format required",target.format(),target.usage()));
            return;
        }
        let Some(layout) = Layout::new(target.width(), target.height()) else {
            self.capture.event(
                "pixels_unobservable",
                (tag, "zero/oversize/overflow target;16MiB padded limit"),
            );
            return;
        };
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Opt-in bounded actual review target readback"),
            size: layout.bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(layout.padded),
                    rows_per_image: Some(layout.height),
                },
            },
            wgpu::Extent3d {
                width: layout.width,
                height: layout.height,
                depth_or_array_layers: 1,
            },
        );
        self.capture.event("pixels_copy_encoded",(tag,layout,target.format(),"same final colour target after overlay, before submit/present; completion UNOBSERVABLE until map callback"));
        self.pending = Some(Pending {
            tag,
            layout,
            format: target.format(),
            buffer,
            result: None,
        });
    }
    pub fn submitted(&mut self) {
        let Some(p) = self.pending.as_mut().filter(|p| p.result.is_none()) else {
            return;
        };
        let (send, receive) = mpsc::channel();
        p.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = send.send(result.map_err(|e| e.to_string()));
            });
        p.result = Some(receive);
        self.capture.event(
            "pixels_map_requested",
            (p.tag, "asynchronous; no wait, no display success claim"),
        );
    }
    pub fn service(&mut self, device: &wgpu::Device) {
        if self.pending.is_none() {
            return;
        }
        if let Err(error) = device.poll(wgpu::PollType::Poll) {
            self.capture
                .event("pixels_poll_error", format!("{error:?}"));
            self.abandon("nonblocking poll failed");
            return;
        }
        let Some(result) = self
            .pending
            .as_ref()
            .and_then(|p| p.result.as_ref())
            .and_then(map_status)
        else {
            return;
        };
        let Some(p) = self.pending.take() else {
            return;
        };
        match result {
            Ok(()) => {
                self.capture.event(
                    "pixels_map_complete",
                    (
                        p.tag,
                        p.layout,
                        p.format,
                        "callback Ok; copied buffer readable; compositor/display UNOBSERVABLE",
                    ),
                );
                let view = p.buffer.slice(..).get_mapped_range();
                let packed = p.layout.unpack(&view);
                drop(view);
                p.buffer.unmap();
                if let Some(bytes) = packed {
                    self.write_artifact(&p, &bytes);
                } else {
                    self.capture.event(
                        "pixels_unobservable",
                        (p.tag, "mapped length did not match bounded layout"),
                    );
                }
            }
            Err(error) => self.capture.event("pixels_map_failed", (p.tag, error)),
        }
    }
    fn write_artifact(&self, p: &Pending, bytes: &[u8]) {
        let Some(directory) = &self.directory else {
            return;
        };
        let path = directory.join(format!(
            "review-request-{}-frame-{}.pixels",
            p.tag.request, p.tag.frame
        ));
        let written = save_pixels(directory, p.tag, bytes);
        match written {
            Ok(_)=>self.capture.event("pixels_artifact",(p.tag,&path,p.layout,p.format,bytes.len(),digest(&[bytes]),"packed top-to-bottom rows, native RGBA/BGRA channel order; not PNG; exact copied target, not screen capture")),
            Err(error)=>self.capture.event("pixels_artifact_failed",(p.tag,&path,error.to_string(),"partial file may exist; never overwritten/retried")),
        }
    }
    fn abandon(&mut self, reason: &'static str) {
        if let Some(p) = self.pending.take() {
            p.buffer.unmap();
            self.capture.event(
                "pixels_unobservable",
                (p.tag, reason, "no completion/artifact claimed"),
            );
        }
    }
}
impl Drop for Pixels {
    fn drop(&mut self) {
        self.abandon("renderer exited with readback incomplete");
    }
}
#[cfg(test)]
mod tests;
