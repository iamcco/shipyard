use crate::entity_id::EntityId;
use crate::error;
use crate::r#mut::Mut;
use crate::sparse_set::SparseSet;
use crate::view::{View, ViewMut};
use core::any::type_name;

/// Retrives components based on their type and entity id.
pub trait Get {
    #[allow(missing_docs)]
    type Out;
    #[allow(missing_docs)]
    type FastOut;
    /// Retrieve components of `entity`.
    ///
    /// Multiple components can be queried at the same time using a tuple.
    ///
    /// ### Example:
    /// ```
    /// use shipyard::{Get, View, World};
    ///
    /// let mut world = World::new();
    ///
    /// let entity = world.add_entity((0usize, 1u32));
    ///
    /// let (usizes, u32s) = world.borrow::<(View<usize>, View<u32>)>().unwrap();
    /// assert_eq!((&usizes, &u32s).get(entity), Ok((&0, &1)));
    /// ```
    fn get(self, entity: EntityId) -> Result<Self::Out, error::MissingComponent>;
    /// Retrieve components of `entity` without fine modification tracking.
    ///
    /// Multiple components can be queried at the same time using a tuple.
    ///
    /// ### Example:
    /// ```
    /// use shipyard::{Get, View, World};
    ///
    /// let mut world = World::new();
    ///
    /// let entity = world.add_entity((0usize, 1u32));
    ///
    /// let (usizes, u32s) = world.borrow::<(View<usize>, View<u32>)>().unwrap();
    /// assert_eq!((&usizes, &u32s).fast_get(entity), Ok((&0, &1)));
    /// ```
    fn fast_get(self, entity: EntityId) -> Result<Self::FastOut, error::MissingComponent>;
}

impl<'a: 'b, 'b, T: 'static> Get for &'b View<'a, T> {
    type Out = &'b T;
    type FastOut = &'b T;

    #[inline]
    fn get(self, entity: EntityId) -> Result<Self::Out, error::MissingComponent> {
        (**self)
            .private_get(entity)
            .ok_or_else(|| error::MissingComponent {
                id: entity,
                name: type_name::<T>(),
            })
    }
    #[inline]
    fn fast_get(self, entity: EntityId) -> Result<Self::FastOut, error::MissingComponent> {
        (**self)
            .private_get(entity)
            .ok_or_else(|| error::MissingComponent {
                id: entity,
                name: type_name::<T>(),
            })
    }
}

impl<'a: 'b, 'b, T: 'static> Get for &'b ViewMut<'a, T> {
    type Out = &'b T;
    type FastOut = &'b T;

    #[inline]
    fn get(self, entity: EntityId) -> Result<Self::Out, error::MissingComponent> {
        (**self)
            .private_get(entity)
            .ok_or_else(|| error::MissingComponent {
                id: entity,
                name: type_name::<T>(),
            })
    }
    #[inline]
    fn fast_get(self, entity: EntityId) -> Result<Self::FastOut, error::MissingComponent> {
        (**self)
            .private_get(entity)
            .ok_or_else(|| error::MissingComponent {
                id: entity,
                name: type_name::<T>(),
            })
    }
}

impl<'a: 'b, 'b, T: 'static> Get for &'b mut ViewMut<'a, T> {
    type Out = Mut<'b, T>;
    type FastOut = &'b mut T;

    #[inline]
    fn get(self, entity: EntityId) -> Result<Self::Out, error::MissingComponent> {
        let index = self
            .index_of(entity)
            .ok_or_else(|| error::MissingComponent {
                id: entity,
                name: type_name::<T>(),
            })?;

        if self.is_tracking_modification() {
            let SparseSet {
                sparse: _,
                dense,
                data,
                metadata: _,
            } = &mut **self;

            let entity = unsafe { dense.get_unchecked_mut(index) };

            Ok(Mut {
                flag: if !entity.is_inserted() {
                    Some(entity)
                } else {
                    None
                },
                data: unsafe { data.get_unchecked_mut(index) },
            })
        } else {
            Ok(Mut {
                flag: None,
                data: unsafe { self.data.get_unchecked_mut(index) },
            })
        }
    }
    #[inline]
    fn fast_get(self, entity: EntityId) -> Result<Self::FastOut, error::MissingComponent> {
        self.private_get_mut(entity)
            .ok_or_else(|| error::MissingComponent {
                id: entity,
                name: type_name::<T>(),
            })
    }
}

macro_rules! impl_get_component {
    ($(($type: ident, $index: tt))+) => {
        impl<$($type: Get),+> Get for ($($type,)+) {
            type Out = ($($type::Out,)+);
            type FastOut = ($($type::FastOut,)+);
            #[inline]
            fn get(self, entity: EntityId) -> Result<Self::Out, error::MissingComponent> {
                Ok(($(self.$index.get(entity)?,)+))
            }
            #[inline]
            fn fast_get(self, entity: EntityId) -> Result<Self::FastOut, error::MissingComponent> {
                Ok(($(self.$index.fast_get(entity)?,)+))
            }
        }
    }
}

macro_rules! get_component {
    ($(($type: ident, $index: tt))+; ($type1: ident, $index1: tt) $(($queue_type: ident, $queue_index: tt))*) => {
        impl_get_component![$(($type, $index))*];
        get_component![$(($type, $index))* ($type1, $index1); $(($queue_type, $queue_index))*];
    };
    ($(($type: ident, $index: tt))+;) => {
        impl_get_component![$(($type, $index))*];
    }
}

get_component![(A, 0); (B, 1) (C, 2) (D, 3) (E, 4) (F, 5) (G, 6) (H, 7) (I, 8) (J, 9)];
