// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::Entity;

use std::ops::Deref;

use camino::Utf8PathBuf;

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Node<'a, P = Utf8PathBuf>(Entity<'a, P>);

impl<'a, P> Deref for Node<'a, P> {
    type Target = Entity<'a, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, P> From<Entity<'a, P>> for Node<'a, P> {
    fn from(entity: Entity<'a, P>) -> Self {
        Self(entity)
    }
}
