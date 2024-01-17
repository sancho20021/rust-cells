use client_lib::{dynamic_owner_check, simple_usage};

mod dllist {
    use std::sync::{Arc, Weak};

    use qcell::{QCell, QCellOwner};

    pub struct Node<T> {
        pub data: T,
        next: Option<NodePtr<T>>,
        prev: Option<WeakNodePtr<T>>,
    }
    pub type NodePtr<T> = Arc<QCell<Node<T>>>;
    pub type WeakNodePtr<T> = Weak<QCell<Node<T>>>;

    impl<T> Node<T> {
        pub fn new(value: T, owner: &QCellOwner) -> NodePtr<T> {
            Arc::new(QCell::new(
                owner,
                Self {
                    data: value,
                    prev: None,
                    next: None,
                },
            ))
        }

        /// Unlink the nodes adjacent to `node`. The node will have `next` and `prev` be `None` after this.
        pub fn remove(node: &NodePtr<T>, token: &mut QCellOwner) {
            let node = node.rw(token);

            let old_prev: Option<NodePtr<T>> = node.prev.take().and_then(|p| p.upgrade());
            let old_next: Option<NodePtr<T>> = node.next.take();
            if let Some(old_next) = &old_next {
                old_next.rw(token).prev = old_prev.as_ref().map(|p| Arc::downgrade(&p));
            }
            if let Some(old_prev) = &old_prev {
                old_prev.rw(token).next = old_next;
            }
        }

        /// Insert `node2` right after `node1` in the list.
        pub fn insert_next(node1: &NodePtr<T>, node2: NodePtr<T>, token: &mut QCellOwner) {
            Self::remove(&node2, token);

            let node1_old_next: Option<NodePtr<T>> = node1.rw(token).next.take();
            if let Some(node1_old_next) = &node1_old_next {
                node1_old_next.rw(token).prev = Some(Arc::downgrade(&node2));
            }

            let node2_inner: &mut Node<T> = node2.rw(token);
            node2_inner.prev = Some(Arc::downgrade(node1));
            node2_inner.next = node1_old_next;

            node1.rw(token).next = Some(node2);
        }

        pub fn from_iter<I: IntoIterator<Item = T>>(
            token: &mut QCellOwner,
            elements: I,
        ) -> Option<NodePtr<T>> {
            let mut iter = elements.into_iter();
            let first_element = iter.next()?;
            let head = Node::new(first_element, token);
            let mut tail = Arc::clone(&head);
            while let Some(e) = iter.next() {
                let node = Node::new(e, token);
                Node::insert_next(&tail, Arc::clone(&node), token);
                tail = node;
            }
            Option::Some(head)
        }

        pub fn view_as_vec<'a>(head: Option<&'a NodePtr<T>>, token: &'a QCellOwner) -> Vec<&'a T> {
            let mut cur: Option<&NodePtr<T>> = head;
            let mut v: Vec<&'a T> = vec![];
            while let Some(node) = cur {
                v.push(&node.ro(token).data);
                cur = (&node.ro(token).next).as_ref();
            }
            v
        }
    }
}

pub mod client_lib {
    use qcell::QCellOwner;

    use super::dllist::Node;

    pub fn simple_usage() {
        let mut token = QCellOwner::new();
        let list1 = Node::from_iter(&mut token, [1, 2, 3]);
        println!("{:?}", Node::view_as_vec(list1.as_ref(), &token));
    }

    pub fn dynamic_owner_check() {
        let mut token1 = QCellOwner::new();
        let list1 = Node::from_iter(&mut token1, [1, 2, 3]);
        let token2 = QCellOwner::new();

        // panics, dynamic check of owner fails:
        // println!("{:?}", list1.map(|l| l.ro(&token2).data))
    }

    pub fn run_all_examples() {
        simple_usage();
        dynamic_owner_check();
    }
}

fn main() {
    client_lib::run_all_examples();
}
