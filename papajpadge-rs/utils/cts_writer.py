import asyncio
from bleak import BleakClient, BleakScanner
from datetime import datetime
import struct

async def write_current_time():
    # Scan for devices
    devices = await BleakScanner.discover()
    target_device = None
    
    for device in devices:
        if "1805" in str(device.metadata.get("uuids", [])):
            target_device = device
            break
    
    if not target_device:
        print("No device with Current Time Service found")
        return
    
    async with BleakClient(target_device.address) as client:
        # Current Time Service UUID: 0x1805
        # Current Time Characteristic UUID: 0x2A2B
        
        now = datetime.now()
        # Pack time data according to BLE spec
        time_data = struct.pack('<HBBBBBBBB',
            now.year,
            now.month,
            now.day,
            now.hour,
            now.minute,
            now.second,
            now.weekday() + 1,
            int((now.microsecond / 1000000) * 256),
            0  # Adjust reason
        )
        
        await client.write_gatt_char("00002A2B-0000-1000-8000-00805F9B34FB", time_data)
        print(f"Time written: {now}")