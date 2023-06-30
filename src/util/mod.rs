use std::iter::Peekable;
use std::mem::replace;

/// An iterator with methods that can peek into previous items
pub struct PrevPeekable<I>
where
    I: Iterator,
    <I as Iterator>::Item: Clone,
{
    /// Iterator that `PrevPeekable` wraps
    iterator: Peekable<I>,
    /// The item before the current item
    prev: Option<I::Item>,
    /// The current item
    current: Option<I::Item>,
    /// Keeps track of whether the iterator has reached the end or not
    finished: bool,
}

impl<I> PrevPeekable<I>
where
    I: Iterator,
    <I as Iterator>::Item: Clone,
{
    pub fn new(iterator: I) -> Self {
        PrevPeekable {
            iterator: iterator.peekable(),
            prev: None,
            current: None,
            finished: false,
        }
    }

    /// Returns a reference to the `next()` item without advancing the iterator
    pub fn peek(&mut self) -> Option<&I::Item> {
        self.iterator.peek()
    }

    /// Returns the previous item in the iterator without moving the iterator backwards
    pub fn prev(&self) -> Option<I::Item> {
        self.prev.clone()
    }

    /// Returns a reference to the previous item in the iterator without moving the iterator
    /// backwards
    pub fn prev_peek(&self) -> Option<&I::Item> {
        self.prev.as_ref()
    }
}

impl<I> Iterator for PrevPeekable<I>
where
    I: Iterator,
    <I as Iterator>::Item: Clone,
{
    type Item = I::Item;

    /// Returns the next item in the iterator
    fn next(&mut self) -> Option<I::Item> {
        if self.iterator.peek().is_some() {
            self.prev = replace(&mut self.current, self.iterator.next());
            return self.current.clone();
        } else if !self.finished {
            self.prev = replace(&mut self.current, self.iterator.next());
            self.finished = true;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! iter {
        ($v: expr) => {
            PrevPeekable::new($v.iter())
        };
    }

    #[test]
    fn test_next() {
        let v = vec![1, 2, 3];
        let mut iter = iter!(v);

        assert_eq!(Some(&1), iter.next());
        assert_eq!(None, iter.prev);
        assert_eq!(Some(&2), iter.next());
        assert_eq!(Some(&1), iter.prev);
        assert_eq!(Some(&3), iter.next());
        assert_eq!(Some(&2), iter.prev);
        assert_eq!(None, iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_peek() {
        let v = vec![1, 2];
        let mut iter = iter!(v);

        assert_eq!(Some(&&1), iter.peek());
        assert_eq!(Some(&1), iter.next());
        assert_eq!(Some(&&2), iter.peek());
        assert_eq!(Some(&2), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_prev() {
        let v = vec![1, 2];
        let mut it = iter!(v);

        assert_eq!(None, it.prev());
        assert_eq!(Some(&1), it.next());
        assert_eq!(None, it.prev());
        assert_eq!(Some(&2), it.next());
        assert_eq!(Some(&1), it.prev());
        assert_eq!(None, it.next());
        assert_eq!(Some(&2), it.prev());

        assert_eq!(None, it.next());
        assert_eq!(Some(&2), it.prev());
    }

    #[test]
    fn test_prev_peek() {
        let v = vec![1, 2];
        let mut it = iter!(v);

        assert_eq!(None, it.prev_peek());
        assert_eq!(Some(&1), it.next());
        assert_eq!(None, it.prev_peek());
        assert_eq!(Some(&2), it.next());
        assert_eq!(Some(&&1), it.prev_peek());
        assert_eq!(None, it.next());
        assert_eq!(Some(&&2), it.prev_peek());

        assert_eq!(None, it.next());
        assert_eq!(Some(&&2), it.prev_peek());
    }
}
