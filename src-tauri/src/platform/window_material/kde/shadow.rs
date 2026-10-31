// SPDX-License-Identifier: GPL-3.0-only

use super::BlurState;
use std::{io::Write, os::fd::AsFd};
use wayland_client::{
    protocol::{wl_buffer::WlBuffer, wl_registry::WlRegistry, wl_shm, wl_surface::WlSurface},
    QueueHandle,
};
use wayland_protocols_plasma::shadow::client::{
    org_kde_kwin_shadow::OrgKdeKwinShadow, org_kde_kwin_shadow_manager::OrgKdeKwinShadowManager,
};

const EXTENT: i32 = 24;
const SIDE: i32 = EXTENT * 2 + 1;
// Clockwise from the top-left; the middle pixel is never submitted.
const TILES: [(i32, i32, i32, i32); 8] = [
    (0, 0, EXTENT, EXTENT),
    (EXTENT, 0, 1, EXTENT),
    (EXTENT + 1, 0, EXTENT, EXTENT),
    (EXTENT + 1, EXTENT, EXTENT, 1),
    (EXTENT + 1, EXTENT + 1, EXTENT, EXTENT),
    (EXTENT, EXTENT + 1, 1, EXTENT),
    (0, EXTENT + 1, EXTENT, EXTENT),
    (0, EXTENT, EXTENT, 1),
];

#[derive(Default)]
pub(super) struct Shadow {
    manager: Option<(u32, OrgKdeKwinShadowManager)>,
    shm: Option<(u32, wl_shm::WlShm)>,
    buffers: Vec<WlBuffer>,
    effect: Option<OrgKdeKwinShadow>,
    failed: bool,
}

impl Shadow {
    pub fn bind(
        &mut self,
        registry: &WlRegistry,
        name: u32,
        interface: &str,
        version: u32,
        qh: &QueueHandle<BlurState>,
    ) {
        // Version 2 lets every owned protocol object be released on window teardown.
        if version < 2 {
            log::warn!("KDE shadow unavailable: {interface} version {version} < 2");
            return;
        }
        match interface {
            "org_kde_kwin_shadow_manager" => {
                self.manager = Some((name, registry.bind(name, 2, qh, ())));
            }
            "wl_shm" => self.shm = Some((name, registry.bind(name, 2, qh, ()))),
            _ => unreachable!(),
        }
        self.failed = false;
    }

    pub fn remove_global(&mut self, name: u32, surface: &WlSurface) {
        let manager_removed = self.manager.as_ref().is_some_and(|(id, _)| *id == name);
        let shm_removed = self.shm.as_ref().is_some_and(|(id, _)| *id == name);
        if manager_removed || shm_removed {
            self.detach(surface);
            self.clear_buffers();
            if manager_removed {
                self.manager.take().unwrap().1.destroy();
            }
            if shm_removed {
                self.shm.take().unwrap().1.release();
            }
            log::info!("KDE shadow capability removed");
        }
    }

    pub fn update(&mut self, surface: &WlSurface, normal: bool, qh: &QueueHandle<BlurState>) {
        if !normal {
            self.detach(surface);
            return;
        }
        let (Some((_, manager)), Some((_, shm))) = (&self.manager, &self.shm) else {
            return;
        };
        if self.effect.is_some() || self.failed {
            return;
        }
        if self.buffers.is_empty() {
            match create_buffers(shm, qh) {
                Ok(buffers) => self.buffers = buffers,
                Err(error) => {
                    self.failed = true;
                    log::warn!("KDE shadow unavailable: {error}");
                    return;
                }
            }
        }
        let shadow = manager.create(surface, qh, ());
        shadow.attach_top_left(&self.buffers[0]);
        shadow.attach_top(&self.buffers[1]);
        shadow.attach_top_right(&self.buffers[2]);
        shadow.attach_right(&self.buffers[3]);
        shadow.attach_bottom_right(&self.buffers[4]);
        shadow.attach_bottom(&self.buffers[5]);
        shadow.attach_bottom_left(&self.buffers[6]);
        shadow.attach_left(&self.buffers[7]);
        shadow.set_left_offset(f64::from(EXTENT));
        shadow.set_top_offset(f64::from(EXTENT));
        shadow.set_right_offset(f64::from(EXTENT));
        shadow.set_bottom_offset(f64::from(EXTENT));
        shadow.commit();
        self.effect = Some(shadow);
        log::debug!("KDE shadow attached");
    }

    fn detach(&mut self, surface: &WlSurface) {
        if let Some(shadow) = self.effect.take() {
            if let Some((_, manager)) = &self.manager {
                manager.unset(surface);
            }
            shadow.destroy();
            log::debug!("KDE shadow detached");
        }
    }

    pub fn clear(&mut self) {
        // GTK may already have destroyed its surface when unmap/unrealize runs.
        if let Some(shadow) = self.effect.take() {
            shadow.destroy();
        }
        self.clear_buffers();
        if let Some((_, manager)) = self.manager.take() {
            manager.destroy();
        }
        if let Some((_, shm)) = self.shm.take() {
            shm.release();
        }
    }

    fn clear_buffers(&mut self) {
        for buffer in self.buffers.drain(..) {
            buffer.destroy();
        }
    }

    #[cfg(test)]
    pub fn attached(&self) -> bool {
        self.effect.is_some()
    }
}

fn create_buffers(
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<BlurState>,
) -> std::io::Result<Vec<WlBuffer>> {
    let mut file = tempfile::tempfile()?;
    let pixels = pixels();
    file.write_all(&pixels)?;
    let pool = shm.create_pool(file.as_fd(), pixels.len() as i32, qh, ());
    let buffers = TILES
        .map(|(x, y, width, height)| {
            pool.create_buffer(
                (y * SIDE + x) * 4,
                width,
                height,
                SIDE * 4,
                wl_shm::Format::Argb8888,
                qh,
                (),
            )
        })
        .into();
    // Buffers retain the pool mapping; its immutable contents are never rewritten.
    pool.destroy();
    Ok(buffers)
}

fn pixels() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((SIDE * (SIDE + 1) * 4) as usize);
    let cutoff = (-f64::from(EXTENT * EXTENT) / 128.0).exp();
    for y in 0..SIDE {
        for x in 0..SIDE {
            let dx = (f64::from((x - EXTENT).abs()) - 0.5).max(0.0);
            let dy = (f64::from((y - EXTENT).abs()) - 0.5).max(0.0);
            let squared_distance = dx * dx + dy * dy;
            let alpha = 0.18 * ((-squared_distance / 128.0).exp() - cutoff).max(0.0)
                + 0.06 * (-squared_distance / 8.0).exp();
            // Native-endian premultiplied ARGB: black with a soft, bounded alpha.
            pixels.extend_from_slice(&(((alpha * 255.0).round() as u32) << 24).to_ne_bytes());
        }
    }
    // wl_shm validates offset + stride * height, including the last row's padding.
    pixels.resize((SIDE * (SIDE + 1) * 4) as usize, 0);
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_fit_the_pool_and_form_a_continuous_fading_black_shadow() {
        let pixels = pixels();
        let pixel = |x, y| {
            let offset = ((y * SIDE + x) * 4) as usize;
            u32::from_ne_bytes(pixels[offset..offset + 4].try_into().unwrap())
        };
        let mut coverage = vec![0; (SIDE * SIDE) as usize];
        for (x, y, width, height) in TILES {
            assert!(((y * SIDE + x + SIDE * height) * 4) as usize <= pixels.len());
            for y in y..y + height {
                for x in x..x + width {
                    coverage[(y * SIDE + x) as usize] += 1;
                    assert_eq!(pixel(x, y) & 0x00ff_ffff, 0);
                    assert!(pixel(x, y) >> 24 <= 61);
                    assert_eq!(pixel(x, y), pixel(SIDE - 1 - x, y));
                    assert_eq!(pixel(x, y), pixel(x, SIDE - 1 - y));
                }
            }
        }
        coverage[(EXTENT * SIDE + EXTENT) as usize] = 1;
        assert!(coverage.into_iter().all(|count| count == 1));
        assert_eq!(pixel(0, EXTENT), 0);
        for y in 0..EXTENT {
            assert!(pixel(EXTENT, y) <= pixel(EXTENT, y + 1));
            assert!(
                (i64::from(pixel(EXTENT - 1, y) >> 24) - i64::from(pixel(EXTENT, y) >> 24)).abs()
                    <= 1
            );
        }
    }
}
