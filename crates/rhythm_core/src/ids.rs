use core::num::NonZeroU64;

macro_rules! define_project_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        }
    };
}

define_project_id!(ObjectId);
define_project_id!(AssetId);
define_project_id!(EffectId);
define_project_id!(KeyframeId);

#[cfg(test)]
mod tests {
    use super::{AssetId, EffectId, KeyframeId, ObjectId};

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
        assert_eq!(KeyframeId::new(u64::MAX).map(KeyframeId::get), Some(u64::MAX));
    }
}
