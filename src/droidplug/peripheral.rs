use crate::{
    api::{
        self, BDAddr, Characteristic, Descriptor, PeripheralProperties, Service, ValueNotification,
        WriteType,
    },
    common::adapter_manager::AdapterManager,
    Error, Result,
};
use async_trait::async_trait;
use futures::stream::Stream;
use jni::{
    objects::{GlobalRef, JByteArray, JList, JObject, JString, JThrowable},
    JNIEnv,
};
use jni_utils::{
    arrays::byte_array_to_vec, exceptions::try_block, future::JSendFuture, stream::JSendStream,
    task::JPollResult, uuid::JUuid,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serde")]
use serde_cr as serde;
use std::{
    collections::BTreeSet,
    convert::TryFrom,
    fmt::{self, Debug, Display, Formatter},
    pin::Pin,
    sync::{Arc, Mutex, Weak},
};

use super::jni::{
    global_jvm,
    objects::{JBluetoothGattCharacteristic, JBluetoothGattService, JPeripheral},
};
use jni::objects::JClass;
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_cr")
)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PeripheralId(pub(super) BDAddr);
impl Display for PeripheralId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

fn map_future_exception<'a>(
    env: &mut JNIEnv<'a>,
    ex: JThrowable<'a>,
) -> std::result::Result<Error, jni::errors::Error> {
    let ex_obj: JObject = ex.into();
    let cause = env
        .call_method(&ex_obj, "getCause", "()Ljava/lang/Throwable;", &[])?
        .l()?;
    if env.is_instance_of(
        &cause,
        <&JClass>::from(
            jni_utils::classcache::get_class(
                "com/nonpolynomial/btleplug/android/impl/NotConnectedException",
            )
            .unwrap()
            .as_obj(),
        ),
    )? {
        Ok(Error::NotConnected)
    } else if env.is_instance_of(
        &cause,
        <&JClass>::from(
            jni_utils::classcache::get_class(
                "com/nonpolynomial/btleplug/android/impl/PermissionDeniedException",
            )
            .unwrap()
            .as_obj(),
        ),
    )? {
        Ok(Error::PermissionDenied)
    } else if env.is_instance_of(
        &cause,
        <&JClass>::from(
            jni_utils::classcache::get_class(
                "com/nonpolynomial/btleplug/android/impl/UnexpectedCallbackException",
            )
            .unwrap()
            .as_obj(),
        ),
    )? {
        Ok(Error::UnexpectedCallback)
    } else if env.is_instance_of(
        &cause,
        <&JClass>::from(
            jni_utils::classcache::get_class(
                "com/nonpolynomial/btleplug/android/impl/UnexpectedCharacteristicException",
            )
            .unwrap()
            .as_obj(),
        ),
    )? {
        Ok(Error::UnexpectedCharacteristic)
    } else if env.is_instance_of(
        &cause,
        <&JClass>::from(
            jni_utils::classcache::get_class(
                "com/nonpolynomial/btleplug/android/impl/NoSuchCharacteristicException",
            )
            .unwrap()
            .as_obj(),
        ),
    )? {
        Ok(Error::NoSuchCharacteristic)
    } else if env.is_instance_of(&cause, "java/lang/RuntimeException")? {
        let msg = env
            .call_method(&cause, "getMessage", "()Ljava/lang/String;", &[])?
            .l()?;
        let msg: JString = msg.into();
        let msgstr: String = env.get_string(&msg)?.into();
        Ok(Error::RuntimeError(msgstr))
    } else {
        env.throw(JThrowable::from(ex_obj))?;
        Err(jni::errors::Error::JavaException)
    }
}

fn check_pending_exception(env: &mut JNIEnv) -> Result<()> {
    if !env.exception_check()? {
        return Ok(());
    }
    let ex = env.exception_occurred()?;
    env.exception_clear()?;
    if env.is_instance_of(
        &ex,
        <&JClass>::from(
            jni_utils::classcache::get_class("io/github/gedgygedgy/rust/future/FutureException")
                .unwrap()
                .as_obj(),
        ),
    )? {
        let err = map_future_exception(env, ex).map_err(Into::<Error>::into)?;
        return Err(err);
    }
    env.throw(ex)?;
    Err(jni::errors::Error::JavaException.into())
}

fn poll_result_from_future<'a>(
    env: &mut JNIEnv<'a>,
    result_ref: &GlobalRef,
) -> Result<JPollResult<'a>> {
    check_pending_exception(env)?;
    let result_obj = match env.new_local_ref(result_ref.as_obj()) {
        Ok(obj) => obj,
        Err(jni::errors::Error::JavaException) => {
            let ex = env.exception_occurred()?;
            env.exception_clear()?;
            return if env.is_instance_of(
                &ex,
                <&JClass>::from(
                    jni_utils::classcache::get_class("io/github/gedgygedgy/rust/future/FutureException")
                        .unwrap()
                        .as_obj(),
                ),
            )? {
                Err(map_future_exception(env, ex)?)
            } else {
                env.throw(ex)?;
                Err(jni::errors::Error::JavaException.into())
            };
        }
        Err(err) => return Err(err.into()),
    };
    Ok(JPollResult::from_env(env, result_obj)?)
}

fn get_poll_result<'a>(env: &mut JNIEnv<'a>, result: JPollResult<'a>) -> Result<JObject<'a>> {
    match result.get() {
        Ok(obj) => Ok(obj),
        Err(jni::errors::Error::JavaException) => {
            let ex = env.exception_occurred()?;
            env.exception_clear()?;
            if env.is_instance_of(
                &ex,
                <&JClass>::from(
                    jni_utils::classcache::get_class(
                        "io/github/gedgygedgy/rust/future/FutureException",
                    )
                    .unwrap()
                    .as_obj(),
                ),
            )? {
                Err(map_future_exception(env, ex)?)
            } else {
                env.throw(ex)?;
                Err(jni::errors::Error::JavaException.into())
            }
        }
        Err(err) => Err(err.into()),
    }
}

#[derive(Debug)]
struct PeripheralShared {
    services: BTreeSet<Service>,
    characteristics: BTreeSet<Characteristic>,
    properties: Option<PeripheralProperties>,
}

#[derive(Clone)]
pub struct Peripheral {
    addr: BDAddr,
    internal: GlobalRef,
    adapter: Weak<AdapterManager<Peripheral>>,
    shared: Arc<Mutex<PeripheralShared>>,
}

impl Peripheral {
    pub(crate) fn new<'a>(
        env: &mut JNIEnv<'a>,
        adapter: JObject<'a>,
        addr: BDAddr,
        manager: Weak<AdapterManager<Peripheral>>,
    ) -> Result<Self> {
        let obj = JPeripheral::new(env, adapter, addr)?;
        let obj_ref: JObject = obj.into();
        Ok(Self {
            addr,
            internal: env.new_global_ref(&obj_ref)?,
            adapter: manager,
            shared: Arc::new(Mutex::new(PeripheralShared {
                services: BTreeSet::new(),
                characteristics: BTreeSet::new(),
                properties: None,
            })),
        })
    }

    pub(crate) fn report_properties(&self, properties: PeripheralProperties) {
        let mut guard = self.shared.lock().unwrap();

        guard.properties = Some(properties);
    }

    fn with_obj<T, E>(
        &self,
        f: impl for<'a> FnOnce(&mut JNIEnv<'a>, JPeripheral<'a>) -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E>
    where
        E: From<::jni::errors::Error>,
    {
        let mut env = global_jvm().get_env()?;
        if env.exception_check()? {
            env.exception_clear()?;
            return Err(::jni::errors::Error::JavaException.into());
        }
        let obj = env.new_local_ref(self.internal.as_obj())?;
        if env.exception_check()? {
            env.exception_clear()?;
            return Err(::jni::errors::Error::JavaException.into());
        }
        let obj = JPeripheral::from_env(&mut env, obj)?;
        f(&mut env, obj)
    }

    async fn set_characteristic_notification(
        &self,
        characteristic: &Characteristic,
        enable: bool,
    ) -> Result<()> {
        self.ensure_available()?;
        let future = self.with_obj(|env, obj| {
            let uuid_obj = JUuid::new(env, characteristic.uuid)?;
            JSendFuture::try_from(obj.set_characteristic_notification(uuid_obj, enable)?)
        })?;
        let result_ref = future.await?;
        let mut env = global_jvm().get_env()?;
        let result = poll_result_from_future(&mut env, &result_ref)?;
        get_poll_result(&mut env, result).map(|_| {})
    }

    fn ensure_available(&self) -> Result<()> {
        let manager = self.adapter.upgrade().ok_or(Error::DeviceNotFound)?;
        let id = PeripheralId(self.addr);
        if manager.peripheral(&id).is_some() {
            Ok(())
        } else {
            Err(Error::DeviceNotFound)
        }
    }
}

impl Debug for Peripheral {
    fn fmt(&self, fmt: &mut Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "{:?}", self.internal.as_obj())
    }
}

#[async_trait]
impl api::Peripheral for Peripheral {
    /// Returns the unique identifier of the peripheral.
    fn id(&self) -> PeripheralId {
        PeripheralId(self.addr)
    }

    fn address(&self) -> BDAddr {
        self.addr
    }

    async fn properties(&self) -> Result<Option<PeripheralProperties>> {
        let guard = self.shared.lock().map_err(Into::<Error>::into)?;
        Ok((&guard.properties).clone())
    }

    fn characteristics(&self) -> BTreeSet<Characteristic> {
        let guard = self.shared.lock().unwrap();
        (&guard.characteristics).clone()
    }

    async fn is_connected(&self) -> Result<bool> {
        self.ensure_available()?;
        self.with_obj(|_env, obj| Ok(obj.is_connected()?))
    }

    async fn mtu(&self, _characteristics: Option<&[Characteristic]>) -> Result<u16> {
        self.ensure_available()?;
        self.with_obj(|env, obj| {
            let result = try_block(env, |_env| Ok(Ok(obj.get_mtu()?)))
                .catch(
                    <&JClass>::from(
                        jni_utils::classcache::get_class(
                            "com/nonpolynomial/btleplug/android/impl/NotConnectedException",
                        )
                        .unwrap()
                        .as_obj(),
                    ),
                    |_env, _ex| Ok(Err(Error::NotConnected)),
                )
                .result()?
                .map_err(Into::<Error>::into)?;
            u16::try_from(result).map_err(|_| Error::Other("MTU conversion failed".into()))
        })
    }

    async fn connect(&self) -> Result<()> {
        self.ensure_available()?;
        let future = self.with_obj(|_env, obj| JSendFuture::try_from(obj.connect()?))?;
        let result_ref = future.await?;
        let mut env = global_jvm().get_env()?;
        let result = poll_result_from_future(&mut env, &result_ref)?;
        get_poll_result(&mut env, result).map(|_| {})
    }

    async fn disconnect(&self) -> Result<()> {
        self.ensure_available()?;
        let future = self.with_obj(|_env, obj| JSendFuture::try_from(obj.disconnect()?))?;
        let result_ref = future.await?;
        let mut env = global_jvm().get_env()?;
        let result = poll_result_from_future(&mut env, &result_ref)?;
        get_poll_result(&mut env, result).map(|_| {})
    }

    /// The set of services we've discovered for this device. This will be empty until
    /// `discover_services` is called.
    fn services(&self) -> BTreeSet<Service> {
        let guard = self.shared.lock().unwrap();
        (&guard.services).clone()
    }

    async fn discover_services(&self) -> Result<()> {
        self.ensure_available()?;
        let future = self.with_obj(|_env, obj| JSendFuture::try_from(obj.discover_services()?))?;
        let result_ref = future.await?;
        let mut env = global_jvm().get_env()?;
        use std::iter::FromIterator;

        let result = poll_result_from_future(&mut env, &result_ref)?;
        let obj = get_poll_result(&mut env, result)?;
        let list = JList::from_env(&mut env, &obj)?;
        let mut peripheral_services = Vec::new();
        let mut peripheral_characteristics = Vec::new();

        let mut iter = list.iter(&mut env)?;
        while let Some(service) = iter.next(&mut env)? {
            let service = JBluetoothGattService::from_env(&mut env, service)?;
            let mut characteristics = BTreeSet::<Characteristic>::new();
            for characteristic in service.get_characteristics()? {
                let mut descriptors = BTreeSet::new();
                for descriptor in characteristic.get_descriptors()? {
                    descriptors.insert(Descriptor {
                        uuid: descriptor.get_uuid()?,
                        service_uuid: service.get_uuid()?,
                        characteristic_uuid: characteristic.get_uuid()?,
                    });
                }
                let char = Characteristic {
                    service_uuid: service.get_uuid()?,
                    uuid: characteristic.get_uuid()?,
                    properties: characteristic.get_properties()?,
                    descriptors: descriptors.clone(),
                };
                // Only consider the first characteristic of each UUID
                // This "should" be unique, but of course it's not enforced
                if characteristics
                    .iter()
                    .filter(|c| c.service_uuid == char.service_uuid && c.uuid == char.uuid)
                    .count()
                    == 0
                {
                    characteristics.insert(char.clone());
                    peripheral_characteristics.push(char.clone());
                }
            }
            peripheral_services.push(Service {
                uuid: service.get_uuid()?,
                primary: service.is_primary()?,
                characteristics,
            })
        }
        let mut guard = self.shared.lock().map_err(Into::<Error>::into)?;
        guard.services = BTreeSet::from_iter(peripheral_services.clone());
        guard.characteristics = BTreeSet::from_iter(peripheral_characteristics.clone());
        Ok(())
    }

    async fn write(
        &self,
        characteristic: &Characteristic,
        data: &[u8],
        write_type: WriteType,
    ) -> Result<()> {
        self.ensure_available()?;
        let future = self.with_obj(|env, obj| {
            let mut local_env = unsafe { env.unsafe_clone() };
            let uuid = JUuid::new(&mut local_env, characteristic.uuid)?;
            let data_obj = jni_utils::arrays::slice_to_byte_array(&mut local_env, data)?;
            let write_type = match write_type {
                WriteType::WithResponse => 2,
                WriteType::WithoutResponse => 1,
            };
            JSendFuture::try_from(obj.write(uuid, data_obj.into(), write_type)?)
        })?;
        let result_ref = future.await?;
        let mut env = global_jvm().get_env()?;
        let result = poll_result_from_future(&mut env, &result_ref)?;
        get_poll_result(&mut env, result).map(|_| {})
    }

    async fn read(&self, characteristic: &Characteristic) -> Result<Vec<u8>> {
        self.ensure_available()?;
        let future = self.with_obj(|env, obj| {
            let uuid = JUuid::new(env, characteristic.uuid)?;
            JSendFuture::try_from(obj.read(uuid)?)
        })?;
        let result_ref = future.await?;
        let mut env = global_jvm().get_env()?;
        let result = poll_result_from_future(&mut env, &result_ref)?;
        let bytes = get_poll_result(&mut env, result)?;
        let mut local_env = unsafe { env.unsafe_clone() };
        Ok(byte_array_to_vec(&mut local_env, JByteArray::from(bytes))?)
    }

    async fn subscribe(&self, characteristic: &Characteristic) -> Result<()> {
        self.ensure_available()?;
        self.set_characteristic_notification(characteristic, true)
            .await
    }

    async fn unsubscribe(&self, characteristic: &Characteristic) -> Result<()> {
        self.ensure_available()?;
        self.set_characteristic_notification(characteristic, false)
            .await
    }

    async fn notifications(&self) -> Result<Pin<Box<dyn Stream<Item = ValueNotification> + Send>>> {
        use futures::stream::StreamExt;
        let stream = self.with_obj(|_env, obj| JSendStream::try_from(obj.get_notifications()?))?;
        let stream = stream
            .map(|item| match item {
                Ok(item) => {
                    let mut env = global_jvm().get_env()?;
                    let item = env.new_local_ref(item.as_obj())?;
                    let characteristic = JBluetoothGattCharacteristic::from_env(&mut env, item)?;
                    let uuid = characteristic.get_uuid()?;
                    let value = characteristic.get_value()?;
                    Ok(ValueNotification { uuid, value })
                }
                Err(err) => Err(err),
            })
            .filter_map(|item| async { item.ok() });
        Ok(Box::pin(stream))
    }

    async fn write_descriptor(&self, descriptor: &Descriptor, data: &[u8]) -> Result<()> {
        self.ensure_available()?;
        let future = self.with_obj(|env, obj| {
            let mut local_env = unsafe { env.unsafe_clone() };
            let characteristic = JUuid::new(&mut local_env, descriptor.characteristic_uuid)?;
            let uuid = JUuid::new(&mut local_env, descriptor.uuid)?;
            let data_obj = jni_utils::arrays::slice_to_byte_array(&mut local_env, data)?;
            JSendFuture::try_from(obj.write_descriptor(characteristic, uuid, data_obj.into())?)
        })?;
        let result_ref = future.await?;
        let mut env = global_jvm().get_env()?;
        let result = poll_result_from_future(&mut env, &result_ref)?;
        get_poll_result(&mut env, result).map(|_| {})
    }

    async fn read_descriptor(&self, descriptor: &Descriptor) -> Result<Vec<u8>> {
        self.ensure_available()?;
        let future = self.with_obj(|env, obj| {
            let characteristic = JUuid::new(env, descriptor.characteristic_uuid)?;
            let uuid = JUuid::new(env, descriptor.uuid)?;
            JSendFuture::try_from(obj.read_descriptor(characteristic, uuid)?)
        })?;
        let result_ref = future.await?;
        let mut env = global_jvm().get_env()?;
        let result = poll_result_from_future(&mut env, &result_ref)?;
        let bytes = get_poll_result(&mut env, result)?;
        let mut local_env = unsafe { env.unsafe_clone() };
        Ok(byte_array_to_vec(&mut local_env, JByteArray::from(bytes))?)
    }
}
