pub struct MemoryCycleIterator<I>
    where
        I: Iterator,
        I::Item: Clone,
{
    iterator: I,
    cycling: usize,
    items: Vec<I::Item>,
}

impl<'a, I> Iterator for MemoryCycleIterator<I>
    where
        I: Iterator,
        I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iterator.next() {
            Some(item) => {
                self.items.push(item.clone());
                return Some(item);
            },
            None => {
                if self.cycling == self.items.len() - 1 {
                    self.cycling = 0;
                }
                else {
                    self.cycling = self.cycling + 1;
                }
                Some(self.items.get(self.cycling)?.clone())
            }
        }
    }
}

pub trait MemoryCycleIteratorExt: Iterator {
    fn memory_cycle(self) -> MemoryCycleIterator<Self>
        where
            Self: Sized,
            Self::Item: Clone,
    {
        MemoryCycleIterator {
            iterator: self,
            cycling: 0,
            items: Vec::new(),
        }
    }
}

impl<I: Iterator> MemoryCycleIteratorExt for I {}
