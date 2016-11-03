// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::sync::Arc;

use command_buffer::StatesManager;
use device::Queue;
use format::ClearValue;
use format::Format;
use format::PossibleFloatFormatDesc;
use format::PossibleUintFormatDesc;
use format::PossibleSintFormatDesc;
use format::PossibleDepthFormatDesc;
use format::PossibleStencilFormatDesc;
use format::PossibleDepthStencilFormatDesc;
use image::Dimensions;
use image::ImageDimensions;
use image::sys::Layout;
use image::sys::UnsafeImage;
use image::sys::UnsafeImageView;
use sampler::Sampler;
use sync::AccessFlagBits;
use sync::PipelineStages;
use sync::Fence;
use sync::Semaphore;

/// Trait for types that represent images.
pub unsafe trait Image {
    /// Returns the inner unsafe image object used by this image.
    fn inner(&self) -> &UnsafeImage;

    /// Returns the format of this image.
    #[inline]
    fn format(&self) -> Format {
        self.inner().format()
    }

    /// Returns true if the image is a color image.
    #[inline]
    fn has_color(&self) -> bool {
        let format = self.format();
        format.is_float() || format.is_uint() || format.is_sint()
    }

    /// Returns true if the image has a depth component. In other words, if it is a depth or a
    /// depth-stencil format. 
    #[inline]
    fn has_depth(&self) -> bool {
        let format = self.format();
        format.is_depth() || format.is_depth_stencil()
    }

    /// Returns true if the image has a stencil component. In other words, if it is a stencil or a
    /// depth-stencil format. 
    #[inline]
    fn has_stencil(&self) -> bool {
        let format = self.format();
        format.is_stencil() || format.is_depth_stencil()
    }

    /// Returns the number of samples of this image.
    #[inline]
    fn samples(&self) -> u32 {
        self.inner().samples()
    }

    /// Returns the dimensions of the image.
    #[inline]
    fn dimensions(&self) -> ImageDimensions {
        self.inner().dimensions()
    }

    /// Returns true if the image can be used as a source for blits.
    #[inline]
    fn supports_blit_source(&self) -> bool {
        self.inner().supports_blit_source()
    }

    /// Returns true if the image can be used as a destination for blits.
    #[inline]
    fn supports_blit_destination(&self) -> bool {
        self.inner().supports_blit_destination()
    }
}

unsafe impl<I: ?Sized> Image for Arc<I> where I: Image {
    #[inline]
    fn inner(&self) -> &UnsafeImage {
        (**self).inner()
    }
}

unsafe impl<'a, I: ?Sized + 'a> Image for &'a I where I: Image {
    #[inline]
    fn inner(&self) -> &UnsafeImage {
        (**self).inner()
    }
}

/// Extension trait for `Image`. Types that implement this can be used in a `StdCommandBuffer`.
///
/// Each buffer and image used in a `StdCommandBuffer` have an associated state which is
/// represented by the `CommandListState` associated type of this trait. You can make multiple
/// buffers or images share the same state by making `is_same` return true.
pub unsafe trait TrackedImage<States = StatesManager>: Image {
    /// Returns a new state that corresponds to the moment after a slice of the image has been
    /// used in the pipeline. The parameters indicate in which way it has been used.
    ///
    /// If the transition should result in a pipeline barrier, then it must be returned by this
    /// function.
    // TODO: what should be the behavior if `num_command` is equal to the `num_command` of a
    // previous transition?
    fn transition(&self, states: &mut States, num_command: usize, first_mipmap: u32,
                  num_mipmaps: u32, first_layer: u32, num_layers: u32, write: bool, layout: Layout,
                  stage: PipelineStages, access: AccessFlagBits)
                  -> Option<TrackedImagePipelineBarrierRequest>;

    /// Function called when the command buffer builder is turned into a real command buffer.
    ///
    /// This function can return an additional pipeline barrier that will be applied at the end
    /// of the command buffer.
    fn finish(&self, in_s: &mut States, out: &mut States)
              -> Option<TrackedImagePipelineBarrierRequest>;

    /// Called right before the command buffer is submitted.
    // TODO: function should be unsafe because it must be guaranteed that a cb is submitted
    fn on_submit(&self, states: &States, queue: &Arc<Queue>, fence: &mut FnMut() -> Arc<Fence>)
                 -> TrackedImageSubmitInfos;
}

unsafe impl<I: ?Sized, S> TrackedImage<S> for Arc<I> where I: TrackedImage<S> {
    #[inline]
    fn transition(&self, states: &mut S, num_command: usize, first_mipmap: u32,
                  num_mipmaps: u32, first_layer: u32, num_layers: u32, write: bool, layout: Layout,
                  stage: PipelineStages, access: AccessFlagBits)
                  -> Option<TrackedImagePipelineBarrierRequest>
    {
        (**self).transition(states, num_command, first_mipmap, num_mipmaps, first_layer, num_layers,
                            write, layout, stage, access)
    }

    #[inline]
    fn finish(&self, in_s: &mut S, out: &mut S)
              -> Option<TrackedImagePipelineBarrierRequest>
    {
        (**self).finish(in_s, out)
    }

    #[inline]
    fn on_submit(&self, states: &S, queue: &Arc<Queue>, fence: &mut FnMut() -> Arc<Fence>)
                 -> TrackedImageSubmitInfos
    {
        (**self).on_submit(states, queue, fence)
    }
}

unsafe impl<'a, I: ?Sized + 'a, S> TrackedImage<S> for &'a I where I: TrackedImage<S> {
    #[inline]
    fn transition(&self, states: &mut S, num_command: usize, first_mipmap: u32,
                  num_mipmaps: u32, first_layer: u32, num_layers: u32, write: bool, layout: Layout,
                  stage: PipelineStages, access: AccessFlagBits)
                  -> Option<TrackedImagePipelineBarrierRequest>
    {
        (**self).transition(states, num_command, first_mipmap, num_mipmaps, first_layer, num_layers,
                            write, layout, stage, access)
    }

    #[inline]
    fn finish(&self, in_s: &mut S, out: &mut S)
              -> Option<TrackedImagePipelineBarrierRequest>
    {
        (**self).finish(in_s, out)
    }

    #[inline]
    fn on_submit(&self, states: &S, queue: &Arc<Queue>, fence: &mut FnMut() -> Arc<Fence>)
                 -> TrackedImageSubmitInfos
    {
        (**self).on_submit(states, queue, fence)
    }
}

/// Requests that a pipeline barrier is created.
pub struct TrackedImagePipelineBarrierRequest {
    /// The number of the command after which the barrier should be placed. Must usually match
    /// the number that was passed to the previous call to `transition`, or 0 if the image hasn't
    /// been used yet.
    pub after_command_num: usize,

    /// The source pipeline stages of the transition.
    pub source_stage: PipelineStages,

    /// The destination pipeline stages of the transition.
    pub destination_stages: PipelineStages,

    /// If true, the pipeliner barrier is by region.
    pub by_region: bool,

    /// An optional memory barrier. See the docs of `TrackedImagePipelineMemoryBarrierRequest`.
    pub memory_barrier: Option<TrackedImagePipelineMemoryBarrierRequest>,
}

/// Requests that a memory barrier is created as part of the pipeline barrier.
///
/// By default, a pipeline barrier only guarantees that the source operations are executed before
/// the destination operations, but it doesn't make memory writes made by source operations visible
/// to the destination operations. In order to make so, you have to add a memory barrier.
///
/// The memory barrier always concerns the image that is currently being processed. You can't add
/// a memory barrier that concerns another resource.
pub struct TrackedImagePipelineMemoryBarrierRequest {
    pub first_mipmap: u32,
    pub num_mipmaps: u32,
    pub first_layer: u32,
    pub num_layers: u32,

    pub old_layout: Layout,
    pub new_layout: Layout,

    /// Source accesses.
    pub source_access: AccessFlagBits,
    /// Destination accesses.
    pub destination_access: AccessFlagBits,
}

pub struct TrackedImageSubmitInfos {
    pub pre_semaphore: Option<(Arc<Semaphore>, PipelineStages)>,
    pub post_semaphore: Option<Arc<Semaphore>>,
    pub pre_barrier: Option<TrackedImagePipelineBarrierRequest>,
    pub post_barrier: Option<TrackedImagePipelineBarrierRequest>,
}

/// Extension trait for images. Checks whether the value `T` can be used as a clear value for the
/// given image.
// TODO: isn't that for image views instead?
pub unsafe trait ImageClearValue<T>: Image {
    fn decode(&self, T) -> Option<ClearValue>;
}

pub unsafe trait ImageContent<P>: Image {
    /// Checks whether pixels of type `P` match the format of the image.
    fn matches_format(&self) -> bool;
}

/// Trait for types that represent image views.
pub unsafe trait ImageView {
    fn parent(&self) -> &Image;

    /// Returns the dimensions of the image view.
    fn dimensions(&self) -> Dimensions;

    /// Returns the inner unsafe image view object used by this image view.
    fn inner(&self) -> &UnsafeImageView;

    /// Returns the format of this view. This can be different from the parent's format.
    #[inline]
    fn format(&self) -> Format {
        self.inner().format()
    }

    #[inline]
    fn samples(&self) -> u32 {
        self.parent().samples()
    }

    /// Returns the image layout to use in a descriptor with the given subresource.
    fn descriptor_set_storage_image_layout(&self) -> Layout;
    /// Returns the image layout to use in a descriptor with the given subresource.
    fn descriptor_set_combined_image_sampler_layout(&self) -> Layout;
    /// Returns the image layout to use in a descriptor with the given subresource.
    fn descriptor_set_sampled_image_layout(&self) -> Layout;
    /// Returns the image layout to use in a descriptor with the given subresource.
    fn descriptor_set_input_attachment_layout(&self) -> Layout;

    /// Returns true if the view doesn't use components swizzling.
    ///
    /// Must be true when the view is used as a framebuffer attachment or TODO: I don't remember
    /// the other thing.
    fn identity_swizzle(&self) -> bool;

    /// Returns true if the given sampler can be used with this image view.
    ///
    /// This method should check whether the sampler's configuration can be used with the format
    /// of the view.
    // TODO: return a Result
    fn can_be_sampled(&self, sampler: &Sampler) -> bool { true /* FIXME */ }

    //fn usable_as_render_pass_attachment(&self, ???) -> Result<(), ???>;
}

unsafe impl<'a, T: ?Sized + 'a> ImageView for &'a T where T: ImageView {
    #[inline]
    fn parent(&self) -> &Image {
        (**self).parent()
    }

    #[inline]
    fn inner(&self) -> &UnsafeImageView {
        (**self).inner()
    }

    #[inline]
    fn dimensions(&self) -> Dimensions {
        (**self).dimensions()
    }

    #[inline]
    fn descriptor_set_storage_image_layout(&self) -> Layout {
        (**self).descriptor_set_storage_image_layout()
    }
    #[inline]
    fn descriptor_set_combined_image_sampler_layout(&self) -> Layout {
        (**self).descriptor_set_combined_image_sampler_layout()
    }
    #[inline]
    fn descriptor_set_sampled_image_layout(&self) -> Layout {
        (**self).descriptor_set_sampled_image_layout()
    }
    #[inline]
    fn descriptor_set_input_attachment_layout(&self) -> Layout {
        (**self).descriptor_set_input_attachment_layout()
    }

    #[inline]
    fn identity_swizzle(&self) -> bool {
        (**self).identity_swizzle()
    }

    #[inline]
    fn can_be_sampled(&self, sampler: &Sampler) -> bool {
        (**self).can_be_sampled(sampler)
    }
}

unsafe impl<T: ?Sized> ImageView for Arc<T> where T: ImageView {
    #[inline]
    fn parent(&self) -> &Image {
        (**self).parent()
    }

    #[inline]
    fn inner(&self) -> &UnsafeImageView {
        (**self).inner()
    }

    #[inline]
    fn dimensions(&self) -> Dimensions {
        (**self).dimensions()
    }

    #[inline]
    fn descriptor_set_storage_image_layout(&self) -> Layout {
        (**self).descriptor_set_storage_image_layout()
    }
    #[inline]
    fn descriptor_set_combined_image_sampler_layout(&self) -> Layout {
        (**self).descriptor_set_combined_image_sampler_layout()
    }
    #[inline]
    fn descriptor_set_sampled_image_layout(&self) -> Layout {
        (**self).descriptor_set_sampled_image_layout()
    }
    #[inline]
    fn descriptor_set_input_attachment_layout(&self) -> Layout {
        (**self).descriptor_set_input_attachment_layout()
    }

    #[inline]
    fn identity_swizzle(&self) -> bool {
        (**self).identity_swizzle()
    }

    #[inline]
    fn can_be_sampled(&self, sampler: &Sampler) -> bool {
        (**self).can_be_sampled(sampler)
    }
}

pub unsafe trait TrackedImageView<States>: ImageView {
    type Image: TrackedImage<States>;

    fn image(&self) -> &Self::Image;
}

unsafe impl<'a, S, T: ?Sized + 'a> TrackedImageView<S> for &'a T where T: TrackedImageView<S> {
    type Image = T::Image;

    #[inline]
    fn image(&self) -> &Self::Image {
        (**self).image()
    }
}

unsafe impl<S, T: ?Sized> TrackedImageView<S> for Arc<T> where T: TrackedImageView<S> {
    type Image = T::Image;

    #[inline]
    fn image(&self) -> &Self::Image {
        (**self).image()
    }
}

pub unsafe trait AttachmentImageView: ImageView {
    fn accept(&self, initial_layout: Layout, final_layout: Layout) -> bool;
}
