use std::{collections::HashMap, path::PathBuf};

use rhythm_core::ids::AssetId;

use crate::image_decode::{
    DecodedImage, ImageDecodeError, ImageDecodeGeneration, ImageDecodeResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedDecodedImage {
    asset_id: AssetId,
    generation: ImageDecodeGeneration,
    path: PathBuf,
    image: DecodedImage,
}

impl ValidatedDecodedImage {
    #[must_use]
    pub const fn asset_id(&self) -> AssetId {
        self.asset_id
    }

    #[must_use]
    pub const fn generation(&self) -> ImageDecodeGeneration {
        self.generation
    }

    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    #[must_use]
    pub const fn image(&self) -> &DecodedImage {
        &self.image
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentImageDecodeFailure {
    pub asset_id: AssetId,
    pub generation: ImageDecodeGeneration,
    pub path: PathBuf,
    pub error: ImageDecodeError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaleImageDecode {
    pub asset_id: AssetId,
    pub result_generation: ImageDecodeGeneration,
    pub expected_generation: Option<ImageDecodeGeneration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageDecodeValidation {
    Ready(ValidatedDecodedImage),
    Failed(CurrentImageDecodeFailure),
    Stale(StaleImageDecode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAssetError {
    GenerationExhausted(AssetId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeImageState {
    Missing {
        generation: ImageDecodeGeneration,
    },
    Decoding {
        generation: ImageDecodeGeneration,
    },
    Ready {
        generation: ImageDecodeGeneration,
    },
    Failed {
        generation: ImageDecodeGeneration,
    },
}

impl RuntimeImageState {
    #[must_use]
    pub const fn generation(self) -> ImageDecodeGeneration {
        match self {
            Self::Missing { generation }
            | Self::Decoding { generation }
            | Self::Ready { generation }
            | Self::Failed { generation } => generation,
        }
    }
}

#[derive(Debug, Default)]
pub struct RuntimeAssetManager {
    image_generations: HashMap<AssetId, ImageDecodeGeneration>,
    image_states: HashMap<AssetId, RuntimeImageState>,
}

impl RuntimeAssetManager {
    #[must_use]
    pub fn current_image_generation(
        &self,
        asset_id: AssetId,
    ) -> Option<ImageDecodeGeneration> {
        self.image_generations.get(&asset_id).copied()
    }

    #[must_use]
    pub fn image_state(&self, asset_id: AssetId) -> Option<RuntimeImageState> {
        self.image_states.get(&asset_id).copied()
    }

    pub fn begin_image_decode(
        &mut self,
        asset_id: AssetId,
    ) -> Result<ImageDecodeGeneration, RuntimeAssetError> {
        let next = self.next_image_generation(asset_id)?;
        self.image_states.insert(
            asset_id,
            RuntimeImageState::Decoding { generation: next },
        );
        Ok(next)
    }

    pub fn mark_image_missing(
        &mut self,
        asset_id: AssetId,
    ) -> Result<ImageDecodeGeneration, RuntimeAssetError> {
        let next = self.next_image_generation(asset_id)?;
        self.image_states.insert(
            asset_id,
            RuntimeImageState::Missing { generation: next },
        );
        Ok(next)
    }

    pub fn remove_image(&mut self, asset_id: AssetId) -> bool {
        let generation_removed = self.image_generations.remove(&asset_id).is_some();
        let state_removed = self.image_states.remove(&asset_id).is_some();
        generation_removed || state_removed
    }

    #[must_use]
    pub fn validate_image_decode(&mut self, result: ImageDecodeResult) -> ImageDecodeValidation {
        let expected_generation = self.current_image_generation(result.asset_id);
        if expected_generation != Some(result.generation) {
            return ImageDecodeValidation::Stale(StaleImageDecode {
                asset_id: result.asset_id,
                result_generation: result.generation,
                expected_generation,
            });
        }

        match result.result {
            Ok(image) => {
                self.image_states.insert(
                    result.asset_id,
                    RuntimeImageState::Ready {
                        generation: result.generation,
                    },
                );
                ImageDecodeValidation::Ready(ValidatedDecodedImage {
                    asset_id: result.asset_id,
                    generation: result.generation,
                    path: result.path,
                    image,
                })
            }
            Err(error) => {
                self.image_states.insert(
                    result.asset_id,
                    RuntimeImageState::Failed {
                        generation: result.generation,
                    },
                );
                ImageDecodeValidation::Failed(CurrentImageDecodeFailure {
                    asset_id: result.asset_id,
                    generation: result.generation,
                    path: result.path,
                    error,
                })
            }
        }
    }

    fn next_image_generation(
        &mut self,
        asset_id: AssetId,
    ) -> Result<ImageDecodeGeneration, RuntimeAssetError> {
        let next = match self.current_image_generation(asset_id) {
            Some(current) => current
                .get()
                .checked_add(1)
                .map(ImageDecodeGeneration::new)
                .ok_or(RuntimeAssetError::GenerationExhausted(asset_id))?,
            None => ImageDecodeGeneration::new(1),
        };
        self.image_generations.insert(asset_id, next);
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageDecodeValidation, RuntimeAssetManager, RuntimeImageState};
    use crate::image_decode::{
        DecodedImage, ImageDecodeError, ImageDecodeGeneration, ImageDecodeResult,
    };
    use rhythm_core::ids::AssetId;
    use std::path::PathBuf;

    fn success_result(
        asset_id: AssetId,
        generation: ImageDecodeGeneration,
    ) -> ImageDecodeResult {
        ImageDecodeResult {
            asset_id,
            generation,
            path: PathBuf::from("image.png"),
            result: Ok(DecodedImage {
                width: 2,
                height: 1,
                rgba8: vec![1, 2, 3, 4, 5, 6, 7, 8],
            }),
        }
    }

    #[test]
    fn generations_increment_per_asset() {
        let first = AssetId::new(1).expect("asset id");
        let second = AssetId::new(2).expect("asset id");
        let mut assets = RuntimeAssetManager::default();

        assert_eq!(
            assets.begin_image_decode(first).expect("generation"),
            ImageDecodeGeneration::new(1)
        );
        assert_eq!(
            assets.begin_image_decode(first).expect("generation"),
            ImageDecodeGeneration::new(2)
        );
        assert_eq!(
            assets.begin_image_decode(second).expect("generation"),
            ImageDecodeGeneration::new(1)
        );
    }

    #[test]
    fn missing_state_invalidates_previous_decode_and_relink_starts_new_generation() {
        let asset_id = AssetId::new(6).expect("asset id");
        let mut assets = RuntimeAssetManager::default();
        let first = assets.begin_image_decode(asset_id).expect("first generation");
        assert_eq!(
            assets.image_state(asset_id),
            Some(RuntimeImageState::Decoding { generation: first })
        );

        let missing = assets.mark_image_missing(asset_id).expect("missing generation");
        assert!(missing.get() > first.get());
        assert_eq!(
            assets.image_state(asset_id),
            Some(RuntimeImageState::Missing {
                generation: missing,
            })
        );

        let validation = assets.validate_image_decode(success_result(asset_id, first));
        assert!(matches!(validation, ImageDecodeValidation::Stale(_)));
        assert_eq!(
            assets.image_state(asset_id),
            Some(RuntimeImageState::Missing {
                generation: missing,
            })
        );

        let relinked = assets.begin_image_decode(asset_id).expect("relink generation");
        assert!(relinked.get() > missing.get());
        assert_eq!(
            assets.image_state(asset_id),
            Some(RuntimeImageState::Decoding {
                generation: relinked,
            })
        );
    }

    #[test]
    fn current_generation_decode_becomes_upload_eligible() {
        let asset_id = AssetId::new(7).expect("asset id");
        let mut assets = RuntimeAssetManager::default();
        let generation = assets.begin_image_decode(asset_id).expect("generation");

        let validation = assets.validate_image_decode(success_result(asset_id, generation));
        let ImageDecodeValidation::Ready(validated) = validation else {
            panic!("current decode must be upload eligible");
        };
        assert_eq!(
            assets.image_state(asset_id),
            Some(RuntimeImageState::Ready { generation })
        );
        assert_eq!(validated.asset_id, asset_id);
        assert_eq!(validated.generation, generation);
        assert_eq!(validated.image.width, 2);
        assert_eq!(validated.image.height, 1);
    }

    #[test]
    fn older_generation_is_rejected_after_relink_or_reload() {
        let asset_id = AssetId::new(7).expect("asset id");
        let mut assets = RuntimeAssetManager::default();
        let old_generation = assets.begin_image_decode(asset_id).expect("old generation");
        let current_generation = assets.begin_image_decode(asset_id).expect("new generation");

        let validation = assets.validate_image_decode(success_result(asset_id, old_generation));
        let ImageDecodeValidation::Stale(stale) = validation else {
            panic!("old generation must be stale");
        };
        assert_eq!(stale.result_generation, old_generation);
        assert_eq!(stale.expected_generation, Some(current_generation));
    }

    #[test]
    fn result_for_removed_asset_is_rejected() {
        let asset_id = AssetId::new(9).expect("asset id");
        let mut assets = RuntimeAssetManager::default();
        let generation = assets.begin_image_decode(asset_id).expect("generation");
        assert!(assets.remove_image(asset_id));

        let validation = assets.validate_image_decode(success_result(asset_id, generation));
        let ImageDecodeValidation::Stale(stale) = validation else {
            panic!("removed asset result must be stale");
        };
        assert_eq!(stale.expected_generation, None);
    }

    #[test]
    fn current_decode_error_is_not_upload_eligible() {
        let asset_id = AssetId::new(11).expect("asset id");
        let mut assets = RuntimeAssetManager::default();
        let generation = assets.begin_image_decode(asset_id).expect("generation");
        let result = ImageDecodeResult {
            asset_id,
            generation,
            path: PathBuf::from("broken.png"),
            result: Err(ImageDecodeError::Decode("broken".to_owned())),
        };

        let validation = assets.validate_image_decode(result);
        let ImageDecodeValidation::Failed(failure) = validation else {
            panic!("current decode error must remain a runtime failure");
        };
        assert_eq!(
            assets.image_state(asset_id),
            Some(RuntimeImageState::Failed { generation })
        );
        assert_eq!(failure.asset_id, asset_id);
        assert_eq!(failure.generation, generation);
        assert_eq!(failure.error, ImageDecodeError::Decode("broken".to_owned()));
    }
}
