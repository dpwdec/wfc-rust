use nalgebra::{DMatrix, DefaultAllocator, Dynamic, Scalar};
use nalgebra::allocator::Allocator;

pub trait SubMatrix {
    fn crop_left(self, offset: usize) -> Self;
    fn crop_right(self, offset: usize) -> Self;
    fn crop_top(self, offset: usize) -> Self;
    fn crop_bottom(self, offset: usize) -> Self;
    fn sub_matrix(&self, position: (usize, usize), size: (usize, usize)) -> Self;
}

impl<T: Scalar> SubMatrix for DMatrix<T>
    where
        DefaultAllocator: Allocator<T, Dynamic, Dynamic>,
{
    fn crop_left(self, offset: usize) -> DMatrix<T> {
        self.remove_columns(0, offset)
    }

    fn crop_right(self, offset: usize) -> DMatrix<T> {
        let cols = self.ncols();
        if offset >= cols { return self }
        self.remove_columns(offset, cols - offset)
    }

    fn crop_top(self, offset: usize) -> DMatrix<T> {
        self.remove_rows(0, offset)
    }

    fn crop_bottom(self, offset: usize) -> DMatrix<T> {
        let rows = self.nrows();
        if offset >= rows { return self }
        self.remove_rows(offset, rows - offset)
    }

    fn sub_matrix(&self, position: (usize, usize), size: (usize, usize)) -> DMatrix<T> {
        self
            .clone()
            .crop_left(position.0)
            .crop_right(size.0)
            .crop_top(position.1)
            .crop_bottom(size.1)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sub_matrix() {
        let v = vec![0, 1, 2,
                     3, 4, 5,
                     6, 7, 8];
        //                                   ┏━━━━━ Height
        //                                   ┃
        //                                   ┃  ┏━━ Width
        //                                   V  V
        let matrix = DMatrix::from_row_slice(3, 3, &v);

        let x = vec![4, 5];
        let target_a = DMatrix::from_row_slice(1, 2, &x);

        let x = vec![1, 2, 4, 5];
        let target_b = DMatrix::from_row_slice(2, 2, &x);

        let x = vec![0, 1, 2];
        let target_c = DMatrix::from_row_slice(1, 3, &x);

        let x = vec![4];
        let target_d = DMatrix::from_row_slice(1, 1, &x);

        //                                    ┏━━━━━ Width
        //                                    ┃
        //                                    ┃  ┏━━ Height
        //                                    V  V
        assert_eq!(matrix.sub_matrix((0, 0), (3, 3)), matrix);

        assert_eq!(matrix.sub_matrix((1, 1), (2, 1)), target_a);

        assert_eq!(matrix.sub_matrix((1, 0), (2, 2)), target_b);

        assert_eq!(matrix.sub_matrix((0, 0), (3, 1)), target_c);

        assert_eq!(matrix.sub_matrix((1, 1), (1, 1)), target_d);
    }
}