#[derive(Debug, Clone, PartialEq, Hash, Eq, Copy)]
pub struct ImageIterator {
    width: u32,
    height: u32,
    item: u32,
}

impl ImageIterator {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            item: 0_u32,
        }
    }
    pub fn with_dimension(dimension: &(u32, u32)) -> Self {
        Self {
            width: dimension.0,
            height: dimension.1,
            item: 0_u32,
        }
    }
}

impl Iterator for ImageIterator {
    type Item = (u32, u32);

    fn next(&mut self) -> Option<Self::Item> {
        let n = self.item;
        self.item += 1;
        if n < (self.width * self.height) {
            Some((n / self.height, n % self.height))
        } else {
            None
        }
    }
}
