use cross_usb::device_filter;

fn main() {
    println!("This example demonstrates DFU download usage. See the async functions below.");
}

pub async fn dfu_download_example() -> Result<(), dfu_cross_usb::Error> {
    let filters = vec![device_filter! {vendor_id: 0xcafe, product_id: 0xdead}];
    let dev_info = cross_usb::get_device(filters)
        .await
        .map_err(|_| dfu_cross_usb::Error::DeviceNotFound)?;

    let dfu_device = dfu_cross_usb::DfuCrossUsb::open(dev_info, 0, 0).await?;
    let mut dfu = dfu_device.into_async_dfu();

    let data: &[u8] = &[0u8, 1, 2];
    dfu.download(data, 3).await?;

    Ok(())
}
