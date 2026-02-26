
#[derive(Debug)]
pub struct Looakhead<I: Iterator, const SIZE: usize> {
    inner: I,
    peeked: [Option<I::Item>; SIZE],
}

impl<I: Iterator, const SIZE: usize> Looakhead<I, SIZE> {
    pub fn new(mut iter: I) -> Self {
        Self {
            peeked: std::array::from_fn(|_| iter.next()),
            inner: iter,
        }
    }
    pub fn lookahead(&self, index: usize) -> Option<&I::Item> {
        self.peeked[index].as_ref()
    }
}

impl<I: Iterator, const SIZE: usize> Iterator for Looakhead<I, SIZE> {
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.peeked.rotate_left(1);
        std::mem::replace(&mut self.peeked[SIZE - 1], self.inner.next())
    }
}
