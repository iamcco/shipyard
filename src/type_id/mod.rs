mod hasher;

pub(crate) use hasher::TypeIdHasher;

use core::hash::{Hash, Hasher};

/// Custom `TypeId` to be able to deserialize it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]

pub struct TypeId(pub(crate) u32);

impl TypeId {
    pub(crate) fn of<T: ?Sized + 'static>() -> Self {
        core::any::TypeId::of::<T>().into()
    }
    #[cfg(test)]
    pub(crate) fn of_val<T: ?Sized + 'static>(_: &T) -> TypeId {
        core::any::TypeId::of::<T>().into()
    }
}

impl From<core::any::TypeId> for TypeId {
    fn from(type_id: core::any::TypeId) -> Self {
        let mut hasher = TypeIdHasher::default();

        type_id.hash(&mut hasher);

        TypeId(hasher.finish() as u32)
    }
}

impl From<&core::any::TypeId> for TypeId {
    fn from(type_id: &core::any::TypeId) -> Self {
        let mut hasher = TypeIdHasher::default();

        type_id.hash(&mut hasher);

        TypeId(hasher.finish() as u32)
    }
}

impl PartialEq<core::any::TypeId> for TypeId {
    fn eq(&self, other: &core::any::TypeId) -> bool {
        let type_id: TypeId = other.into();

        *self == type_id
    }
}

impl PartialEq<TypeId> for core::any::TypeId {
    fn eq(&self, other: &TypeId) -> bool {
        *other == *self
    }
}
