use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Copy, Clone)]
pub enum TimedIteratorStrategy {
    Sleep,
    Repeat,
}

#[derive(Copy, Clone)]
pub struct TimedIterator<I>
where
    I: Iterator,
    I::Item: Clone,
{
    duration: Duration,
    next_time: Duration,
    underlying: I,
    reference: SystemTime,
    strategy: TimedIteratorStrategy,
    current: Option<I::Item>,
}

impl<I> Iterator for TimedIterator<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let now = self.reference.elapsed().unwrap();

        match self.strategy {
            TimedIteratorStrategy::Repeat => {
                if self.next_time <= now {
                    self.current = self.underlying.next();
                    self.next_time += self.duration;
                }
                self.current.clone()
            }
            TimedIteratorStrategy::Sleep => {
                if self.next_time > now {
                    thread::sleep(self.next_time - now);
                }

                self.next_time += self.duration;
                self.underlying.next()
            }
        }
    }
}

// TODO: floating point fps
pub trait TimedExt: Iterator {
    fn timed(self, fps: f32, strategy: TimedIteratorStrategy) -> TimedIterator<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        TimedIterator {
            duration: Duration::from_nanos(1_000_000_000 / fps as u64),
            next_time: Duration::new(0, 0),
            underlying: self,
            reference: SystemTime::now(),
            strategy,
            current: None,
        }
    }
}

impl<I: Iterator> TimedExt for I {}
