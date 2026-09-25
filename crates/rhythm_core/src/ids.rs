use core::num::NonZeroU64;
use serde::{Deserialize, Serialize};

macro_rules! define_project_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        pub struct $name(NonZeroU64);

        impl $name {
            #[must_use]
            pub const fn new(value: u64) -> Option<Self> {
                match NonZeroU64::new(value) {
                    Some(value) => Some(Self(value)),
                    None => None,
                }
            }

            #[must_use]
            pub const fn get(self) -> u64 {
                self.0.get()
            }

            const fn from_nonzero(value: NonZeroU64) -> Self {
                Self(value)
            }
        }
    };
}

define_project_id!(ObjectId);
define_project_id!(AssetId);
define_project_id!(EffectId);
define_project_id!(KeyframeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdAllocationError {
    ZeroNextEntityId,
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityIdAllocator {
    next_entity_id: NonZeroU64,
}

impl EntityIdAllocator {
    pub fn new(next_entity_id: u64) -> Result<Self, IdAllocationError> {
        let next_entity_id =
            NonZeroU64::new(next_entity_id).ok_or(IdAllocationError::ZeroNextEntityId)?;
        Ok(Self { next_entity_id })
    }

    #[must_use]
    pub const fn next_entity_id(&self) -> u64 {
        self.next_entity_id.get()
    }

    pub fn allocate_object(&mut self) -> Result<ObjectId, IdAllocationError> {
        self.allocate_nonzero().map(ObjectId::from_nonzero)
    }

    pub fn allocate_asset(&mut self) -> Result<AssetId, IdAllocationError> {
        self.allocate_nonzero().map(AssetId::from_nonzero)
    }

    pub fn allocate_effect(&mut self) -> Result<EffectId, IdAllocationError> {
        self.allocate_nonzero().map(EffectId::from_nonzero)
    }

    pub fn allocate_keyframe(&mut self) -> Result<KeyframeId, IdAllocationError> {
        self.allocate_nonzero().map(KeyframeId::from_nonzero)
    }

    fn allocate_nonzero(&mut self) -> Result<NonZeroU64, IdAllocationError> {
        let current = self.next_entity_id;
        let next = current
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .ok_or(IdAllocationError::Exhausted)?;
        self.next_entity_id = next;
        Ok(current)
    }
}

#[cfg(test)]
mod tests {
    use super::{AssetId, EffectId, EntityIdAllocator, IdAllocationError, KeyframeId, ObjectId};

    #[test]
    fn zero_is_reserved_for_every_project_id_type() {
        assert!(ObjectId::new(0).is_none());
        assert!(AssetId::new(0).is_none());
        assert!(EffectId::new(0).is_none());
        assert!(KeyframeId::new(0).is_none());
    }

    #[test]
    fn nonzero_values_round_trip() {
        assert_eq!(ObjectId::new(1).map(ObjectId::get), Some(1));
        assert_eq!(AssetId::new(2).map(AssetId::get), Some(2));
        assert_eq!(EffectId::new(3).map(EffectId::get), Some(3));
        assert_eq!(
            KeyframeId::new(u64::MAX).map(KeyframeId::get),
            Some(u64::MAX)
        );
    }

    #[test]
    fn allocator_is_shared_monotonic_and_never_reuses() {
        let mut allocator = EntityIdAllocator::new(1).expect("valid starting id");

        let object = allocator.allocate_object().expect("object id");
        let asset = allocator.allocate_asset().expect("asset id");
        let effect = allocator.allocate_effect().expect("effect id");
        let keyframe = allocator.allocate_keyframe().expect("keyframe id");

        assert_eq!(object.get(), 1);
        assert_eq!(asset.get(), 2);
        assert_eq!(effect.get(), 3);
        assert_eq!(keyframe.get(), 4);
        assert_eq!(allocator.next_entity_id(), 5);
    }

    #[test]
    fn allocator_rejects_zero_and_exhaustion() {
        assert_eq!(
            EntityIdAllocator::new(0),
            Err(IdAllocationError::ZeroNextEntityId)
        );

        let mut allocator = EntityIdAllocator::new(u64::MAX).expect("nonzero start");
        assert_eq!(
            allocator.allocate_object(),
            Err(IdAllocationError::Exhausted)
        );
        assert_eq!(allocator.next_entity_id(), u64::MAX);
    }
}
