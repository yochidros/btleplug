use jni::{
    errors::Result,
    objects::{JByteArray, JClass, JList, JMap, JMethodID, JObject, JString, JValue},
    signature::{Primitive, ReturnType},
    strings::JavaStr,
    sys::jint,
    JNIEnv,
};
use jni_utils::{future::JFuture, stream::JStream, uuid::JUuid};
use std::{collections::HashMap, convert::TryFrom, iter::Iterator};
use uuid::Uuid;

use crate::api::{BDAddr, CharPropFlags, PeripheralProperties, ScanFilter};

pub struct JPeripheral<'a> {
    internal: JObject<'a>,
    connect: JMethodID,
    disconnect: JMethodID,
    is_connected: JMethodID,
    discover_services: JMethodID,
    read: JMethodID,
    write: JMethodID,
    set_characteristic_notification: JMethodID,
    get_notifications: JMethodID,
    read_descriptor: JMethodID,
    write_descriptor: JMethodID,
    env: JNIEnv<'a>,
}

impl<'a> ::std::ops::Deref for JPeripheral<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a> From<JPeripheral<'a>> for JObject<'a> {
    fn from(other: JPeripheral<'a>) -> JObject<'a> {
        other.internal
    }
}

impl<'a> JPeripheral<'a> {
    pub fn from_env(env: &mut JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        //Self::from_env_impl(env, obj)
        //let class = env.find_class("com/nonpolynomial/btleplug/android/impl/Peripheral")?;
        //Self::from_env_impl(env, obj, class)
        Self::from_env_impl(env, obj)
    }

    fn from_env_impl(env: &mut JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        //let class = env.auto_local(class);
        let class_static =
            jni_utils::classcache::get_class("com/nonpolynomial/btleplug/android/impl/Peripheral")
                .unwrap();
        let class = <&JClass>::from(class_static.as_obj());

        let connect = env.get_method_id(
            class,
            "connect",
            "()Lio/github/gedgygedgy/rust/future/Future;",
        )?;
        let disconnect = env.get_method_id(
            class,
            "disconnect",
            "()Lio/github/gedgygedgy/rust/future/Future;",
        )?;
        let is_connected = env.get_method_id(class, "isConnected", "()Z")?;
        let discover_services = env.get_method_id(
            class,
            "discoverServices",
            "()Lio/github/gedgygedgy/rust/future/Future;",
        )?;
        let read = env.get_method_id(
            class,
            "read",
            "(Ljava/util/UUID;)Lio/github/gedgygedgy/rust/future/Future;",
        )?;
        let write = env.get_method_id(
            class,
            "write",
            "(Ljava/util/UUID;[BI)Lio/github/gedgygedgy/rust/future/Future;",
        )?;
        let set_characteristic_notification = env.get_method_id(
            class,
            "setCharacteristicNotification",
            "(Ljava/util/UUID;Z)Lio/github/gedgygedgy/rust/future/Future;",
        )?;
        let get_notifications = env.get_method_id(
            class,
            "getNotifications",
            "()Lio/github/gedgygedgy/rust/stream/Stream;",
        )?;
        let read_descriptor = env.get_method_id(
            class,
            "readDescriptor",
            "(Ljava/util/UUID;Ljava/util/UUID;)Lio/github/gedgygedgy/rust/future/Future;",
        )?;
        let write_descriptor = env.get_method_id(
            class,
            "writeDescriptor",
            "(Ljava/util/UUID;Ljava/util/UUID;[BI)Lio/github/gedgygedgy/rust/future/Future;",
        )?;
        Ok(Self {
            internal: obj,
            connect,
            disconnect,
            is_connected,
            discover_services,
            read,
            write,
            set_characteristic_notification,
            get_notifications,
            read_descriptor,
            write_descriptor,
            env: unsafe { env.unsafe_clone() },
        })
    }

    pub fn new(env: &mut JNIEnv<'a>, adapter: JObject<'a>, addr: BDAddr) -> Result<Self> {
        // let class = env.find_class("com/nonpolynomial/btleplug/android/impl/Peripheral")?;
        let addr_jstr = env.new_string(format!("{:X}", addr))?;
        let obj = env.new_object(
            <&JClass>::from(
                jni_utils::classcache::get_class(
                    "com/nonpolynomial/btleplug/android/impl/Peripheral",
                )
                .unwrap()
                .as_obj(),
            ),
            //class.as_obj(),
            "(Lcom/nonpolynomial/btleplug/android/impl/Adapter;Ljava/lang/String;)V",
            &[JValue::from(&adapter), JValue::from(&addr_jstr)],
        )?;
        //Self::from_env_impl(env, obj, class)
        Self::from_env_impl(env, obj)
    }

    pub fn connect(&self) -> Result<JFuture<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let future_obj = unsafe {
            env.call_method_unchecked(&self.internal, self.connect, ReturnType::Object, &[])
        }?
        .l()?;
        JFuture::from_env(&mut env, future_obj)
    }

    pub fn disconnect(&self) -> Result<JFuture<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let future_obj = unsafe {
            env.call_method_unchecked(&self.internal, self.disconnect, ReturnType::Object, &[])
        }?
        .l()?;
        let mut env = unsafe { self.env.unsafe_clone() };
        JFuture::from_env(&mut env, future_obj)
    }

    pub fn is_connected(&self) -> Result<bool> {
        let mut env = unsafe { self.env.unsafe_clone() };
        unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.is_connected,
                ReturnType::Primitive(Primitive::Boolean),
                &[],
            )
        }?
        .z()
    }

    pub fn discover_services(&self) -> Result<JFuture<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let future_obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.discover_services,
                ReturnType::Object,
                &[],
            )
        }?
        .l()?;
        JFuture::from_env(&mut env, future_obj)
    }

    pub fn read(&self, uuid: JUuid<'a>) -> Result<JFuture<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let uuid_obj: JObject = uuid.into();
        let args = [JValue::from(&uuid_obj).as_jni()];
        let future_obj = unsafe {
            env.call_method_unchecked(&self.internal, self.read, ReturnType::Object, &args)
        }?
        .l()?;
        JFuture::from_env(&mut env, future_obj)
    }

    pub fn write(
        &self,
        uuid: JUuid<'a>,
        data: JObject<'a>,
        write_type: jint,
    ) -> Result<JFuture<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let uuid_obj: JObject = uuid.into();
        let args = [
            JValue::from(&uuid_obj).as_jni(),
            JValue::from(&data).as_jni(),
            JValue::from(write_type).as_jni(),
        ];
        let future_obj = unsafe {
            env.call_method_unchecked(&self.internal, self.write, ReturnType::Object, &args)
        }?
        .l()?;
        JFuture::from_env(&mut env, future_obj)
    }

    pub fn set_characteristic_notification(
        &self,
        uuid: JUuid<'a>,
        enable: bool,
    ) -> Result<JFuture<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let uuid_obj: JObject = uuid.into();
        let args = [
            JValue::from(&uuid_obj).as_jni(),
            JValue::from(enable).as_jni(),
        ];
        let future_obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.set_characteristic_notification,
                ReturnType::Object,
                &args,
            )
        }?
        .l()?;
        JFuture::from_env(&mut env, future_obj)
    }

    pub fn get_notifications(&self) -> Result<JStream<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let stream_obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_notifications,
                ReturnType::Object,
                &[],
            )
        }?
        .l()?;
        JStream::from_env(&mut env, stream_obj)
    }

    pub fn read_descriptor(
        &self,
        characteristic: JUuid<'a>,
        uuid: JUuid<'a>,
    ) -> Result<JFuture<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let characteristic_obj: JObject = characteristic.into();
        let uuid_obj: JObject = uuid.into();
        let args = [
            JValue::from(&characteristic_obj).as_jni(),
            JValue::from(&uuid_obj).as_jni(),
        ];
        let future_obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.read_descriptor,
                ReturnType::Object,
                &args,
            )
        }?
        .l()?;
        JFuture::from_env(&mut env, future_obj)
    }

    pub fn write_descriptor(
        &self,
        characteristic: JUuid<'a>,
        uuid: JUuid<'a>,
        data: JObject<'a>,
    ) -> Result<JFuture<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let characteristic_obj: JObject = characteristic.into();
        let uuid_obj: JObject = uuid.into();
        let args = [
            JValue::from(&characteristic_obj).as_jni(),
            JValue::from(&uuid_obj).as_jni(),
            JValue::from(&data).as_jni(),
        ];
        let future_obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.write_descriptor,
                ReturnType::Object,
                &args,
            )
        }?
        .l()?;
        JFuture::from_env(&mut env, future_obj)
    }
}

pub struct JBluetoothGattService<'a> {
    internal: JObject<'a>,
    get_uuid: JMethodID,
    //is_primary: JMethodID<'a>,
    get_characteristics: JMethodID,
    env: JNIEnv<'a>,
}

impl<'a> JBluetoothGattService<'a> {
    pub fn from_env(env: &mut JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.find_class("android/bluetooth/BluetoothGattService")?;
        let class = env.auto_local(class);

        let get_uuid = env.get_method_id(&class, "getUuid", "()Ljava/util/UUID;")?;
        //let is_primary = env.get_method_id(&class, "isPrimary", "()Z;")?;
        let get_characteristics =
            env.get_method_id(&class, "getCharacteristics", "()Ljava/util/List;")?;
        Ok(Self {
            internal: obj,
            get_uuid,
            //is_primary,
            get_characteristics,
            env: unsafe { env.unsafe_clone() },
        })
    }

    pub fn is_primary(&self) -> Result<bool> {
        /*
        self.env
        .call_method_unchecked(
            self.internal,
            self.is_primary,
            JavaType::Primitive(Primitive::Boolean),
            &[],
        )?
        .z()
        */
        Ok(true)
    }

    pub fn get_uuid(&self) -> Result<Uuid> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(&self.internal, self.get_uuid, ReturnType::Object, &[])
        }?
        .l()?;
        let uuid_obj = JUuid::from_env(&mut env, obj)?;
        Ok(uuid_obj.as_uuid()?)
    }

    pub fn get_characteristics(&self) -> Result<Vec<JBluetoothGattCharacteristic>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_characteristics,
                ReturnType::Object,
                &[],
            )
        }?
        .l()?;
        let chr_list = JList::from_env(&mut env, &obj)?;
        let mut chr_vec = vec![];
        let mut iter = chr_list.iter(&mut env)?;
        while let Some(chr) = iter.next(&mut env)? {
            chr_vec.push(JBluetoothGattCharacteristic::from_env(&mut env, chr)?);
        }
        Ok(chr_vec)
    }
}

pub struct JBluetoothGattCharacteristic<'a> {
    internal: JObject<'a>,
    get_uuid: JMethodID,
    get_properties: JMethodID,
    get_value: JMethodID,
    get_descriptors: JMethodID,
    env: JNIEnv<'a>,
}

impl<'a> JBluetoothGattCharacteristic<'a> {
    pub fn from_env(env: &mut JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.find_class("android/bluetooth/BluetoothGattCharacteristic")?;
        let class = env.auto_local(class);

        let get_uuid = env.get_method_id(&class, "getUuid", "()Ljava/util/UUID;")?;
        let get_properties = env.get_method_id(&class, "getProperties", "()I")?;
        let get_descriptors = env.get_method_id(&class, "getDescriptors", "()Ljava/util/List;")?;
        let get_value = env.get_method_id(&class, "getValue", "()[B")?;
        Ok(Self {
            internal: obj,
            get_uuid,
            get_properties,
            get_value,
            get_descriptors,
            env: unsafe { env.unsafe_clone() },
        })
    }

    pub fn get_uuid(&self) -> Result<Uuid> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(&self.internal, self.get_uuid, ReturnType::Object, &[])
        }?
        .l()?;
        let uuid_obj = JUuid::from_env(&mut env, obj)?;
        Ok(uuid_obj.as_uuid()?)
    }

    pub fn get_properties(&self) -> Result<CharPropFlags> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let flags = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_properties,
                ReturnType::Primitive(Primitive::Int),
                &[],
            )
        }?
        .i()?;
        Ok(CharPropFlags::from_bits_truncate(flags as u8))
    }

    pub fn get_value(&self) -> Result<Vec<u8>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let value = unsafe {
            env.call_method_unchecked(&self.internal, self.get_value, ReturnType::Array, &[])
        }?
        .l()?;
        jni_utils::arrays::byte_array_to_vec(&mut env, JByteArray::from(value))
    }

    pub fn get_descriptors(&self) -> Result<Vec<JBluetoothGattDescriptor>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_descriptors,
                ReturnType::Object,
                &[],
            )
        }?
        .l()?;
        let desc_list = JList::from_env(&mut env, &obj)?;
        let mut desc_vec = vec![];
        let mut iter = desc_list.iter(&mut env)?;
        while let Some(desc) = iter.next(&mut env)? {
            desc_vec.push(JBluetoothGattDescriptor::from_env(&mut env, desc)?);
        }
        Ok(desc_vec)
    }
}

pub struct JBluetoothGattDescriptor<'a> {
    internal: JObject<'a>,
    get_uuid: JMethodID,
    env: JNIEnv<'a>,
}

impl<'a> JBluetoothGattDescriptor<'a> {
    pub fn from_env(env: &mut JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.find_class("android/bluetooth/BluetoothGattDescriptor")?;
        let class = env.auto_local(class);

        let get_uuid = env.get_method_id(&class, "getUuid", "()Ljava/util/UUID;")?;
        Ok(Self {
            internal: obj,
            get_uuid,
            env: unsafe { env.unsafe_clone() },
        })
    }

    pub fn get_uuid(&self) -> Result<Uuid> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(&self.internal, self.get_uuid, ReturnType::Object, &[])
        }?
        .l()?;
        let uuid_obj = JUuid::from_env(&mut env, obj)?;
        Ok(uuid_obj.as_uuid()?)
    }
}

pub struct JBluetoothDevice<'a> {
    internal: JObject<'a>,
    get_address: JMethodID,
    env: JNIEnv<'a>,
}

impl<'a> JBluetoothDevice<'a> {
    pub fn from_env(env: &mut JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.find_class("android/bluetooth/BluetoothDevice")?;
        let class = env.auto_local(class);

        let get_address = env.get_method_id(&class, "getAddress", "()Ljava/lang/String;")?;
        Ok(Self {
            internal: obj,
            get_address,
            env: unsafe { env.unsafe_clone() },
        })
    }

    pub fn get_address(&self) -> Result<JString<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(&self.internal, self.get_address, ReturnType::Object, &[])
        }?
        .l()?;
        Ok(obj.into())
    }
}

pub struct JScanFilter<'a> {
    internal: JObject<'a>,
}

impl<'a> JScanFilter<'a> {
    pub fn new(env: &mut JNIEnv<'a>, filter: ScanFilter) -> Result<Self> {
        let string_class = env.find_class("java/lang/String")?;
        let uuids =
            env.new_object_array(filter.services.len() as i32, string_class, JObject::null())?;
        for (idx, uuid) in filter.services.into_iter().enumerate() {
            let uuid_str = env.new_string(uuid.to_string())?;
            env.set_object_array_element(&uuids, idx as i32, uuid_str)?;
        }
        let obj = env.new_object(
            <&JClass>::from(
                jni_utils::classcache::get_class(
                    "com/nonpolynomial/btleplug/android/impl/ScanFilter",
                )
                .unwrap()
                .as_obj(),
            ),
            //class.as_obj(),
            "([Ljava/lang/String;)V",
            &[JValue::from(&uuids)],
        )?;
        Ok(Self { internal: obj })
    }
}

impl<'a> From<JScanFilter<'a>> for JObject<'a> {
    fn from(value: JScanFilter<'a>) -> Self {
        value.internal
    }
}

pub struct JScanResult<'a> {
    internal: JObject<'a>,
    get_device: JMethodID,
    get_scan_record: JMethodID,
    get_tx_power: JMethodID,
    get_rssi: JMethodID,
    env: JNIEnv<'a>,
}

impl<'a> JScanResult<'a> {
    pub fn from_env(env: &mut JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.find_class("android/bluetooth/le/ScanResult")?;
        let class = env.auto_local(class);

        let get_device =
            env.get_method_id(&class, "getDevice", "()Landroid/bluetooth/BluetoothDevice;")?;
        let get_scan_record = env.get_method_id(
            &class,
            "getScanRecord",
            "()Landroid/bluetooth/le/ScanRecord;",
        )?;
        let get_tx_power = env.get_method_id(&class, "getTxPower", "()I")?;
        let get_rssi = env.get_method_id(&class, "getRssi", "()I")?;
        Ok(Self {
            internal: obj,
            get_device,
            get_scan_record,
            get_tx_power,
            get_rssi,
            env: unsafe { env.unsafe_clone() },
        })
    }

    pub fn get_device(&self) -> Result<JBluetoothDevice<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(&self.internal, self.get_device, ReturnType::Object, &[])
        }?
        .l()?;
        JBluetoothDevice::from_env(&mut env, obj)
    }

    pub fn get_scan_record(&self) -> Result<JScanRecord<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_scan_record,
                ReturnType::Object,
                &[],
            )
        }?
        .l()?;
        JScanRecord::from_env(&mut env, obj)
    }

    pub fn get_tx_power(&self) -> Result<jint> {
        let mut env = unsafe { self.env.unsafe_clone() };
        unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_tx_power,
                ReturnType::Primitive(Primitive::Int),
                &[],
            )
        }?
        .i()
    }

    pub fn get_rssi(&self) -> Result<jint> {
        let mut env = unsafe { self.env.unsafe_clone() };
        unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_rssi,
                ReturnType::Primitive(Primitive::Int),
                &[],
            )
        }?
        .i()
    }
}

impl<'a> TryFrom<JScanResult<'a>> for (BDAddr, Option<PeripheralProperties>) {
    type Error = crate::Error;

    fn try_from(result: JScanResult<'a>) -> std::result::Result<Self, Self::Error> {
        use std::str::FromStr;

        let device = result.get_device()?;

        let addr_obj = device.get_address()?;
        let addr_str = JavaStr::from_env(&result.env, &addr_obj)?;
        let addr = BDAddr::from_str(
            addr_str
                .to_str()
                .map_err(|e| Self::Error::Other(e.into()))?,
        )?;

        let record = result.get_scan_record()?;
        let record_obj: &JObject = &record;
        let properties = if result.env.is_same_object(record_obj, JObject::null())? {
            None
        } else {
            let device_name_obj = record.get_device_name()?;
            let device_name = if result
                .env
                .is_same_object(&device_name_obj, JObject::null())?
            {
                None
            } else {
                let device_name_str = JavaStr::from_env(&result.env, &device_name_obj)?;
                // On Android, there is a chance that a device name may not actually be valid UTF-8.
                // We're given the full buffer, regardless of if it's just UTF-8 characters,
                // possibly c str with null characters, or whatever. We should try UTF-8 first, if
                // that doesn't work out, see if there's a null termination character in it and try
                // parsing that.
                Some(
                    String::from_utf8_lossy(device_name_str.to_bytes())
                        .chars()
                        .filter(|&c| c != '\u{fffd}')
                        .collect(),
                )
            };

            let tx_power_level = result.get_tx_power()?;
            const TX_POWER_NOT_PRESENT: jint = 127; // from ScanResult documentation
            let tx_power_level = if tx_power_level == TX_POWER_NOT_PRESENT {
                None
            } else {
                Some(tx_power_level as i16)
            };

            let rssi = Some(result.get_rssi()? as i16);
            let raw_bytes = {
                let arr = record.get_bytes()?;
                result.env.convert_byte_array(arr)?
            };
            // parse AD structure here if needed
            let mut index = 0;
            let mut manufacturer_data: HashMap<u16, Vec<u8>> = HashMap::new();

            while index < raw_bytes.len() {
                let length = raw_bytes[index] as usize;
                if length == 0 {
                    break;
                }

                if index + length >= raw_bytes.len() {
                    break;
                }

                let ad_type = raw_bytes[index + 1] as u8;
                if ad_type == 0xFF {
                    // Manufacturer Specific Data
                    let company_id =
                        ((raw_bytes[index + 3] as u16) << 8) | (raw_bytes[index + 2] as u16);

                    let data_start = index + 4;
                    let data_end = index + 1 + length;
                    if data_end <= raw_bytes.len() {
                        let data = raw_bytes[data_start..data_end].to_vec();

                        manufacturer_data
                            .entry(company_id)
                            .and_modify(|v| v.extend_from_slice(&data))
                            .or_insert(data);
                    }
                }

                index += length + 1;
            }

            // let manufacturer_specific_data_array = record.get_manufacturer_specific_data()?;
            // let manufacturer_specific_data_obj: &JObject = &manufacturer_specific_data_array;
            // let mut manufacturer_data = HashMap::new();
            // if !result
            //     .env
            //     .is_same_object(manufacturer_specific_data_obj.clone(), JObject::null())?
            // {
            //     for item in manufacturer_specific_data_array.iter() {
            //         let (index, data) = item?;
            //
            //         let index = index as u16;
            //         let data = jni_utils::arrays::byte_array_to_vec(result.env, data.into_inner())?;
            //         manufacturer_data.insert(index, data);
            //     }
            // }

            let service_data_obj = record.get_service_data()?;
            let mut service_data = HashMap::new();
            if !result
                .env
                .is_same_object(&service_data_obj, JObject::null())?
            {
                let mut iter_env = unsafe { result.env.unsafe_clone() };
                let service_data_map = JMap::from_env(&mut iter_env, &service_data_obj)?;
                let mut iter = service_data_map.iter(&mut iter_env)?;
                while let Some((key, value)) = iter.next(&mut iter_env)? {
                    let mut item_env = unsafe { result.env.unsafe_clone() };
                    let uuid = JParcelUuid::from_env(&mut item_env, key)?
                        .get_uuid()?
                        .as_uuid()?;
                    let data = jni_utils::arrays::byte_array_to_vec(
                        &mut item_env,
                        JByteArray::from(value),
                    )?;
                    service_data.insert(uuid, data);
                }
            }

            let services_obj = record.get_service_uuids()?;
            let mut services = Vec::new();
            if !result.env.is_same_object(&services_obj, JObject::null())? {
                let mut iter_env = unsafe { result.env.unsafe_clone() };
                let services_list = JList::from_env(&mut iter_env, &services_obj)?;
                let mut iter = services_list.iter(&mut iter_env)?;
                while let Some(obj) = iter.next(&mut iter_env)? {
                    let mut item_env = unsafe { result.env.unsafe_clone() };
                    let uuid = JParcelUuid::from_env(&mut item_env, obj)?
                        .get_uuid()?
                        .as_uuid()?;
                    services.push(uuid);
                }
            }

            Some(PeripheralProperties {
                address: addr,
                address_type: None,
                local_name: device_name,
                tx_power_level,
                manufacturer_data,
                service_data,
                services,
                rssi,
                class: None,
            })
        };
        Ok((addr, properties))
    }
}

pub struct JScanRecord<'a> {
    internal: JObject<'a>,
    get_device_name: JMethodID,
    get_tx_power_level: JMethodID,
    get_manufacturer_specific_data: JMethodID,
    get_bytes: JMethodID,
    get_service_data: JMethodID,
    get_service_uuids: JMethodID,
    env: JNIEnv<'a>,
}

impl<'a> From<JScanRecord<'a>> for JObject<'a> {
    fn from(scan_record: JScanRecord<'a>) -> Self {
        scan_record.internal
    }
}

impl<'a> ::std::ops::Deref for JScanRecord<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a> JScanRecord<'a> {
    pub fn from_env(env: &mut JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.find_class("android/bluetooth/le/ScanRecord")?;
        let class = env.auto_local(class);

        let get_device_name = env.get_method_id(&class, "getDeviceName", "()Ljava/lang/String;")?;
        let get_tx_power_level = env.get_method_id(&class, "getTxPowerLevel", "()I")?;
        let get_manufacturer_specific_data = env.get_method_id(
            &class,
            "getManufacturerSpecificData",
            "()Landroid/util/SparseArray;",
        )?;
        let get_service_data = env.get_method_id(&class, "getServiceData", "()Ljava/util/Map;")?;
        let get_service_uuids =
            env.get_method_id(&class, "getServiceUuids", "()Ljava/util/List;")?;

        let get_bytes = env.get_method_id(&class, "getBytes", "()[B")?;
        Ok(Self {
            internal: obj,
            get_device_name,
            get_tx_power_level,
            get_manufacturer_specific_data,
            get_bytes,
            get_service_data,
            get_service_uuids,
            env: unsafe { env.unsafe_clone() },
        })
    }

    pub fn get_device_name(&self) -> Result<JString<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_device_name,
                ReturnType::Object,
                &[],
            )
        }?
        .l()?;
        Ok(obj.into())
    }

    pub fn get_tx_power_level(&self) -> Result<jint> {
        let mut env = unsafe { self.env.unsafe_clone() };
        unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_tx_power_level,
                ReturnType::Primitive(Primitive::Int),
                &[],
            )
        }?
        .i()
    }

    pub fn get_manufacturer_specific_data(&self) -> Result<JSparseArray<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_manufacturer_specific_data,
                ReturnType::Object,
                &[],
            )
        }?
        .l()?;
        JSparseArray::from_env(&mut env, obj)
    }
    pub fn get_bytes(&self) -> Result<JByteArray<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(&self.internal, self.get_bytes, ReturnType::Array, &[])
        }?
        .l()?;
        Ok(JByteArray::from(obj))
    }

    pub fn get_service_data(&self) -> Result<JObject<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_service_data,
                ReturnType::Object,
                &[],
            )
        }?
        .l()?;
        Ok(obj)
    }

    pub fn get_service_uuids(&self) -> Result<JObject<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.get_service_uuids,
                ReturnType::Object,
                &[],
            )
        }?
        .l()?;
        Ok(obj)
    }
}

pub struct JSparseArray<'a> {
    internal: JObject<'a>,
    size: JMethodID,
    key_at: JMethodID,
    value_at: JMethodID,
    env: JNIEnv<'a>,
}

impl<'a> From<JSparseArray<'a>> for JObject<'a> {
    fn from(sparse_array: JSparseArray<'a>) -> Self {
        sparse_array.internal
    }
}

impl<'a> ::std::ops::Deref for JSparseArray<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a> JSparseArray<'a> {
    pub fn from_env(env: &mut JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.find_class("android/util/SparseArray")?;
        let class = env.auto_local(class);

        let size = env.get_method_id(&class, "size", "()I")?;
        let key_at = env.get_method_id(&class, "keyAt", "(I)I")?;
        let value_at = env.get_method_id(&class, "valueAt", "(I)Ljava/lang/Object;")?;
        Ok(Self {
            internal: obj,
            size,
            key_at,
            value_at,
            env: unsafe { env.unsafe_clone() },
        })
    }

    pub fn size(&self) -> Result<jint> {
        let mut env = unsafe { self.env.unsafe_clone() };
        unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.size,
                ReturnType::Primitive(Primitive::Int),
                &[],
            )
        }?
        .i()
    }

    pub fn key_at(&self, index: jint) -> Result<jint> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let args = [JValue::from(index).as_jni()];
        unsafe {
            env.call_method_unchecked(
                &self.internal,
                self.key_at,
                ReturnType::Primitive(Primitive::Int),
                &args,
            )
        }?
        .i()
    }

    pub fn value_at(&self, index: jint) -> Result<JObject<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let args = [JValue::from(index).as_jni()];
        unsafe {
            env.call_method_unchecked(&self.internal, self.value_at, ReturnType::Object, &args)
        }?
        .l()
    }

    pub fn iter(&self) -> JSparseArrayIter<'a, '_> {
        JSparseArrayIter {
            internal: self,
            index: 0,
        }
    }
}

pub struct JSparseArrayIter<'a, 'b> {
    internal: &'b JSparseArray<'a>,
    index: jint,
}

impl<'a, 'b> JSparseArrayIter<'a, 'b> {
    fn next_internal(&mut self) -> Result<Option<(jint, JObject<'a>)>> {
        let size = self.internal.size()?;
        Ok(if self.index >= size {
            None
        } else {
            let key = self.internal.key_at(self.index)?;
            let value = self.internal.value_at(self.index)?;
            self.index += 1;
            Some((key, value))
        })
    }
}

impl<'a, 'b> Iterator for JSparseArrayIter<'a, 'b> {
    type Item = Result<(jint, JObject<'a>)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_internal().transpose()
    }
}
pub struct JParcelUuid<'a> {
    internal: JObject<'a>,
    get_uuid: JMethodID,
    env: JNIEnv<'a>,
}

impl<'a> JParcelUuid<'a> {
    pub fn from_env(env: &mut JNIEnv<'a>, obj: JObject<'a>) -> Result<Self> {
        let class = env.find_class("android/os/ParcelUuid")?;
        let class = env.auto_local(class);

        let get_uuid = env.get_method_id(&class, "getUuid", "()Ljava/util/UUID;")?;
        Ok(Self {
            internal: obj,
            get_uuid,
            env: unsafe { env.unsafe_clone() },
        })
    }

    pub fn get_uuid(&self) -> Result<JUuid<'a>> {
        let mut env = unsafe { self.env.unsafe_clone() };
        let obj = unsafe {
            env.call_method_unchecked(&self.internal, self.get_uuid, ReturnType::Object, &[])
        }?
        .l()?;
        JUuid::from_env(&mut env, obj)
    }
}
