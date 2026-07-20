// SPDX-License-Identifier: GPL-3.0-only

use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, Sender};

use rodio::cpal::traits::HostTrait;
use rodio::cpal::{self, DeviceId, InterfaceType};
use rodio::{Device, DeviceSinkBuilder, DeviceTrait, MixerDeviceSink};
use serde::Serialize;

use super::{PlaybackError, PlaybackFailure};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStatus {
    #[default]
    Closed,
    Ready,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputDeviceSnapshot {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub interface_type: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSnapshot {
    pub status: OutputStatus,
    pub devices: Vec<OutputDeviceSnapshot>,
    pub follow_system_default: bool,
    pub selected_device_id: Option<String>,
    pub active_device_id: Option<String>,
    pub active_device_name: Option<String>,
    pub active_sample_rate: Option<u32>,
    pub active_channel_count: Option<u16>,
    pub active_sample_format: Option<String>,
    pub error: Option<PlaybackFailure>,
}

pub(super) struct OutputDeviceManager {
    output: Option<MixerDeviceSink>,
    selected_device_id: Option<String>,
    active_device_id: Option<String>,
    active_device_name: Option<String>,
    devices: Vec<OutputDeviceSnapshot>,
    status: OutputStatus,
    error: Option<PlaybackFailure>,
    stream_errors: Receiver<String>,
    stream_error_sender: Sender<String>,
}

impl OutputDeviceManager {
    pub(super) fn new() -> Self {
        let (stream_error_sender, stream_errors) = mpsc::channel();
        Self {
            output: None,
            selected_device_id: None,
            active_device_id: None,
            active_device_name: None,
            devices: Vec::new(),
            status: OutputStatus::Closed,
            error: None,
            stream_errors,
            stream_error_sender,
        }
    }

    pub(super) fn snapshot(&self) -> OutputSnapshot {
        let config = self.output.as_ref().map(MixerDeviceSink::config);
        OutputSnapshot {
            status: self.status,
            devices: self.devices.clone(),
            follow_system_default: self.selected_device_id.is_none(),
            selected_device_id: self.selected_device_id.clone(),
            active_device_id: self.active_device_id.clone(),
            active_device_name: self.active_device_name.clone(),
            active_sample_rate: config.map(|value| value.sample_rate().get()),
            active_channel_count: config.map(|value| value.channel_count().get()),
            active_sample_format: config.map(|value| value.sample_format().to_string()),
            error: self.error.clone(),
        }
    }

    pub(super) fn output(&self) -> Option<&MixerDeviceSink> {
        self.output.as_ref()
    }

    pub(super) fn has_output(&self) -> bool {
        self.output.is_some()
    }

    pub(super) fn close(&mut self) {
        self.output = None;
        self.active_device_id = None;
        self.active_device_name = None;
        self.status = OutputStatus::Closed;
    }

    pub(super) fn ensure_open(&mut self) -> Result<(), PlaybackError> {
        if self.output.is_none() {
            self.open_selection(self.selected_device_id.clone())?;
        }
        Ok(())
    }

    pub(super) fn select(&mut self, device_id: Option<String>) -> Result<bool, PlaybackError> {
        if self.selected_device_id == device_id && self.output.is_some() {
            return Ok(false);
        }
        self.open_selection(device_id)?;
        Ok(true)
    }

    pub(super) fn reopen(&mut self) -> Result<(), PlaybackError> {
        self.open_selection(self.selected_device_id.clone())
    }

    pub(super) fn refresh(&mut self) -> Result<OutputSnapshot, PlaybackError> {
        self.devices = enumerate_output_devices()?;
        Ok(self.snapshot())
    }

    pub(super) fn needs_reopen(&mut self) -> Result<bool, PlaybackError> {
        self.devices = enumerate_output_devices()?;
        let desired_id = desired_device_id(self.selected_device_id.as_deref())?;
        Ok(self.active_device_id.as_deref() != Some(desired_id.as_str()))
    }

    pub(super) fn selected_available(&self) -> Result<bool, PlaybackError> {
        let host = cpal::default_host();
        match self.selected_device_id.as_deref() {
            Some(id) => {
                let id = DeviceId::from_str(id)
                    .map_err(|error| PlaybackError::InvalidOutputDevice(error.to_string()))?;
                Ok(host.device_by_id(&id).is_some())
            }
            None => Ok(host.default_output_device().is_some()),
        }
    }

    pub(super) fn take_stream_error(&self) -> Option<String> {
        self.stream_errors.try_recv().ok()
    }

    pub(super) fn mark_unavailable(&mut self, error: &PlaybackError) {
        self.close();
        self.status = OutputStatus::Unavailable;
        self.error = Some(error.failure());
    }

    fn open_selection(&mut self, device_id: Option<String>) -> Result<(), PlaybackError> {
        let (device, active_id, active_name) = resolve_device(device_id.as_deref())?;
        let error_sender = self.stream_error_sender.clone();
        let mut output = DeviceSinkBuilder::from_device(device)
            .map_err(|error| PlaybackError::OpenOutput(error.to_string()))?
            .with_error_callback(move |error| {
                let _ = error_sender.send(error.to_string());
            })
            .open_sink_or_fallback()
            .map_err(|error| PlaybackError::OpenOutput(error.to_string()))?;
        output.log_on_drop(false);

        self.output = Some(output);
        self.selected_device_id = device_id;
        self.active_device_id = Some(active_id);
        self.active_device_name = Some(active_name);
        self.status = OutputStatus::Ready;
        self.error = None;
        if let Ok(devices) = enumerate_output_devices() {
            self.devices = devices;
        }
        Ok(())
    }
}

fn enumerate_output_devices() -> Result<Vec<OutputDeviceSnapshot>, PlaybackError> {
    let host = cpal::default_host();
    let default_id = host
        .default_output_device()
        .and_then(|device| device.id().ok())
        .map(|id| id.to_string());
    let devices = host
        .output_devices()
        .map_err(|error| PlaybackError::ListOutputDevices(error.to_string()))?;
    let mut snapshots = devices
        .filter_map(|device| {
            let id = device.id().ok()?.to_string();
            let description = device.description().ok();
            Some(OutputDeviceSnapshot {
                is_default: default_id.as_deref() == Some(id.as_str()),
                id,
                name: description
                    .as_ref()
                    .map(|description| description.name().to_owned())
                    .unwrap_or_else(|| "Unknown output device".to_owned()),
                interface_type: description
                    .as_ref()
                    .map(|description| interface_type_name(description.interface_type()))
                    .unwrap_or("unknown")
                    .to_owned(),
            })
        })
        .collect::<Vec<_>>();
    snapshots.sort_by(|left, right| {
        right
            .is_default
            .cmp(&left.is_default)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(snapshots)
}

fn desired_device_id(selected_device_id: Option<&str>) -> Result<String, PlaybackError> {
    let host = cpal::default_host();
    let device = match selected_device_id {
        Some(id) => {
            let id = DeviceId::from_str(id)
                .map_err(|error| PlaybackError::InvalidOutputDevice(error.to_string()))?;
            host.device_by_id(&id)
        }
        None => host.default_output_device(),
    }
    .ok_or_else(|| {
        PlaybackError::OutputDeviceUnavailable(
            selected_device_id.unwrap_or("system_default").to_owned(),
        )
    })?;
    device
        .id()
        .map(|id| id.to_string())
        .map_err(|error| PlaybackError::InvalidOutputDevice(error.to_string()))
}

fn resolve_device(
    selected_device_id: Option<&str>,
) -> Result<(Device, String, String), PlaybackError> {
    let host = cpal::default_host();
    let device = match selected_device_id {
        Some(id) => {
            let id = DeviceId::from_str(id)
                .map_err(|error| PlaybackError::InvalidOutputDevice(error.to_string()))?;
            host.device_by_id(&id)
        }
        None => host.default_output_device(),
    }
    .ok_or_else(|| {
        PlaybackError::OutputDeviceUnavailable(
            selected_device_id.unwrap_or("system_default").to_owned(),
        )
    })?;
    let id = device
        .id()
        .map_err(|error| PlaybackError::InvalidOutputDevice(error.to_string()))?
        .to_string();
    let name = device
        .description()
        .map(|description| description.name().to_owned())
        .unwrap_or_else(|_| "Unknown output device".to_owned());
    Ok((device, id, name))
}

fn interface_type_name(interface_type: InterfaceType) -> &'static str {
    match interface_type {
        InterfaceType::BuiltIn => "built_in",
        InterfaceType::Usb => "usb",
        InterfaceType::Bluetooth => "bluetooth",
        InterfaceType::Pci => "pci",
        InterfaceType::FireWire => "firewire",
        InterfaceType::Thunderbolt => "thunderbolt",
        InterfaceType::Hdmi => "hdmi",
        InterfaceType::Line => "line",
        InterfaceType::Spdif => "spdif",
        InterfaceType::Network => "network",
        InterfaceType::Virtual => "virtual",
        InterfaceType::DisplayPort => "display_port",
        InterfaceType::Aggregate => "aggregate",
        InterfaceType::Unknown => "unknown",
        _ => "unknown",
    }
}
