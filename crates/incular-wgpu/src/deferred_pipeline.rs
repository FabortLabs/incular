use crate::RendererError;
use std::sync::{Arc, OnceLock};

/// A device/format-owned pipeline compiled once, when first needed by a draw.
/// Clones share both successful compilation and a labeled validation failure.
///
/// Content pipelines can be drawn in passes with or without the clip stencil
/// attachment; each form compiles independently on first use, so frames
/// without stencil clips never allocate a stencil texture.
#[derive(Clone)]
pub(crate) struct DeferredPipeline(Arc<DeferredPipelineInner>);

type PipelineResult = Result<wgpu::RenderPipeline, String>;

struct DeferredPipelineInner {
    device: wgpu::Device,
    label: &'static str,
    stencil_optional: bool,
    /// Builds the pipeline; the flag selects a stencil-tested form.
    create: Box<dyn Fn(bool) -> wgpu::RenderPipeline + Send + Sync>,
    /// `[stenciled, stencil-free]`; the second stays empty unless optional.
    values: [OnceLock<PipelineResult>; 2],
}

impl DeferredPipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        label: &'static str,
        stencil_optional: bool,
        create: impl Fn(bool) -> wgpu::RenderPipeline + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(DeferredPipelineInner {
            device: device.clone(),
            label,
            stencil_optional,
            create: Box::new(create),
            values: [OnceLock::new(), OnceLock::new()],
        }))
    }

    /// The pipeline for a pass with a clip stencil attachment.
    pub(crate) fn get(&self) -> Result<&wgpu::RenderPipeline, RendererError> {
        self.get_for(true)
    }

    /// The pipeline matching a pass with (`stenciled`) or without a stencil
    /// attachment. Pipelines with a fixed stencil contract ignore the flag.
    pub(crate) fn get_for(&self, stenciled: bool) -> Result<&wgpu::RenderPipeline, RendererError> {
        let stenciled = stenciled || !self.0.stencil_optional;
        self.0.values[usize::from(!stenciled)]
            .get_or_init(|| {
                let scope = self
                    .0
                    .device
                    .push_error_scope(wgpu::ErrorFilter::Validation);
                let pipeline = (self.0.create)(stenciled);
                // Native pipeline creation and validation are synchronous;
                // resolving this error scope does not wait for GPU execution.
                match pollster::block_on(scope.pop()) {
                    Some(error) => Err(error.to_string()),
                    None => Ok(pipeline),
                }
            })
            .as_ref()
            .map_err(|reason| RendererError::PipelineCreation {
                label: self.0.label.to_owned(),
                reason: reason.clone(),
            })
    }

    pub(crate) fn is_created(&self) -> bool {
        self.0
            .values
            .iter()
            .any(|value| matches!(value.get(), Some(Ok(_))))
    }
}
