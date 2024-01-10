use std::{borrow::BorrowMut, fmt::Debug, rc::Rc};

use cell_family::GetWithOwner;

cell_family::define!(type FooFamily: FooCellOwner for FooCell<T>);

struct Node<T> {
    data: T,
    next: Option<Rc<FooCell<Node<T>>>>,
    previous: Option<Rc<FooCell<Node<T>>>>,
}

impl<T> Node<T> {
    fn new(data: T) -> Self {
        Node {
            data,
            next: Option::None,
            previous: Option::None,
        }
    }
}

struct Deque<T> {
    head: Option<Rc<FooCell<Node<T>>>>,
    tail: Option<Rc<FooCell<Node<T>>>>,
    owner: FooCellOwner,
}

impl<T> Deque<T> {
    fn add_to_empty(&mut self, x: Node<T>) {
        let node = Rc::new(FooCell::new(x));
        self.head = Option::Some(node.clone());
        self.tail = Option::Some(node);
    }

    pub fn add_first(&mut self, x: T) {
        let mut node = Node::new(x);
        match &mut self.head {
            Option::None => self.add_to_empty(node),
            Option::Some(previous_head) => {
                let previous_head = previous_head.clone();
                node.next = Option::Some(previous_head.clone());
                let new_head = Rc::new(FooCell::new(node));

                let previous_head_ref = previous_head.get_mut(&mut self.owner);
                previous_head_ref.previous = Option::Some(new_head.clone());
                self.head = Option::Some(new_head);
            }
        }
    }

    pub fn add_last(&mut self, x: T) {
        let mut node = Node::new(x);
        match &mut self.tail {
            Option::None => self.add_to_empty(node),
            Option::Some(previous_tail) => {
                let previous_tail = previous_tail.clone();
                node.previous = Option::Some(previous_tail.clone());
                let new_tail = Rc::new(FooCell::new(node));

                let previous_tail_ref = previous_tail.get_mut(&mut self.owner);
                previous_tail_ref.next = Option::Some(new_tail.clone());
                self.tail = Option::Some(new_tail);
            }
        }
    }

    fn as_vec(&self) -> Vec<&T> {
        let mut elements = Vec::<&T>::new();
        let mut next = &self.head;
        while let Some(node) = next {
            elements.push(&node.get(&self.owner).data);
            next = &node.get(&self.owner).next;
        }
        elements
    }
}

impl<T: Debug> Debug for Deque<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_vec().fmt(f)
    }
}

fn two_aliases_example() {
    #[derive(Debug)]
    struct MyStruct {
        data: usize,
    }
    // cells usage
    cell_family::define!(type XFamily: XCellOwner for XCell<T>);
    let mut owner = XCellOwner::new();
    let cell = XCell::new(MyStruct { data: 123 });
    let ref1 = Rc::new(cell);
    let ref2 = ref1.clone();
    {
        // mutation through ref1
        ref1.get_mut(&mut owner).data = 35;
        assert_eq!(ref1.get(&owner).data, 35);
        assert_eq!(ref2.get(&owner).data, 35);
    }
    {
        // mutation through ref2
        ref2.get_mut(&mut owner).data = 42;
        assert_eq!(ref1.get(&owner).data, 42);
        assert_eq!(ref2.get(&owner).data, 42);
    }
}

fn deque_example() {
    let mut deque = Deque::<usize> {
        head: Option::None,
        tail: Option::None,
        owner: FooCellOwner::new(),
    };
    deque.add_first(2);
    deque.add_first(1);
    deque.add_last(3);
    println!("{:?}", deque);
}

fn main() {
    deque_example();
    two_aliases_example();
}
