// SPDX-License-Identifier: MPL-2.0

use core::ops::{Deref, DerefMut};

use alloc::fmt;

/// Always [`Sync`], but unsafe to reference the data.
pub(super) struct ForceSync<T>(T);

impl<T> fmt::Debug for ForceSync<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForceSync").finish_non_exhaustive()
    }
}

// SAFETY: The caller of the `ForceSync::get` method must ensure that the underlying data is not
// concurrently accessed if the underlying type is not `Sync`.
unsafe impl<T> Sync for ForceSync<T> {}

impl<T> Deref for ForceSync<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ForceSync<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[allow(unused)]
impl<T> ForceSync<T> {
    /// Creates an instance with `data` as the inner data.
    pub(super) fn new(data: T) -> Self {
        Self(data)
    }
}
