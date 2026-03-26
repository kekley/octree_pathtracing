use glam::{Vec3A, Vec4};

trait VectorExtensions {
    type VectorType;
    fn iter(&self) -> impl Iterator<Item = Self::VectorType>;
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::VectorType>;
}
macro_rules! vector_ext_impl {
    ($($vector_type:ty, $inner_type:ty,$vector_iter:ident, $vector_iter_mut:ident, $vector_length:literal),* $(,)?) => {
        $(
        struct $vector_iter<'a> {
            vector: &'a $vector_type,
            index: u8,
        }

        struct $vector_iter_mut<'a> {
            vector: &'a mut $vector_type,
            index: u8,
        }

        impl Iterator for $vector_iter<'_> {
            type Item = $inner_type;

            fn next(&mut self) -> Option<Self::Item> {
                const VECTOR_SIZE: usize = $vector_length;
                if self.index as usize >= VECTOR_SIZE {
                    return None;
                } else {
                    Some(self.vector[self.index.into()])
                }
            }
        }

        impl<'a> Iterator for $vector_iter_mut<'a> {
            type Item = &'a mut $inner_type;

            fn next(&mut self) -> Option<Self::Item> {
                const VECTOR_SIZE: usize = $vector_length;
                if self.index as usize >= VECTOR_SIZE {
                    return None;
                } else {
                    unsafe { std::mem::transmute(Some(&mut self.vector[self.index.into()])) }
                }
            }
        }
        impl VectorExtensions for $vector_type {
            type VectorType = $inner_type;
            fn iter(&self) -> impl Iterator<Item = Self::VectorType> {
                $vector_iter {
                    vector: self,
                    index: 0,
                }
            }

            fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::VectorType> {
                $vector_iter_mut {
                    vector: self,
                    index: 0,
                }
            }
        }
        )*
    };
}

vector_ext_impl!(
    Vec3A,
    f32,
    Vec3AIter,
    Vec3AIterMut,
    3,
    Vec4,
    f32,
    Vec4Iter,
    Vec4IterMut,
    4,
    Vec2,
    f32,
    Vec2Iter,
    Vec2IterMut
    2
);
