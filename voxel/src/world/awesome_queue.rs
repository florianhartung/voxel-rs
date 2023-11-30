use std::collections::VecDeque;
use std::hash::Hash;
use std::sync::Mutex;

pub struct AwesomeQueue<T> {
    queue: Mutex<VecDeque<T>>,
}

impl<T: Hash + PartialEq> AwesomeQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn insert(&self, t: T) {
        self.queue.lock().unwrap().push_back(t);
    }

    pub fn remove(&self, t: &T) {
        self.queue.lock().unwrap().retain(|x| x != t);
    }

    pub fn take_all(&self) -> Vec<T> {
        let mut queue = self.queue.lock().unwrap();
        let num_elems = queue.len();
        queue.drain(0..num_elems).collect()
    }

    pub fn take_n(&self, max_num_elems: usize) -> Vec<T> {
        let mut queue = self.queue.lock().unwrap();

        let num_elems = max_num_elems.min(queue.len());

        queue.drain(0..num_elems).collect()
    }

    pub fn contains(&self, t: &T) -> bool {
        self.queue.lock().unwrap().contains(t)
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }
}
