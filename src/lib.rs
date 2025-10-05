use cross_usb::usb::{
    ControlIn, ControlOut, ControlType, Recipient, UsbDevice, UsbDeviceInfo, UsbInterface,
};
use dfu_core::DfuProtocol;
use futures::executor::block_on;
use thiserror::Error;

// USB standard constants from the `usb` crate
use usb::standard_request;

// DFU-specific descriptor constants (DFU 1.1 Specification, Section 4.2.4)
// Reference: https://www.usb.org/sites/default/files/DFU_1.1.pdf
const DFU_FUNCTIONAL_DESCRIPTOR_TYPE: u8 = 0x21;
const DFU_FUNCTIONAL_DESCRIPTOR_INDEX: u8 = 0x00;

// Type aliases for DFU helper wrappers
pub type DfuSync = dfu_core::sync::DfuSync<DfuCrossUsb, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Device not found")]
    DeviceNotFound,
    #[error("Functional Desciptor not found")]
    FunctionalDescriptorNotFound,
    #[error("Alternative setting not found")]
    AltSettingNotFound,
    #[error(transparent)]
    FunctionalDescriptor(#[from] dfu_core::functional_descriptor::Error),
    #[error(transparent)]
    Dfu(#[from] dfu_core::Error),
    #[error(transparent)]
    WebUsb(#[from] cross_usb::usb::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct DfuCrossUsb {
    device: cross_usb::Device,
    interface: cross_usb::Interface,
    descriptor: dfu_core::functional_descriptor::FunctionalDescriptor,
    protocol: dfu_core::DfuProtocol<dfu_core::memory_layout::MemoryLayout>,
}

impl DfuCrossUsb {
    /// Open a DFU device from a device info.
    ///
    /// Since cross_usb doesn't expose descriptor parsing (limited in Web USB),
    /// we use control transfers to fetch the DFU functional descriptor.
    ///
    /// # Arguments
    /// * `device_info` - The device info to open
    /// * `interface_num` - The interface number to claim (usually 0)
    /// * `alt_setting` - The alternate setting to use (usually 0)
    pub async fn open_async(
        device_info: cross_usb::DeviceInfo,
        interface_num: u8,
        alt_setting: u8,
    ) -> Result<Self, Error> {
        // Open the device
        let device = device_info.open().await?;

        // Open the interface
        let interface = device.open_interface(interface_num).await?;

        // Set alternate setting using control transfer
        interface
            .control_out(ControlOut {
                control_type: ControlType::Standard,
                recipient: Recipient::Interface,
                request: standard_request::SET_INTERFACE,
                value: alt_setting as u16,
                index: interface_num as u16,
                data: &[],
            })
            .await?;

        // Fetch the DFU functional descriptor using a control transfer
        // wValue format: high byte = descriptor type, low byte = descriptor index
        let descriptor_bytes = interface
            .control_in(ControlIn {
                control_type: ControlType::Standard,
                recipient: Recipient::Interface,
                request: standard_request::GET_DESCRIPTOR,
                value: ((DFU_FUNCTIONAL_DESCRIPTOR_TYPE as u16) << 8)
                    | (DFU_FUNCTIONAL_DESCRIPTOR_INDEX as u16),
                index: interface_num as u16,
                length: 9, // DFU functional descriptor is 9 bytes
            })
            .await?;

        let descriptor =
            dfu_core::functional_descriptor::FunctionalDescriptor::from_bytes(&descriptor_bytes)
                .ok_or(Error::FunctionalDescriptorNotFound)??;

        // Try to read interface string descriptor for DfuSe memory layout
        // This requires GET_DESCRIPTOR for string, but may not be available
        // For now, use empty string (works for standard DFU, DfuSe may need memory layout passed in)
        let interface_string = String::new();

        let protocol = DfuProtocol::new(&interface_string, descriptor.dfu_version)?;

        Ok(Self {
            device,
            interface,
            descriptor,
            protocol,
        })
    }

    /// Wrap device in a *sync* DFU helper.
    ///
    /// This provides convenient methods like `download()` for firmware uploads.
    pub fn into_sync_dfu(self) -> DfuSync {
        DfuSync::new(self)
    }
}

fn split_request_type(request_type: u8) -> (ControlType, Recipient) {
    (
        match request_type >> 5 & 0x03 {
            0 => ControlType::Standard,
            1 => ControlType::Class,
            2 => ControlType::Vendor,
            _ => ControlType::Standard,
        },
        match request_type & 0x1f {
            0 => Recipient::Device,
            1 => Recipient::Interface,
            2 => Recipient::Endpoint,
            3 => Recipient::Other,
            _ => Recipient::Device,
        },
    )
}

impl dfu_core::DfuIo for DfuCrossUsb {
    type Read = usize;
    type Write = usize;
    type Reset = ();
    type Error = Error;
    type MemoryLayout = dfu_core::memory_layout::MemoryLayout;

    fn read_control(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        buffer: &mut [u8],
    ) -> Result<Self::Read, Self::Error> {
        let (control_type, recipient) = split_request_type(request_type);
        let bytes = block_on(self.interface.control_in(ControlIn {
            control_type,
            index: self.interface.number() as u16,
            recipient,
            request,
            value,
            length: buffer.len() as u16,
        }))?;
        buffer.copy_from_slice(&bytes);
        Ok(bytes.len())
    }

    fn write_control(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        buffer: &[u8],
    ) -> Result<Self::Write, Self::Error> {
        let (control_type, recipient) = split_request_type(request_type);
        let bytes_written = block_on(self.interface.control_out(ControlOut {
            control_type,
            index: self.interface.number() as u16,
            recipient,
            request,
            value,
            data: buffer,
        }))?;
        Ok(bytes_written)
    }

    fn usb_reset(&self) -> Result<Self::Reset, Self::Error> {
        Ok(block_on(self.device.reset())?)
    }

    fn protocol(&self) -> &DfuProtocol<Self::MemoryLayout> {
        &self.protocol
    }

    fn functional_descriptor(&self) -> &dfu_core::functional_descriptor::FunctionalDescriptor {
        &self.descriptor
    }
}
